#![allow(dead_code, unused_variables, unused_imports)]

extern crate mio;
extern crate x11_dl;
extern crate dylib;

#[macro_use] mod link;
mod xfixes;
mod x11;

use x11::*;
use mio::*;
use mio::udp::UdpSocket;
use std::net::{SocketAddr, SocketAddrV4};
use std::str;

const X11_TOKEN: Token = Token(0);
const NET_TOKEN: Token = Token(1);

fn main() {
    let config = (Ipv4Addr::new(239, 255, 80, 80), 8080, 8080);
    let mut event_loop = EventLoop::new().unwrap();
    let mut server = Server::new(&mut event_loop, config);
    event_loop.run(&mut server).unwrap();
}

struct Server {
    config: (Ipv4Addr, u16, u16),

    // I/O
    display: Display,
    x11_socket: Io, // Keep alive to prevent closing the X11 socket
    udp_socket: UdpSocket,

    // State
    focused: bool,
    x: i32, y: i32,
    real_x: i32, real_y: i32,
    width: i32, height: i32,
}

impl Server {
    fn new(event_loop: &mut EventLoop<Self>, config: (Ipv4Addr, u16, u16)) -> Self {
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

        let x11_socket = Io::from_raw_fd(display.connection_number());
        event_loop.register(&x11_socket,
                            X11_TOKEN,
                            EventSet::readable(),
                            PollOpt::level()).unwrap();

        display.flush(); // TODO: Not sure why this is necessary

        // Setup UDP socket
        let udp_socket = UdpSocket::v4().unwrap();
        udp_socket.join_multicast(&IpAddr::V4(config.0)).unwrap();
        udp_socket.bind(&SocketAddr::V4(
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), config.1)
        )).unwrap();

        // Listen for UDP connections
        event_loop.register(&udp_socket,
                            NET_TOKEN,
                            EventSet::readable(),
                            PollOpt::edge()).unwrap();

        // Query dimensions for local screen
        let (x, y) = display.query_pointer();
        let (width, height) = display.query_screen();

        Server {
            config: config,

            display: display,
            x11_socket: x11_socket,
            udp_socket: udp_socket,

            focused: true,
            x: x, y: y,
            real_x: x, real_y: y,
            width: width, height: height,
        }
    }

    fn update_cursor(&mut self, x: i32, y: i32) {
        // Ignore if cursor is already positioned at (x,y)
        if x == self.real_x && y == self.real_y {
            return;
        }

        self.x += x - self.real_x;
        self.y += y - self.real_y;
        self.real_x = x;
        self.real_y = y;

        let addr = SocketAddr::V4(SocketAddrV4::new(self.config.0, self.config.2));
        let message = format!("cursor: {},{}", self.x, self.y);
        self.udp_socket.send_to(message.as_bytes(), &addr).unwrap();

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
            self.display.hide_cursor();
            self.display.grab_pointer(PointerMotionMask | ButtonPressMask | ButtonReleaseMask);
            self.focused = false;
        }

        self.center_cursor();
    }

    fn focus(&mut self) {
        if !self.focused {
            self.restore_cursor();
            self.display.ungrab_pointer();
            self.display.show_cursor();
            self.focused = true;
        }
    }

    fn center_cursor(&mut self) {
        self.real_x = self.width / 2;
        self.real_y = self.height / 2;
        self.display.warp_pointer(self.real_x, self.real_y);
        self.display.next_event();
    }

    fn restore_cursor(&mut self) {
        self.real_x = self.x;
        self.real_y = self.y;
        self.display.warp_pointer(self.real_x, self.real_y);
        self.display.next_event();
    }
}

impl Handler for Server {
    type Timeout = ();
    type Message = ();

    #[allow(unused_variables)]
    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        match token {
            X11_TOKEN => {
                match self.display.next_event() {
                    Some(Event::MotionNotify(e)) => {
                        self.update_cursor(e.x_root, e.y_root);
                    },
                    Some(Event::KeyPress(e)) => {
                    },
                    Some(Event::ButtonPress(e)) => {
                        let addr = SocketAddr::V4(SocketAddrV4::new(self.config.0, self.config.2));
                        self.udp_socket.send_to(b"button press", &addr).unwrap();
                    },
                    Some(Event::ButtonRelease(e)) => {
                        let addr = SocketAddr::V4(SocketAddrV4::new(self.config.0, self.config.2));
                        self.udp_socket.send_to(b"button release", &addr).unwrap();
                    },
                    Some(Event::GenericEvent(e)) => {
                        let (x, y) = self.display.query_pointer();
                        self.update_cursor(x, y);
                    },
                    Some(_) => unreachable!(),
                    None => (),
                }
            },
            NET_TOKEN => {
                let mut buf = [0u8; 255];
                match self.udp_socket.recv_from(&mut buf).unwrap() {
                    Some((len, addr)) => {
                        println!("{}: {}", addr, str::from_utf8(&buf[..len]).unwrap());
                    },
                    None => (),
                }
            },
            _ => unreachable!(),
        }
    }
}
