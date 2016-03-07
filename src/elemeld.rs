use io;
use cluster::Cluster;

pub struct Elemeld<H, N> where
    H: io::HostInterface,
    N: io::NetInterface,
{
    host: H,
    net: N,
    state: State,
    cluster: Cluster,
}

#[derive(Clone, Copy, Debug)]
enum State {
    Connecting,
    Waiting,
    Connected,
}

impl<H, N> Elemeld<H, N> where
    H: io::HostInterface,
    N: io::NetInterface,
{
    pub fn new(host: H, net: N) -> Self {
        let (width, height) = host.screen_size();
        let (x, y) = host.cursor_pos();

        Elemeld {
            host: host,
            net: net,
            state: State::Connecting,
            cluster: Cluster::new(width, height, x, y),
        }
    }

    pub fn host_event(&mut self) {
        match self.state {
            State::Connected => match self.cluster.filter_host_event(&self.host) {
                Some(event) => {
                    match event {
                        io::NetEvent::Focus(focus) => {
                            self.net.send_to_all(&[event]);
                        },
                        event => {
                            let addr = self.cluster.focused_screen().default_route();
                            self.net.send_to(&[event], addr);
                        }
                    }
                },
                None => (),
            },
            _ => (),
        }
    }

    pub fn net_event(&mut self) {
        match self.net.recv_from() {
            Some((events, addr)) => for event in events {
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
                    event => match self.cluster.filter_net_event(event) {
                        Some(event) => { self.host.send_event(event); },
                        None => (),
                    },
                }
            },
            None => (),
        }

        match self.state {
            State::Connecting => {
                self.net.send_to_all(&[io::NetEvent::Connect(self.cluster.clone())]);
                self.state = State::Waiting;
            },
            _ => ()
        }
    }
}
