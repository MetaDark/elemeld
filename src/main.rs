#![allow(dead_code, unused_variables, unused_imports)]
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate mio;
extern crate serde;
extern crate serde_json;
extern crate x11_dl;
extern crate dylib;

mod io;
mod x11;
#[macro_use] mod link;
mod xfixes;

use io::HostInterface;
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
    host: X11Host,
    net: UdpSocket,

    // Screen
    focused: bool,
    x: i32, y: i32,
    width: i32, height: i32,

    // Debug
    out_packets: Cell<usize>,
    in_packets: Cell<usize>,
}

struct Config {
    addr: Ipv4Addr,
    port: u16,
}

impl Server {
    fn new(event_loop: &mut EventLoop<Self>, config: Config) -> Self {
        // Setup X11 host
        let host = X11Host::open();
        event_loop.register(&host,
                            X11_EVENT,
                            EventSet::readable(),
                            PollOpt::level()).unwrap();

        // Setup UDP socket
        let net = UdpSocket::v4().unwrap();
        net.set_multicast_loop(false).unwrap();
        net.join_multicast(&IpAddr::V4(config.addr)).unwrap();
        net.bind(&SocketAddr::V4(
            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), config.port)
        )).unwrap();

        // Listen for UDP connections
        event_loop.register(&net,
                            NET_EVENT,
                            EventSet::readable(),
                            PollOpt::level()).unwrap();

        // Query information for local screen
        let (x, y) = host.cursor_pos();
        let (width, height) = host.screen_size();

        Server {
            config: config,
            host: host,
            net: net,

            focused: true,
            x: x, y: y,
            width: width, height: height,

            out_packets: Cell::new(0),
            in_packets: Cell::new(0),
        }
    }

    // Host functions
    fn unfocus(&mut self) {
        if self.focused {
            self.host.grab_cursor();
            self.host.grab_keyboard();
            self.focused = false;
        }
    }

    fn focus(&mut self) {
        if !self.focused {
            self.host.send_position_event(io::PositionEvent {
                x: self.x,
                y: self.y,
            });

            self.host.ungrab_cursor();
            self.host.ungrab_keyboard();
            self.focused = true;
        }
    }

    // Net functions
    fn send_to(&self, events: &[io::Event], addr: &SocketAddr) -> Option<usize> {
        let id = self.out_packets.get();
        let msg = serde_json::to_string(&(id, events)).unwrap();

        // Debug
        println!("=> {}", msg);
        self.out_packets.set(self.out_packets.get() + 1);

        self.net.send_to(msg.as_bytes(), &addr).unwrap()
    }

    fn send_to_all(&self, events: &[io::Event]) -> Option<usize> {
        let multicast_addr = SocketAddr::V4(SocketAddrV4::new(self.config.addr, self.config.port));
        self.send_to(events, &multicast_addr)
    }

    fn recv_from(&self) -> Option<(Vec<io::Event>, SocketAddr)> {
        let mut buf = [0; 256];
        match self.net.recv_from(&mut buf).unwrap() {
            Some((len, addr)) => {
                let msg = str::from_utf8(&buf[..len]).unwrap();
                let (id, events): (usize, Vec<io::Event>) = serde_json::from_str(msg).unwrap();

                // Debug
                println!("<= {}", msg);
                let expected_id = self.in_packets.get();
                if id < expected_id {
                    println!("^ old packet");
                    None
                } else {
                    if id > expected_id {
                        println!("^ lost {} packets", id - expected_id)
                    }

                    self.in_packets.set(id + 1);
                    Some((events, addr))
                }
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
            X11_EVENT => match self.host.recv_event() {
                Some(event) => match event {
                    io::Event::Motion(event) => {
                        self.x += event.dx;
                        self.y += event.dy;

                        if self.x <= 0 {
                            // TODO: Check left
                            self.unfocus();
                        } else if self.x >= self.width - 1 {
                            // TODO: Check right
                            self.unfocus();
                        } else if self.y <= 0 {
                            // TODO: Check top
                            self.unfocus();
                        } else if self.y >= self.height - 1 {
                            // TODO: Check bottom
                            self.unfocus();
                        } else {
                            self.focus();
                        }

                        self.send_to_all(&[io::Event::Position(io::PositionEvent {
                            x: self.x,
                            y: self.y,
                        })]);
                    },
                    event => if !self.focused {
                        self.send_to_all(&[event]);
                    },
                },
                None => (),
            },
            NET_EVENT => match self.recv_from() {
                Some((events, addr)) => for event in events {
                    match event {
                        io::Event::Position(event) => {
                            self.x = event.x;
                            self.y = event.y;
                            self.host.send_position_event(event);
                        },
                        event => if self.focused {
                            self.host.send_event(event);
                        },
                    }
                },
                None => (),
            },
            _ => unreachable!(),
        }
    }
}
