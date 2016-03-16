use io::*;
use elemeld::Config;

use mio::*;
use mio::udp::UdpSocket;
use serde_json;

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
    fn send_to(&self, event: NetEvent, addr: &SocketAddr) -> io::Result<Option<()>> {
        debug!("=> {} <= {:#?}", addr, event);
        let packet = serde_json::to_vec(&event).unwrap();
        match self.socket.send_to(&packet, addr) {
            Ok(Some(_)) => Ok(Some(())),
            Ok(None) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "The OS socket buffer is probably full"
            )),
            Err(err) => Err(err),
        }
    }

    fn send_to_all(&self, event: NetEvent) -> io::Result<Option<()>> {
        let addr = match self.config.multicast_addr {
            IpAddr::V4(addr) => SocketAddr::V4((SocketAddrV4::new(addr, self.config.port))),
            IpAddr::V6(addr) => SocketAddr::V6((SocketAddrV6::new(addr, self.config.port, 0, 0))),
        };
        
        self.send_to(event, &addr)
    }

    fn recv_from(&self) -> io::Result<Option<(NetEvent, SocketAddr)>> {
        let mut buf = [0; 1024];
        match self.socket.recv_from(&mut buf) {
            Ok(Some((len, addr))) => match serde_json::from_slice(&buf[..len]) {
                Ok(event) => {
                    debug!("<= {} => {:#?}", addr, event);
                    Ok(Some((event, addr)))
                },
                Err(err) => Err(io::Error::new(io::ErrorKind::InvalidData, err)),
            },
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
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
