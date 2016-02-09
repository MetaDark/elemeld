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
    x: i32, y: i32,
    real_x: i32, real_y: i32,
    focused: bool,
}

impl Server {
    fn new(event_loop: &mut EventLoop<Self>, config: (Ipv4Addr, u16, u16)) -> Self {
        // Setup X11 display
        let display = Display::open();

        let root = display.default_root_window();
        let mut mask = [0u8; (XI_LASTEVENT as usize + 7) / 8];
        XISetMask(&mut mask, XI_RawMotion);

        let mut events = [XIEventMask {
            deviceid: XIAllMasterDevices,
            mask_len: mask.len() as i32,
            mask: &mut mask[0] as *mut u8,
        }];

        display.xi_select_events(root, &mut events);

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
        let (_, _, x, y, _, _, _) = display.query_pointer();

        Server {
            config: config,

            display: display,
            x11_socket: x11_socket,
            udp_socket: udp_socket,

            x: x, y: y,
            real_x: x, real_y: y,
            focused: true,
        }
    }

    fn update_cursor(&mut self, x: i32, y: i32) {
        let addr = SocketAddr::V4(SocketAddrV4::new(self.config.0, self.config.2));
        self.udp_socket.send_to(b"cursor move\n", &addr).unwrap();

        self.x += x - self.real_x;
        self.y += y - self.real_y;
        self.real_x = x;
        self.real_y = y;

        if self.cursor_in_screen() {
            self.focus();
        } else {
            self.unfocus();
        }
    }

    fn cursor_in_screen(&self) -> bool {
        let screen = self.display.default_screen_of_display();
        self.x > 0 && self.y > 0 && self.x < screen.width - 1 && self.y < screen.height - 1
    }

    fn unfocus(&mut self) {
        if self.focused {
            let root = self.display.default_root_window();
            self.display.hide_cursor(root);
            self.display.grab_pointer(root, true,
                                      PointerMotionMask | ButtonPressMask | ButtonReleaseMask,
                                      GrabModeAsync, GrabModeAsync, 0, 0, CurrentTime);
            self.focused = false;
        }

        self.center_cursor();
    }

    fn focus(&mut self) {
        if !self.focused {
            self.restore_cursor();
            self.display.ungrab_pointer(CurrentTime);
            self.display.show_cursor(self.display.default_root_window());
            self.focused = true;
        }
    }

    fn center_cursor(&mut self) {
        let root = self.display.default_root_window();
        let screen = self.display.default_screen_of_display();
        self.real_x = screen.width / 2;
        self.real_y = screen.height / 2;
        self.display.warp_pointer(0, root, 0, 0, 0, 0, self.real_x, self.real_y);
        self.display.next_event();
    }

    fn restore_cursor(&mut self) {
        let root = self.display.default_root_window();
        self.real_x = self.x;
        self.real_y = self.y;
        self.display.warp_pointer(0, root, 0, 0, 0, 0, self.real_x, self.real_y);
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
                // TODO: Would XEventsQueued with QueuedAlready make more sense?
                assert!(self.display.pending() != 0);

                match self.display.next_event() {
                    Event::MotionNotify(e) => {
                        self.update_cursor(e.x_root, e.y_root);
                    },
                    Event::KeyPress(e) => {
                    },
                    Event::ButtonPress(e) => {
                        let addr = SocketAddr::V4(SocketAddrV4::new(self.config.0, self.config.2));
                        self.udp_socket.send_to(b"button press\n", &addr).unwrap();
                    },
                    Event::ButtonRelease(e) => {
                        let addr = SocketAddr::V4(SocketAddrV4::new(self.config.0, self.config.2));
                        self.udp_socket.send_to(b"button release\n", &addr).unwrap();
                    },
                    Event::GenericEvent(e) => {
                        let (_, _, x, y, _, _, _) = self.display.query_pointer();
                        self.update_cursor(x, y);
                    },
                    _ => unreachable!(),
                }
            },
            NET_TOKEN => {
                let mut buf = [0u8; 255];
                match self.udp_socket.recv_from(&mut buf).unwrap() {
                    Some((len, addr)) => {
                        print!("{}: {}", addr, str::from_utf8(&buf[..len]).unwrap());
                    },
                    None => (),
                }
            },
            _ => unreachable!(),
        }
    }
}
