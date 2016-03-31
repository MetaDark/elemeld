use io::*;
use elemeld::Config;

use mio::*;
use mio::udp::UdpSocket;
use bincode::{serde as bincode_serde, SizeLimit};

use std::io;
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};

pub struct IpInterface<'a> {
    config: &'a Config,
    socket: udp::UdpSocket,
}

impl<'a> IpInterface<'a> {
    pub fn open(config: &'a Config) -> io::Result<Self> {
        let socket = try!(UdpSocket::v4());
        try!(socket.set_multicast_loop(false));
        try!(socket.join_multicast(&config.multicast_addr));
        try!(socket.bind(&match config.server_addr {
            IpAddr::V4(addr) => SocketAddr::V4((SocketAddrV4::new(addr, config.port))),
            IpAddr::V6(addr) => SocketAddr::V6((SocketAddrV6::new(addr, config.port, 0, 0))),
        }));

        Ok(IpInterface {
            config: config,
            socket: socket,
        })
    }
}

impl<'a> NetInterface for IpInterface<'a> {
    fn send_to(&self, event: &NetEvent, addr: &SocketAddr) -> io::Result<Option<()>> {
        let packet = bincode_serde::serialize(event, SizeLimit::Bounded(1024)).unwrap();
        debug!("=> {} <= ({} bytes) {:#?}", addr, packet.len(), event);
        match self.socket.send_to(&packet, addr) {
            Ok(Some(_)) => Ok(Some(())),
            Ok(None) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "The OS socket buffer is probably full"
            )),
            Err(err) => Err(err),
        }
    }

    fn send_to_all(&self, event: &NetEvent) -> io::Result<Option<()>> {
        let addr = match self.config.multicast_addr {
            IpAddr::V4(addr) => SocketAddr::V4((SocketAddrV4::new(addr, self.config.port))),
            IpAddr::V6(addr) => SocketAddr::V6((SocketAddrV6::new(addr, self.config.port, 0, 0))),
        };
        
        self.send_to(event, &addr)
    }

    fn recv_from(&self) -> io::Result<Option<(NetEvent, SocketAddr)>> {
        let mut buf = [0; 1024];
        self.socket.recv_from(&mut buf).map(|result| {
            result.map(|(len, addr)| {
                let event = bincode_serde::deserialize::<NetEvent>(&buf[..len]).unwrap();
                debug!("<= {} => ({} bytes) {:#?}", addr, len, event);
                (event, addr)
            })
        })
    }
}

/*
 * FIXME(Future):
 * Method delegation: https://github.com/rust-lang/rfcs/pull/1406
 */
impl<'a> Evented for IpInterface<'a> {
    fn register(&self, selector: &mut Selector, token: Token, interest: EventSet, opts: PollOpt) -> io::Result<()> {
        self.socket.register(selector, token, interest, opts)
    }

    fn reregister(&self, selector: &mut Selector, token: Token, interest: EventSet, opts: PollOpt) -> io::Result<()> {
        self.socket.reregister(selector, token, interest, opts)
    }

    fn deregister(&self, selector: &mut Selector) -> io::Result<()> {
        self.socket.deregister(selector)
    }
}
