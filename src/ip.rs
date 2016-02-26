use io::{self, NetInterface};

use mio;
use serde_json;

use std::str;
use std::net::{SocketAddr, SocketAddrV4, Ipv4Addr};
use std::cell::Cell;

pub struct IpInterface {
    config: Config,
    socket: mio::udp::UdpSocket,
    out_packets: Cell<usize>,
    in_packets: Cell<usize>,
}

pub struct Config {
    pub server_addr: Ipv4Addr,
    pub multicast_addr: Ipv4Addr,
    pub port: u16
}

impl IpInterface {
    pub fn open(config: Config) -> Self {
        let socket = mio::udp::UdpSocket::v4().unwrap();
        socket.set_multicast_loop(false).unwrap();
        socket.join_multicast(&mio::IpAddr::V4(config.multicast_addr)).unwrap();
        socket.bind(&SocketAddr::V4(
            SocketAddrV4::new(config.server_addr, config.port)
        )).unwrap();

        IpInterface {
            config: config,
            socket: socket,
            out_packets: Cell::new(0),
            in_packets: Cell::new(0),
        }
    }
}

impl NetInterface for IpInterface {
    fn send_to(&self, events: &[io::NetEvent], addr: &SocketAddr) -> Option<usize> {
        let id = self.out_packets.get();
        let msg = serde_json::to_string(&(id, events)).unwrap();
        println!("=> {}", msg);
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
        let multicast_addr = SocketAddr::V4(SocketAddrV4::new(self.config.multicast_addr, self.config.port));
        self.send_to(events, &multicast_addr)
    }

    fn recv_from(&self) -> Option<(Vec<io::NetEvent>, SocketAddr)> {
        let mut buf = [0; 1024];
        match self.socket.recv_from(&mut buf).unwrap() {
            Some((len, addr)) => {
                let msg = str::from_utf8(&buf[..len]).unwrap();
                println!("<= {}", msg);
                
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
impl mio::Evented for IpInterface {
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
