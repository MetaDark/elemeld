#![allow(dead_code, unused_variables, unused_imports)]

#![feature(custom_derive, plugin)]
#![plugin(serde_macros, docopt_macros)]

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
extern crate rustc_serialize;
extern crate x11_dl;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate docopt;

mod elemeld;
mod cluster;
mod io;
mod x11;
mod ip;
mod util;

use elemeld::{Elemeld, Config};
use mio::{IpAddr, EventLoop};
// use docopt::Docopt;
use std::net::Ipv4Addr;

docopt!(Args derive Debug, "
Usage:
  elemeld [-b <bind_addr>] [-m <multicast_addr>] [-p <port>]
  elemeld -h | --help
  elemeld --version

Options:
  -b <bind_addr>       Bind address [default: 0.0.0.0].
  -m <multicast_addr>  Multicast address [default: 239.255.80.80].
  -p <port>            Port [default: 8080].
  -h --help            Show this screen.
  -v --version         Show version.
", flag_p: u16);

fn main() {
    env_logger::init().unwrap();

    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    if args.flag_version {
        println!("elemeld 0.1.0");
        return;
    };

    let config = Config {
        server_addr: args.flag_b.parse::<IpAddr>().unwrap(),
        multicast_addr: args.flag_m.parse::<IpAddr>().unwrap(),
        port: args.flag_p,
    };

    let mut event_loop = EventLoop::new().unwrap();
    let mut elemeld = Elemeld::new(&mut event_loop, &config).unwrap();
    event_loop.run(&mut elemeld).unwrap();

}
