use cluster::{Cluster, Focus};

use std::io;
use std::net::SocketAddr;

pub trait HostInterface {
    fn screen_size(&self) -> (i32, i32);
    fn cursor_pos(&self) -> (i32, i32);
    fn grab_cursor(&self);
    fn ungrab_cursor(&self);
    fn grab_keyboard(&self);
    fn ungrab_keyboard(&self);
    fn recv_event(&self) -> Option<HostEvent>;
    fn send_event(&self, event: HostEvent);
}

pub trait NetInterface {
    fn send_to(&self, event: NetEvent, addr: &SocketAddr) -> io::Result<Option<()>>;
    fn send_to_all(&self, event: NetEvent) -> io::Result<Option<()>>;
    fn recv_from(&self) -> io::Result<Option<(NetEvent, SocketAddr)>>;
}

#[derive(Serialize, Deserialize, Debug)]
pub enum HostEvent {
    Position(PositionEvent),
    Motion(MotionEvent),
    Button(ButtonEvent),
    Key(KeyEvent),
    Selection(Selection),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum NetEvent {
    Connect(Cluster),
    Cluster(Cluster),
    Focus(Focus),
    Button(ButtonEvent),
    Key(KeyEvent),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PositionEvent {
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MotionEvent {
    pub dx: i32,
    pub dy: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ButtonEvent {
    pub button: u32,
    pub state: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyEvent {
    pub key: u64,
    pub state: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum Selection {
    Primary,
    Clipboard,
}
