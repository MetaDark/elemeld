pub trait HostInterface {
    fn screen_size(&self) -> (i32, i32);
    fn cursor_pos(&self) -> (i32, i32);
    fn grab_cursor(&self);
    fn ungrab_cursor(&self);
    fn grab_keyboard(&self);
    fn ungrab_keyboard(&self);
    fn recv_event(&self) -> Option<Event>;
    fn send_event(&self, event: Event);
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Button { Left, Right, Middle }

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Key {
    ControlL, ControlR,
    AltL, AltR,
    ShiftL, ShiftR,
    SuperL, SuperR,
    CapsLock,

    Space, Enter, Tab,
    Backspace, Delete,

    Num0, Num1, Num2, Num3, Num4,
    Num5, Num6, Num7, Num8, Num9,

    A, B, C, D, E, F, G, H,
    I, J, K, L, M, N, O, P,
    Q, R, S, T, U, V, W, X,
    Y, Z,

    F1, F2, F3, F4,
    F5, F6, F7, F8,
    F9, F10, F11, F12,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    Position(PositionEvent),
    Motion(MotionEvent),
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
    pub button: Button,
    pub state: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyEvent {
    pub key: u64,
    pub state: bool,
}
