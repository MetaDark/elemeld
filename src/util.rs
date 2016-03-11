use nix;
use nix::errno::Errno;
use nix::unistd::{gethostname};
use nix::sys::socket::{Ipv4Addr, Ipv6Addr, sockaddr_in, sockaddr_in6};

use std::{mem, ptr, net, env};
use libc::{strlen, getifaddrs, freeifaddrs, AF_INET, AF_INET6};
use std::path::PathBuf;

/// Obtain the host's name
pub fn get_host_name() -> Result<String, nix::Error> {
    let mut buf = [0; 255];
    match gethostname(&mut buf) {
        Ok(_) => {
            let len = unsafe { strlen(mem::transmute(&buf as *const u8)) };
            Ok(String::from_utf8_lossy(&buf[..len]).into_owned())
        },
        Err(err) => Err(err),
    }
}

/// Obtain all of the host's IP addresses
pub fn get_host_ips() -> Result<Vec<net::IpAddr>, nix::Error> {
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

// Obtain the directory for storing application data
pub fn user_app_dir(name: &str) -> Option<PathBuf> {
    match env::home_dir() {
        Some(base) => Some(base.join(".config").join(name)),
        None => None,
    }

    // I think this is what is needed for other
    // OSes but I can't test them right now:
    //
    // Windows: %APPDATA%\<name>
    // Mac: ~/Library/Preferences/<name> or
    //      ~/Library/Application Support/<name>
    //
}
