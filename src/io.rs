use cluster;
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
    fn send_to(&self, events: &[NetEvent], addr: &SocketAddr) -> Option<usize>;
    fn send_to_all(&self, events: &[NetEvent]) -> Option<usize>;
    fn recv_from(&self) -> Option<(Vec<NetEvent>, SocketAddr)>;
}

#[derive(Serialize, Deserialize, Debug)]
pub enum HostEvent {
    Position(PositionEvent),
    Motion(MotionEvent),
    Button(ButtonEvent),
    Key(KeyEvent),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum NetEvent {
    Connect(cluster::Cluster),
    Cluster(cluster::Cluster),
    Focus(cluster::Focus),
    Button(ButtonEvent),
    Key(KeyEvent),
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Button { Left, Right, Middle }

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
    pub button: Button,
    pub state: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyEvent {
    pub key: u64,
    pub state: bool,
}
