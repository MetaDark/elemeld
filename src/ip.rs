use io::{self, NetInterface};
use elemeld::Config;

use mio;
use serde_json;

use std::{net, str};
use std::cell::Cell;

pub struct IpInterface<'a> {
    config: &'a Config,
    socket: mio::udp::UdpSocket,
    out_packets: Cell<usize>,
    in_packets: Cell<usize>,
}

impl<'a> IpInterface<'a> {
    pub fn open(config: &'a Config) -> Self {
        let socket = mio::udp::UdpSocket::v4().unwrap();
        socket.set_multicast_loop(false).unwrap();
        socket.join_multicast(&config.multicast_addr).unwrap();
        socket.bind(&match config.server_addr {
            mio::IpAddr::V4(addr) => net::SocketAddr::V4((net::SocketAddrV4::new(addr, config.port))),
            mio::IpAddr::V6(addr) => net::SocketAddr::V6((net::SocketAddrV6::new(addr, config.port, 0, 0))),
        }).unwrap();

        IpInterface {
            config: config,
            socket: socket,
            out_packets: Cell::new(0),
            in_packets: Cell::new(0),
        }
    }
}

impl<'a> NetInterface for IpInterface<'a> {
    fn send_to(&self, events: &[io::NetEvent], addr: &net::SocketAddr) -> Option<usize> {
        let id = self.out_packets.get();
        let msg = serde_json::to_string(&(id, events)).unwrap();
        println!("=> {} {}", addr, msg);
        match self.socket.send_to(msg.as_bytes(), &addr).unwrap() {
            Some(size) => {
                self.out_packets.set(id + 1);
                Some(size)
            },
            None => {
                println!("Failed to send: {}", msg);
                None
            },
        }
    }

    fn send_to_all(&self, events: &[io::NetEvent]) -> Option<usize> {
        let addr = match self.config.multicast_addr {
            mio::IpAddr::V4(addr) => net::SocketAddr::V4((net::SocketAddrV4::new(addr, self.config.port))),
            mio::IpAddr::V6(addr) => net::SocketAddr::V6((net::SocketAddrV6::new(addr, self.config.port, 0, 0))),
        };
        self.send_to(events, &addr)
    }

    fn recv_from(&self) -> Option<(Vec<io::NetEvent>, net::SocketAddr)> {
        let mut buf = [0; 1024];
        match self.socket.recv_from(&mut buf).unwrap() {
            Some((len, addr)) => {
                let msg = str::from_utf8(&buf[..len]).unwrap();
                println!("<= {} {}", addr, msg);
                
                let (id, events): (usize, Vec<io::NetEvent>) = serde_json::from_str(msg).unwrap();
                let expected_id = self.in_packets.get();
                if id < expected_id {
                    println!("^ out of sync packet");
                } else {
                    if id > expected_id {
                        println!("^ lost {} packets", id - expected_id)
                    }

                    self.in_packets.set(id + 1);
                }

                Some((events, addr))
            },
            None => None,
        }
    }
}

/*
 * FIXME(Future):
 * Method delegation: https://github.com/rust-lang/rfcs/pull/1406
 */
impl<'a> mio::Evented for IpInterface<'a> {
    fn register(&self, selector: &mut mio::Selector, token: mio::Token, interest: mio::EventSet, opts: mio::PollOpt) -> ::std::io::Result<()> {
        self.socket.register(selector, token, interest, opts)
    }

    fn reregister(&self, selector: &mut mio::Selector, token: mio::Token, interest: mio::EventSet, opts: mio::PollOpt) -> ::std::io::Result<()> {
        self.socket.reregister(selector, token, interest, opts)
    }

    fn deregister(&self, selector: &mut mio::Selector) -> ::std::io::Result<()> {
        self.socket.deregister(selector)
    }
}
