#![allow(dead_code, unused_variables, unused_imports, non_upper_case_globals)]
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate mio;
extern crate serde;
extern crate serde_json;
extern crate x11_dl;
extern crate dylib;

mod event;
mod x11;
#[macro_use] mod link;
mod xfixes;

use x11::*;
use mio::*;
use mio::udp::UdpSocket;
use std::net::{SocketAddr, SocketAddrV4};
use std::str;

use std::cell::Cell;

const X11_EVENT: Token = Token(0);
const NET_EVENT: Token = Token(1);

fn main() {
    let mut event_loop = EventLoop::new().unwrap();
    let mut server = Server::new(&mut event_loop, Config {
        addr: Ipv4Addr::new(239, 255, 80, 80),
        port: 8080,
    });
    event_loop.run(&mut server).unwrap();
}

struct Server {
    config: Config,

    // I/O
    display: Display,
    x11_socket: Io, // Keep alive to prevent closing the X11 socket
    udp_socket: UdpSocket,

    // State
    focused: bool,
    x: i32, y: i32,
    real_x: i32, real_y: i32,
    width: i32, height: i32,

    // Debug
    bytes: Cell<usize>,
    packets: Cell<usize>,
}

struct Config {
    addr: Ipv4Addr,
    port: u16,
}

impl Server {
    fn new(event_loop: &mut EventLoop<Self>, config: Config) -> Self {
        // Setup X11 display
        let display = Display::open();

        let mut mask = [0u8; (XI_LASTEVENT as usize + 7) / 8];
        XISetMask(&mut mask, XI_RawMotion);

        let mut events = [XIEventMask {
            deviceid: XIAllMasterDevices,
            mask_len: mask.len() as i32,
            mask: &mut mask[0] as *mut u8,
        }];

        display.xi_select_events(&mut events);
        display.grab_key(display.keysym_to_keycode(XK_Escape as KeySym), AltLMask);

        let x11_socket = Io::from_raw_fd(display.connection_number());
        event_loop.register(&x11_socket,
                            X11_EVENT,
                            EventSet::readable(),
                            PollOpt::level()).unwrap();

        // Setup UDP socket
        let udp_socket = UdpSocket::v4().unwrap();
        udp_socket.set_multicast_loop(false).unwrap();
        udp_socket.join_multicast(&IpAddr::V4(config.addr)).unwrap();
        udp_socket.bind(&SocketAddr::V4(
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), config.port)
        )).unwrap();

        // Listen for UDP connections
        event_loop.register(&udp_socket,
                            NET_EVENT,
                            EventSet::readable(),
                            PollOpt::edge()).unwrap();

        // Query dimensions for local screen
        let (x, y) = display.cursor_pos();
        let (width, height) = display.screen_size();

        Server {
            config: config,

            display: display,
            x11_socket: x11_socket,
            udp_socket: udp_socket,

            focused: true,
            x: x, y: y,
            real_x: x, real_y: y,
            width: width, height: height,

            bytes: Cell::new(0),
            packets: Cell::new(0),
        }
    }

    // display functions
    fn update_cursor(&mut self, x: i32, y: i32) {
        // Ignore if cursor is already positioned at (x,y)
        if x == self.real_x && y == self.real_y {
            return;
        }

        self.x += x - self.real_x;
        self.y += y - self.real_y;
        self.real_x = x;
        self.real_y = y;

        self.send_to_all(&[event::Server::CursorMotion(event::CursorMotion {
            x: self.x,
            y: self.y,
        })]);

        if self.cursor_in_screen() {
            self.focus();
        } else {
            self.unfocus();
        }
    }

    fn cursor_in_screen(&self) -> bool {
        self.x > 0 && self.y > 0 && self.x < self.width - 1 && self.y < self.height - 1
    }

    fn unfocus(&mut self) {
        if self.focused {
            self.display.grab_cursor(PointerMotionMask | ButtonPressMask | ButtonReleaseMask);
            self.display.grab_keyboard();
            self.display.hide_cursor();
            self.focused = false;
        }

        self.center_cursor();
    }

    fn focus(&mut self) {
        if !self.focused {
            self.display.ungrab_cursor();
            self.display.ungrab_keyboard();
            self.restore_cursor();
            self.display.show_cursor();
            self.focused = true;
        }
    }

    fn center_cursor(&mut self) {
        self.real_x = self.width / 2;
        self.real_y = self.height / 2;
        self.display.move_cursor(self.real_x, self.real_y);
    }

    fn restore_cursor(&mut self) {
        self.real_x = self.x;
        self.real_y = self.y;
        self.display.move_cursor(self.real_x, self.real_y);
    }

    // network functions
    fn send_to(&self, events: &[event::Server], addr: &SocketAddr) -> Option<usize> {
        let msg = serde_json::to_string(&events).unwrap();

        // Debug
        self.bytes.set(self.bytes.get() + msg.len() + 28);
        self.packets.set(self.packets.get() + 1);
        println!("message: {}", msg);
        println!("bytes: {} | packets: {}", self.bytes.get(), self.packets.get());

        self.udp_socket.send_to(msg.as_bytes(), &addr).unwrap()
    }

    fn send_to_all(&self, events: &[event::Server]) -> Option<usize> {
        let multicast_addr = SocketAddr::V4(SocketAddrV4::new(self.config.addr, self.config.port));
        self.send_to(events, &multicast_addr)
    }

    fn recv_from(&self) -> Option<(Vec<event::Server>, SocketAddr)> {
        let mut buf = [0; 256];
        match self.udp_socket.recv_from(&mut buf).unwrap() {
            Some((len, addr)) => {
                let buf = &buf[..len];
                Some((match serde_json::from_slice(buf) {
                    Ok(event) => event,
                    Err(e) => {
                        println!("{:?}: {}", e, String::from_utf8_lossy(buf));
                        return None;
                    },
                }, addr))
            },
            None => None,
        }
    }
}

