#![allow(dead_code, unused_variables, unused_imports)]

#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

// Used for util
extern crate libc;
extern crate nix;

// Hacky work around for Xfixes
extern crate dylib;
#[macro_use]
mod link;
mod xfixes;

extern crate mio;
extern crate serde;
extern crate serde_json;
extern crate x11_dl;

#[macro_use]
extern crate log;
extern crate env_logger;

mod elemeld;
mod cluster;
mod io;
mod x11;
mod ip;
mod util;

use elemeld::{Elemeld, Config};
use mio::{IpAddr, EventLoop};
use std::net::Ipv4Addr;

fn main() {
    env_logger::init().unwrap();

    let config = Config {
        server_addr: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        multicast_addr: IpAddr::V4(Ipv4Addr::new(239, 255, 80, 80)),
        port: 8080,
    };

    let mut event_loop = EventLoop::new().unwrap();
    let mut elemeld = Elemeld::new(&mut event_loop, &config).unwrap();
    event_loop.run(&mut elemeld).unwrap();
}
