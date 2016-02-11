// pub trait InputInterface {
//     fn move_cursor(&mut self, x: i32, y: i32);
//     fn set_key(&mut self, key: Key, is_press: bool);
// }

#[derive(Serialize, Deserialize, Debug)]
pub enum Server {
    CursorMotion(CursorMotion),
    CursorClick(CursorClick),
    Keyboard(Keyboard),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CursorMotion {
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CursorClick {
    pub button: CursorButton,
    pub state: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum CursorButton { Left, Right, Middle }

#[derive(Serialize, Deserialize, Debug)]
pub struct Keyboard {
    pub key: Key,
    pub state: bool,
}

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
