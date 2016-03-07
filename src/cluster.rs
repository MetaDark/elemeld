use io;
use elemeld::Elemeld;
use util;

use serde;
use std::net;

pub type Index = u8;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Cluster {
    screens: Vec<Screen>,
    local_screen: Index,
    focus: Focus,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct Focus {
    screen: Index,
    pos: Dimensions,
}

impl Cluster {
    pub fn new(width: i32, height: i32, x: i32, y: i32) -> Self {
        Cluster {
            screens: vec![Screen::new(width, height)],
            local_screen: 0,
            focus: Focus {
                screen: 0,
                pos: Dimensions { x: x , y: y },
            },
        }
    }
    
    pub fn filter_host_event<H>(&mut self, host: &H) -> Option<io::NetEvent>
        where H: io::HostInterface
    {
        match host.recv_event() {
            Some(io::HostEvent::Motion(event)) =>
                if event.dx != 0 || event.dy != 0 {
                    let focus = Focus {
                        screen: self.focus.screen,
                        pos: Dimensions {
                            x: self.focus.pos.x + event.dx,
                            y: self.focus.pos.y + event.dy,
                        }
                    };
                    
                    self.refocus(host, focus);
                    Some(io::NetEvent::Focus(focus))
                } else { None },
            Some(event) =>
                if !self.locally_focused() {
                    match event {
                        io::HostEvent::Button(event) => Some(io::NetEvent::Button(event)),
                        io::HostEvent::Key(event) => Some(io::NetEvent::Key(event)),
                        _ => None,
                    }
                } else { None },
            None => None,
        }
    }

    pub fn filter_net_event(&mut self, event: io::NetEvent) -> Option<io::HostEvent> {
        if self.locally_focused() {
            match event {
                io::NetEvent::Button(event) => Some(io::HostEvent::Button(event)),
                io::NetEvent::Key(event) => Some(io::HostEvent::Key(event)),
                _ => None,
            }
        } else { None }
    }

    pub fn focused_screen(&self) -> &Screen {
        &self.screens[self.focus.screen as usize]
    }
    
    fn locally_focused(&self) -> bool {
        self.focus.screen == self.local_screen
    }

    fn reset_local_screen(&mut self) {
        for ip in util::my_ips().unwrap() {
            for (i, screen) in self.screens.iter().enumerate() {
                for addr in &screen.addrs {
                    if addr.0.ip() == ip {
                        self.local_screen = i as Index;
                        return;
                    }
                }
            }
        }

        panic!("Local IP was not found in cluster");
    }

    pub fn refocus<H>(&mut self, host: &H, focus: Focus) where
        H: io::HostInterface
    {
        let was_focused = self.locally_focused();
        self.private_refocus(host, focus, was_focused);
    }
    
    fn private_refocus<H>(&mut self, host: &H, focus: Focus, was_focused: bool) where
        H: io::HostInterface
    {
        self.focus = self.normalize_focus(focus);
        if self.locally_focused() {
            if !was_focused {
                host.ungrab_cursor();
                host.ungrab_keyboard();
            }
            
            host.send_event(io::HostEvent::Position(io::PositionEvent {
                x: self.focus.pos.x, y: self.focus.pos.y,
            }));
        } else {
            if was_focused {
                host.grab_cursor();
                host.grab_keyboard();
            }
        }
    }

    /*
     * Walk through the screens untill the x and y are contained within a screen
     */
    fn normalize_focus(&self, focus: Focus) -> Focus {
        let (screen, x, y) = (focus.screen, focus.pos.x, focus.pos.y);
        let (screen, x) = self.normalize_x(screen, x);
        let (screen, y) = self.normalize_y(screen, y);
        
        Focus {
            screen: screen,
            pos: Dimensions { x: x, y: y },
        }
    }

    fn normalize_x(&self, focus: Index, x: i32) -> (Index, i32) {
        let screen = &self.screens[focus as usize];
        if x < 20 {
            match screen.edges.left {
                Some(focus) => self.normalize_x(focus, x + self.screens[focus as usize].size.x - 40),
                None => (focus, 0),
            }
        } else if x >= screen.size.x - 20 {
            match screen.edges.right {
                Some(focus) => self.normalize_x(focus, x - screen.size.x + 40),
                None => (focus, screen.size.x - 1),
            }
        } else {
            (focus, x)
        }
    }

    fn normalize_y(&self, focus: Index, y: i32) -> (Index, i32) {
        let screen = &self.screens[focus as usize];
        if y < 20 {
            match screen.edges.top {
                Some(focus) => self.normalize_y(focus, y + self.screens[focus as usize].size.y - 40),
                None => (focus, 0),
            }
        } else if y >= screen.size.y - 20 {
            match screen.edges.bottom {
                Some(focus) => self.normalize_y(focus, y - screen.size.y + 40),
                None => (focus, screen.size.y - 1),
            }
        } else {
            (focus, y)
        }
    }

    /**
     * Add a new screen to the far right of the cluster
     */
    fn add(&mut self, mut new_screen: Screen) {
        let new_index = self.screens.len() as Index;
        let mut index = 0 as Index;
        
        loop {
            let screen = &mut self.screens[index as usize];
            index = match screen.edges.right {
                Some(index) => index,
                None => {
                    screen.edges.right = Some(new_index);
                    new_screen.edges.left = Some(index);
                    break;
                }
            }
        }
        
        self.screens.push(new_screen);
    }

    /**
     * Attempt to merge two clusters together
     */
    pub fn merge(&mut self, other: Self) {
        'outer: for other_screen in other.screens {
            for other_addr in &other_screen.addrs {
                for screen in &self.screens {
                    for addr in &screen.addrs {
                        if addr.0.ip() == other_addr.0.ip() {
                            // TODO: Merge screens
                            continue 'outer;
                        }
                    }
                }
            }

            // If new address, add new screen 
            self.add(other_screen);
        }
    }

    /**
     * Replace an existing cluster with a new cluster
     */
    pub fn replace<H>(&mut self, host: &H, mut other: Self) where
        H: io::HostInterface
    {
        other.reset_local_screen();
        
        let focus = other.focus;
        let was_focused = self.locally_focused();
        other.private_refocus(host, focus, was_focused);
        
        *self = other;
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Screen {
    size: Dimensions,
    edges: Edges,
    addrs: Vec<Addr>,
}

impl Screen {
    pub fn new(width: i32, height: i32) -> Self {
        let port = 8080; // FIXME: Get from config
        Screen {
            addrs: util::my_ips().unwrap().into_iter()
                .filter_map(|addr| match addr {
                    net::IpAddr::V4(addr) =>
                        if !addr.is_loopback() {
                            Some(net::SocketAddr::V4(net::SocketAddrV4::new(addr, port)))
                        } else { None },
                    net::IpAddr::V6(addr) =>
                        if !addr.is_loopback() {
                            Some(net::SocketAddr::V6(net::SocketAddrV6::new(addr, port, 0, 0)))
                        } else { None },
                })
                .map(|addr| Addr(addr))
                .collect::<Vec<_>>(),
            
            size: Dimensions { x: width, y: height },
            edges: Edges {
                top: None,
                right: None,
                bottom: None,
                left: None
            }
        }
    }

    pub fn default_route(&self) -> &net::SocketAddr {
        &self.addrs[0].0
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct Addr(net::SocketAddr);

impl serde::Serialize for Addr {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        serializer.visit_str(&format!("{}", self.0))
    }
}

impl serde::Deserialize for Addr {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        struct SocketAddrVisitor;
        impl serde::de::Visitor for SocketAddrVisitor {
            type Value = Addr;

            fn visit_str<E>(&mut self, val: &str) -> Result<Self::Value, E>
                where E: serde::de::Error,
            {
                match val.parse::<net::SocketAddr>() {
                    Ok(addr) => Ok(Addr(addr)),
                    Err(_) => Err(serde::de::Error::syntax("expected socket address")),
                }
            }
        }

        deserializer.visit(SocketAddrVisitor)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
struct Dimensions {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
struct Edges {
    top: Option<Index>,
    right: Option<Index>,
    bottom: Option<Index>,
    left: Option<Index>,
}
