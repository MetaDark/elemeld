use io::*;
use cluster::Cluster;
use config_server::ConfigServer;

use mio::*;
use ws::{WebSocket, Sender as WsSender};
use serde_json;

use std::io;
use std::net::SocketAddr;
use std::thread;

const HOST_EVENT: Token = Token(0);
const NET_EVENT: Token = Token(1);

pub struct Hub<H, N> where
    H: HostInterface + Evented,
    N: NetInterface + Evented,
{
    cluster: Cluster,
    host: H,
    net: N,
    clients: Option<WsSender>,
    state: State,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum State {
    Connecting,
    Waiting,
    Connected,
}

impl<H, N> Hub<H, N> where
    H: HostInterface + Evented,
    N: NetInterface + Evented,
{
    pub fn new(host: H, net: N) -> io::Result<Self> {
        let (width, height) = host.screen_size();
        let (x, y) = host.cursor_pos();
        let cluster = Cluster::new(width, height, x, y);

        Ok(Hub {
            cluster: cluster,
            host: host,
            net: net,
            clients: None,
            state: State::Connecting,
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        let mut event_loop = try!(EventLoop::new());

        try!(event_loop.register(&self.host,
                                 HOST_EVENT,
                                 EventSet::readable(),
                                 PollOpt::level()));

        try!(event_loop.register(&self.net,
                                 NET_EVENT,
                                 EventSet::readable() |
                                 EventSet::writable(),
                                 PollOpt::oneshot()));

        let channel = event_loop.channel();
        let socket = WebSocket::new(move |out| {
            ConfigServer::new(out, channel.clone())
        }).unwrap();

        self.clients = Some(socket.broadcaster());
        thread::spawn(move || {
            socket.listen("127.0.0.1:3012").unwrap();
            warn!("Configuration server has shutdown");
        });

        try!(event_loop.run(self));
        // TODO: Should probably kill spawned threads
        Ok(())
    }

    pub fn host_event(&mut self, event: HostEvent) {
        if self.state != State::Connected { return }

        if let Some(event) = self.cluster.process_host_event(&self.host, event) {
            match event {
                // Global events
                NetEvent::Focus(_) => {
                    if let Err(err) = self.net.send_to_all(&event) {
                        error!("Failed to send event to cluster: {}", err);
                        self.state = State::Waiting;
                    }
                },
                // Focused events
                event => {
                    let addr = self.cluster.focused_screen().default_route();
                    if let Err(err) = self.net.send_to(&event, addr) {
                        error!("Failed to send event to {}: {}", addr, err);
                    }
                },
            }
        }
    }

    pub fn net_event(&mut self, event: NetEvent, addr: &SocketAddr) {
        match event {
            // Initialization events
            NetEvent::Connect(cluster) => {
                self.cluster.merge(cluster);
                self.broadcast_net_event(&NetEvent::Cluster(self.cluster.clone()));
                match self.net.send_to_all(&NetEvent::Cluster(self.cluster.clone())) {
                    Ok(_) => self.state = State::Connected,
                    Err(err) => error!("Failed to connect: {}", err),
                };
            },
            NetEvent::Cluster(cluster) => {
                self.cluster.replace(&self.host, cluster);
                self.broadcast_net_event(&NetEvent::Cluster(self.cluster.clone()));
                self.state = State::Connected;
            },
            NetEvent::RequestCluster => {
                if let Err(err) = self.net.send_to(&NetEvent::Cluster(self.cluster.clone()), addr) {
                    error!("Failed to passively connect: {}", err);
                }
            },
            NetEvent::Screens(screens) => {
                self.cluster.set_screens(screens);
            },
            // Global events
            NetEvent::Focus(focus) => {
                self.cluster.refocus(&self.host, focus);
            },
            // Focued events
            event => if let Some(event) = self.cluster.process_net_event(event) {
                self.host.send_event(event);
            },
        }
    }

    fn send_net_event(&self, event: &NetEvent, sender: &WsSender) {
        let msg = serde_json::to_string(&event).unwrap();
        sender.send(msg).unwrap();
    }

    fn broadcast_net_event(&self, event: &NetEvent) {
        match self.clients {
            Some(ref clients) => self.send_net_event(event, clients),
            None => unreachable!("Cannot broadcast without clients"),
        }
    }
}

impl<H, N> Handler for Hub<H, N> where
    H: HostInterface + Evented,
    N: NetInterface + Evented,
{
    type Timeout = ();
    type Message = (NetEvent, WsSender);

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
                    while let Some(event) = self.host.recv_event() {
                        self.host_event(event);
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
                            if let Err(err) = self.net.send_to_all(&NetEvent::Connect(self.cluster.clone())) {
                                error!("Failed to connect: {}", err);
                            }

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

    fn notify(&mut self, _: &mut EventLoop<Self>, msg: Self::Message) {
        match msg.0 {
            NetEvent::RequestCluster => {
                self.send_net_event(&NetEvent::Cluster(self.cluster.clone()), &msg.1);
            },
            NetEvent::Screens(screens) => {
                self.cluster.set_screens(screens);
                self.net.send_to_all(&NetEvent::Cluster(self.cluster.clone())).unwrap();
            },
            event => warn!("Unexpected config event: {:?}", event),
        }
    }
}
