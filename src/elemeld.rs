use io::{self, HostInterface, NetInterface};
use cluster::Cluster;
use x11::X11Interface;
use ip::IpInterface;

use mio;
use std::net;

const HOST_EVENT: mio::Token = mio::Token(0);
const NET_EVENT: mio::Token = mio::Token(1);

pub struct Elemeld<'a> {
    config: &'a Config,
    cluster: Cluster,
    host: X11Interface,
    net: IpInterface<'a>,
    state: State,
}

pub struct Config {
    pub server_addr: mio::IpAddr,
    pub multicast_addr: mio::IpAddr,
    pub port: u16
}

#[derive(Clone, Copy, Debug)]
enum State {
    Connecting,
    Waiting,
    Connected,
}

impl<'a> Elemeld<'a> {
    pub fn new(event_loop: &mut mio::EventLoop<Self>, config: &'a Config) -> Self {
        // Setup host interface
        let host = X11Interface::open();
        event_loop.register(&host,
                            HOST_EVENT,
                            mio::EventSet::readable(),
                            mio::PollOpt::level()).unwrap();

        // Setup cluster
        let (width, height) = host.screen_size();
        let (x, y) = host.cursor_pos();
        let cluster = Cluster::new(width, height, x, y);

        // Setup net interface
        let net = IpInterface::open(config);
        event_loop.register(&net,
                            NET_EVENT,
                            mio::EventSet::readable() |
                            mio::EventSet::writable(),
                            mio::PollOpt::oneshot()).unwrap();

        Elemeld {
            config: config,
            cluster: cluster,
            host: host,
            net: net,
            state: State::Connecting,
        }
    }

    pub fn host_event(&mut self, event: io::HostEvent) {
        match self.state {
            State::Connected => match self.cluster.process_host_event(&self.host, event) {
                Some(event) => match event {
                    // Global events
                    io::NetEvent::Focus(focus) => {
                        self.net.send_to_all(&[event]);
                    },
                    // Focused events
                    event => {
                        let addr = self.cluster.focused_screen().default_route();
                        self.net.send_to(&[event], addr);
                    },
                },
                None => (),
            },
            _ => (),
        }
    }

    pub fn net_event(&mut self, event: io::NetEvent, addr: net::SocketAddr) {
        match event {
            // Initialization events
            io::NetEvent::Connect(cluster) => {
                self.cluster.merge(cluster);
                self.net.send_to(&[io::NetEvent::Cluster(self.cluster.clone())], &addr);
                self.state = State::Connected;
            },
            io::NetEvent::Cluster(cluster) => {
                self.cluster.replace(&self.host, cluster);
                self.state = State::Connected;
            },
            // Global events
            io::NetEvent::Focus(focus) => {
                self.cluster.refocus(&self.host, focus);
            },
            // Focued events
            event => match self.cluster.process_net_event(event) {
                Some(event) => { self.host.send_event(event); },
                None => (),
            },
        }
    }
}

impl<'a> mio::Handler for Elemeld<'a> {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self,
             event_loop: &mut mio::EventLoop<Self>,
             token: mio::Token, events: mio::EventSet)
    {
        match token {
            HOST_EVENT => {
                if events.is_readable() {
                    match self.host.recv_event() {
                        Some(event) => self.host_event(event),
                        None => (),
                    }
                }
            },
            NET_EVENT => {
                if events.is_readable() {
                    match self.net.recv_from() {
                        Some((events, addr)) => for event in events {
                            self.net_event(event, addr);
                        },
                        None => (),
                    }
                }

                if events.is_writable() {
                    match self.state {
                        State::Connecting => {
                            self.net.send_to_all(&[io::NetEvent::Connect(self.cluster.clone())]);
                            self.state = State::Waiting;
                            event_loop.reregister(&self.net,
                                                  NET_EVENT,
                                                  mio::EventSet::readable(),
                                                  mio::PollOpt::level()).unwrap();
                        },
                        _ => ()
                    }
                }
            },
            _ => unreachable!(),
        }
    }
}
