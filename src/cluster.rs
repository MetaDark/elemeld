use io;
use elemeld::Elemeld;

use serde;
use std::net;

pub type Index = u8;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Cluster {
    screens: Vec<Screen>,
    focus: Index,
    pos: Dimensions,
}

impl Cluster {
    pub fn new(width: i32, height: i32, x: i32, y: i32) -> Self {
        Cluster {
            screens: vec![Screen::new(width, height)],
            focus: 0,
            pos: Dimensions { x: x , y: y },
        }
    }
    
    pub fn filter_host_event<H>(&mut self, host: &H) -> Option<io::NetEvent>
        where H: io::HostInterface
    {
        match host.recv_event() {
            Some(io::HostEvent::Motion(event)) => {
                if event.dx != 0 || event.dy != 0 {
                    let (focus, x, y) = (self.focus, self.pos.x + event.dx, self.pos.y + event.dy);
                    self.refocus(host, focus, x, y);
                    Some(io::NetEvent::Focus(self.focus, io::PositionEvent {
                        x: self.pos.x, y: self.pos.y,
                    }))
                } else {
                    None
                }
            },
            Some(event) => {
                if !self.locally_focused() {
                    match event {
                        io::HostEvent::Button(event) => Some(io::NetEvent::Button(event)),
                        io::HostEvent::Key(event) => Some(io::NetEvent::Key(event)),
                        _ => None,
                    }
                } else {
                    None
                }
            }
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
        } else {
            return None
        }
    }

    fn locally_focused(&self) -> bool {
        self.screens[self.focus as usize].is_local()
    }
    
    pub fn refocus<H>(&mut self, host: &H, focus: Index, x: i32, y: i32) where
        H: io::HostInterface
    {
        let (focus, x, y) = self.normalize_focus(focus, x, y);
        let was_focused = self.locally_focused();
        
        self.focus = focus;
        self.pos.x = x;
        self.pos.y = y;
        
        if self.locally_focused() {
            if !was_focused {
                host.send_event(io::HostEvent::Position(io::PositionEvent {
                    x: self.pos.x, y: self.pos.y,
                }));

                host.ungrab_cursor();
                host.ungrab_keyboard();
            }
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
    fn normalize_focus(&self, focus: Index, x: i32, y: i32) -> (Index, i32, i32) {
        let (focus, x) = self.normalize_x(focus, x);
        let (focus, y) = self.normalize_y(focus, y);
        (focus, x, y)
    }

    fn normalize_x(&self, focus: Index, x: i32) -> (Index, i32) {
        let screen = self.screens[focus as usize];
        if self.pos.x < 20 {
            match screen.edges.left {
                Some(focus) => return self.normalize_x(focus, x + screen.size.x - 40),
                None => (),
            }
        } else if self.pos.x >= screen.size.x - 20 {
            match screen.edges.right {
                Some(focus) => return self.normalize_x(focus, x - screen.size.x + 40),
                None => (),
            }
        }

        (focus, x)
    }

    fn normalize_y(&self, focus: Index, y: i32) -> (Index, i32) {
        let screen = self.screens[focus as usize];
        if self.pos.y < 20 {
            match screen.edges.left {
                Some(focus) => return self.normalize_y(focus, y + screen.size.y - 40),
                None => (),
            }
        } else if self.pos.y >= screen.size.y - 20 {
            match screen.edges.right {
                Some(focus) => return self.normalize_y(focus, y - screen.size.y + 40),
                None => (),
            }
        }

        (focus, y)
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
    pub fn merge(&mut self, other: &Self, src: net::SocketAddr) {
        'outer: for other_screen in &other.screens {
            // Interpret other's None addresses as the src address
            let other_addr = match other_screen.addr {
                Some(addr) => addr.0,
                None => src,
            };

            for screen in &mut self.screens {
                // Interpret self's None addresses as a null address
                // TODO: Use getifaddrs to lookup matching interface address
                let screen_addr = match screen.addr {
                    Some(addr) => addr.0,
                    None => net::SocketAddr::V4(net::SocketAddrV4::new(net::Ipv4Addr::new(0, 0, 0, 0), 0)),
                };

                // If same address, replace screen with other_screen
                if screen_addr == other_addr {
                    // TODO: Merge screen properties
                    // *screen = *other_screen;
                    continue 'outer;
                }
            }

            // If new address, add new screen 
            let mut new_screen = *other_screen;
            new_screen.addr = Some(SocketAddr(src));
            self.add(new_screen);
        }
    }

    /**
     * Replace an existing cluster with a new cluster
     */
    pub fn replace(&mut self, other: &Self) {
        *self = other.clone();
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
struct Screen {
    size: Dimensions,
    edges: Edges,
    addr: Option<SocketAddr>,
}

impl Screen {
    pub fn new(width: i32, height: i32) -> Self {
        Screen {
            addr: None, // None implies local machine
            size: Dimensions { x: width, y: height },
            edges: Edges {
                top: None,
                right: None,
                bottom: None,
                left: None
            }
        }
    }

    pub fn is_local(&self) -> bool {
        self.addr.is_none()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
struct SocketAddr(net::SocketAddr);

impl serde::Serialize for SocketAddr {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        serializer.visit_str(&format!("{}", self.0))
    }
}

impl serde::Deserialize for SocketAddr {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        struct SocketAddrVisitor;
        impl serde::de::Visitor for SocketAddrVisitor {
            type Value = SocketAddr;

            fn visit_str<E>(&mut self, val: &str) -> Result<Self::Value, E>
                where E: serde::de::Error,
            {
                match val.parse::<net::SocketAddr>() {
                    Ok(addr) => Ok(SocketAddr(addr)),
                    Err(_) => Err(serde::de::Error::syntax("expected socket address")),
                }
            }
        }

        deserializer.visit(SocketAddrVisitor)
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
struct Dimensions {
    x: i32,
    y: i32,
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
struct Edges {
    top: Option<Index>,
    right: Option<Index>,
    bottom: Option<Index>,
    left: Option<Index>,
}
