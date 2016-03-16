use io::*;
use cluster::Cluster;
use x11::X11Interface;
use ip::IpInterface;

use mio::*;

use std::io;
use std::net::SocketAddr;

const HOST_EVENT: Token = Token(0);
const NET_EVENT: Token = Token(1);

pub struct Elemeld<'a> {
    config: &'a Config,
    cluster: Cluster,
    host: X11Interface,
    net: IpInterface<'a>,
    state: State,
}

pub struct Config {
    pub server_addr: IpAddr,
    pub multicast_addr: IpAddr,
    pub port: u16
}

#[derive(Clone, Copy, Debug)]
enum State {
    Connecting,
    Waiting,
    Connected,
}

impl<'a> Elemeld<'a> {
    pub fn new(event_loop: &mut EventLoop<Self>, config: &'a Config) -> io::Result<Self> {
        // Setup host interface
        let host = X11Interface::open();
        try!(event_loop.register(&host,
                                 HOST_EVENT,
                                 EventSet::readable(),
                                 PollOpt::level()));

        // Setup cluster
        let (width, height) = host.screen_size();
        let (x, y) = host.cursor_pos();
        let cluster = Cluster::new(width, height, x, y);

        // Setup net interface
        let net = try!(IpInterface::open(config));
        try!(event_loop.register(&net,
                                 NET_EVENT,
                                 EventSet::readable() |
                                 EventSet::writable(),
                                 PollOpt::oneshot()));

        Ok(Elemeld {
            config: config,
            cluster: cluster,
            host: host,
            net: net,
            state: State::Connecting,
        })
    }

    pub fn host_event(&mut self, event: HostEvent) {
        match self.state {
            State::Connected => match self.cluster.process_host_event(&self.host, event) {
                Some(event) => match event {
                    // Global events
                    NetEvent::Focus(focus) => {
                        match self.net.send_to_all(NetEvent::Focus(focus)) {
                            Err(e) => {
                                error!("Failed to send event to cluster: {}", e);
                                self.state = State::Waiting;
                            }
                            _ => (),
                        };
                    },
                    // Focused events
                    event => {
                        let addr = self.cluster.focused_screen().default_route();
                        match self.net.send_to(event, addr) {
                            Err(e) => error!("Failed to send event to {}: {}", addr, e),
                            _ => (),
                        };
                    },
                },
                None => (),
            },
            _ => (),
        }
    }

    pub fn net_event(&mut self, event: NetEvent, addr: &SocketAddr) {
        match event {
            // Initialization events
            NetEvent::Connect(cluster) => {
                self.cluster.merge(cluster);
                match self.net.send_to(NetEvent::Cluster(self.cluster.clone()), addr) {
                    Ok(_) => self.state = State::Connected,
                    Err(err) => error!("Failed to connect: {}", err),
                };
            },
            NetEvent::Cluster(cluster) => {
                self.cluster.replace(&self.host, cluster);
                self.state = State::Connected;
            },
            // Global events
            NetEvent::Focus(focus) => {
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

impl<'a> Handler for Elemeld<'a> {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self,
             event_loop: &mut EventLoop<Self>,
             token: Token, events: EventSet)
    {
        match token {
            HOST_EVENT => {
                if events.is_readable() {
                    // A single mio event trigger may correspond to
                    // many host events, so process all host events
                    // Be careful in host.recv_event so this doesn't infinite loop
                    loop {
                        match self.host.recv_event() {
                            Some(event) => self.host_event(event),
                            None => break,
                        }
                    }
                }
            },
            NET_EVENT => {
                if events.is_readable() {
                    match self.net.recv_from() {
                        Ok(Some((event, addr))) => self.net_event(event, &addr),
                        Ok(None) => (),
                        Err(err) => error!("Failed to receive event: {}", err),
                    }
                }

                if events.is_writable() {
                    match self.state {
                        State::Connecting => {
                            match self.net.send_to_all(NetEvent::Connect(self.cluster.clone())) {
                                Err(err) => error!("Failed to connect: {}", err),
                                _ => (),
                            };

                            self.state = State::Waiting;
                            event_loop.reregister(&self.net,
                                                  NET_EVENT,
                                                  EventSet::readable(),
                                                  PollOpt::level()).unwrap();
                        },
                        _ => ()
                    }
                }
            },
            _ => unreachable!(),
        }
    }
}
