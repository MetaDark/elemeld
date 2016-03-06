use nix;
use nix::errno::Errno;
use nix::sys::socket::{Ipv4Addr, Ipv6Addr, sockaddr_in, sockaddr_in6};

use std::{net, ptr};
use libc::{getifaddrs, freeifaddrs, AF_INET, AF_INET6};

/*
 * Obtain a list of ip addresses for each interface
 */
pub fn my_ips() -> Result<Vec<net::IpAddr>, nix::Error> {
    let mut addrs = Vec::new();

    unsafe {
        let mut list = ptr::null_mut();
        if getifaddrs(&mut list) != 0 {
            return Err(nix::Error::Sys(Errno::last()));
        }

        let mut ptr = list;
        while !ptr.is_null() {
            if !(*ptr).ifa_addr.is_null() {
                match (*(*ptr).ifa_addr).sa_family as i32 {
                    AF_INET => {
                        let sa = (*ptr).ifa_addr as *const sockaddr_in;
                        addrs.push(net::IpAddr::V4(Ipv4Addr((*sa).sin_addr).to_std()));
                    },
                    AF_INET6 => {
                        let sa = (*ptr).ifa_addr as *const sockaddr_in6;
                        addrs.push(net::IpAddr::V6(Ipv6Addr((*sa).sin6_addr).to_std()));
                    },
                    _ => (),
                }
            }
            ptr = (*ptr).ifa_next;
        }

        freeifaddrs(list);
    };

    Ok(addrs)
}
