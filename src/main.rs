#![allow(dead_code, unused_variables, unused_imports)]
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate mio;
extern crate serde;
extern crate serde_json;
extern crate x11_dl;
extern crate dylib;

mod elemeld;
mod cluster;
mod io;
mod x11;
mod ip;

// Hacky work around for Xfixes
#[macro_use] mod link;
mod xfixes;

use elemeld::Elemeld;
use x11::X11Interface;
use ip::{IpInterface, Config};

use mio::*;

const HOST_EVENT: Token = Token(0);
const NET_EVENT: Token = Token(1);

fn main() {
    let mut event_loop = EventLoop::new().unwrap();
    let mut manager = EventManager::new(&mut event_loop, Config {
        server_addr: Ipv4Addr::new(0, 0, 0, 0),
        multicast_addr: Ipv4Addr::new(239, 255, 80, 80),
        port: 8080,
    });

    event_loop.run(&mut manager).unwrap();
}

struct EventManager {
    elemeld: Elemeld<X11Interface, IpInterface>,
}

impl EventManager {
    fn new(event_loop: &mut EventLoop<Self>, config: Config) -> Self {
        // Setup host interface
        let host = X11Interface::open();
        event_loop.register(&host,
                            HOST_EVENT,
                            EventSet::readable(),
                            PollOpt::level()).unwrap();

        // Setup net interface
        let net = IpInterface::open(config);
        event_loop.register(&net,
                            NET_EVENT,
                            EventSet::readable() | EventSet::writable(),
                            PollOpt::level()).unwrap();

        EventManager { elemeld: Elemeld::new(host, net) }
    }
}

impl Handler for EventManager {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        match token {
            HOST_EVENT => self.elemeld.host_event(),
            NET_EVENT => self.elemeld.net_event(),
            _ => unreachable!(),
        }
    }
}