impl Handler for Server {
    type Timeout = ();
    type Message = ();

    #[allow(unused_variables)]
    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        match token {
            X11_EVENT => {
                match self.display.next_event() {
                    Some(Event::MotionNotify(event)) =>
                        self.update_cursor(event.x_root, event.y_root),
                    Some(Event::KeyPress(event)) => {
                        let keysym = self.display.keycode_to_keysym(event.keycode as u8, 0);
                        match keysym as u32 {
                            XK_Escape => { if event.state == AltLMask {
                                println!("Alt-Escape");
                            } },
                            keysym => {
                                let keysym = self.display.keycode_to_keysym(event.keycode as u8, 0);
                                self.send_to_all(&[event::Server::Keyboard(event::Keyboard {
                                    key: match keysym {
                                        _ => event::Key::Space,
                                    },
                                    state: true,
                                })]);
                            },
                        }
                    },
                    Some(Event::KeyRelease(event)) => {
                        let keysym = self.display.keycode_to_keysym(event.keycode as u8, 0);
                        self.send_to_all(&[event::Server::Keyboard(event::Keyboard {
                            key: match keysym {
                                _ => event::Key::Space,
                            },
                            state: false,
                        })]);
                    },
                    Some(Event::ButtonPress(event)) => {
                        self.send_to_all(&[event::Server::CursorClick(event::CursorClick {
                            button: match event.button {
                                _ => event::CursorButton::Left,
                            },
                            state: true,
                        })]);
                    },
                    Some(Event::ButtonRelease(event)) => {
                        self.send_to_all(&[event::Server::CursorClick(event::CursorClick {
                            button: match event.button {
                                _ => event::CursorButton::Left,
                            },
                            state: false,
                        })]);
                    },
                    Some(Event::GenericEvent(event)) => { if event.evtype == XI_RawMotion {
                        let (x, y) = self.display.cursor_pos();
                        self.update_cursor(x, y);
                    } },
                    _ => (),
                }
            },
            NET_EVENT => {
                match self.recv_from() {
                    Some((events, addr)) => for event in events { match event {
                        event::Server::CursorMotion(event) =>
                            self.display.move_cursor(event.x, event.y),
                        event::Server::CursorClick(event) =>
                            self.display.set_button(event.button, event.state),
                        event::Server::Keyboard(event) =>
                            self.display.set_key(event.key, event.state),
                    } },
                    None => (),
                }
            },
            _ => unreachable!(),
        }
    }
}
