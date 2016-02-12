#![allow(non_upper_case_globals)]

use io::{self, HostInterface};

use x11_dl::keysym::*;
use x11_dl::xlib;
use x11_dl::xinput2;
use x11_dl::xtest;
use xfixes;
use mio;

use std::{ptr, mem, str};
use std::ffi::CString;
use std::cell::Cell;

pub struct X11Host {
    xlib: xlib::Xlib,
    xinput2: xinput2::XInput2,
    xtest: xtest::Xf86vmode,
    xfixes: xfixes::XFixes,
    display: *mut xlib::Display,
    root: xlib::Window,
    last_pos: Cell<(i32, i32)>,
    cursor_grabbed: Cell<bool>,
}

impl X11Host {
    pub fn open() -> Self {
        // Connect to display
        let xlib = xlib::Xlib::open().unwrap();

        // FIXME: Should I use XQueryExtension for these extensions?
        let xinput2 = xinput2::XInput2::open().unwrap();
        let xtest = xtest::Xf86vmode::open().unwrap();
        let xfixes = xfixes::XFixes::open().unwrap();

        let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
        if display.is_null() {
            panic!("Failed to open display");
        }

        let root = unsafe { (xlib.XDefaultRootWindow)(display) };
        let host = X11Host {
            xlib: xlib,
            xinput2: xinput2,
            xtest: xtest,
            xfixes: xfixes,
            display: display,
            root: root,
            last_pos: Cell::new((0, 0)),
            cursor_grabbed: Cell::new(false),
        };

        // Initialize last_pos for computing deltas
        host.last_pos.set(host.cursor_pos());

        // Setup default events
        let mut mask = [0u8; (xinput2::XI_LASTEVENT as usize + 7) / 8];
        xinput2::XISetMask(&mut mask, xinput2::XI_RawMotion);

        let mut events = [xinput2::XIEventMask {
            deviceid: xinput2::XIAllMasterDevices,
            mask_len: mask.len() as i32,
            mask: &mut mask[0] as *mut u8,
        }];

        host.xi_select_events(&mut events);
        host
    }

    // xlib interface
    fn connection_number(&self) -> i32 {
        unsafe { (self.xlib.XConnectionNumber)(self.display) }
    }

    fn next_event(&self) -> xlib::XEvent {
        unsafe {
            let mut event = mem::uninitialized();
            (self.xlib.XNextEvent)(self.display, &mut event);
            event
        }
    }

    // xinput2
    fn xi_select_events(&self, mask: &mut [xinput2::XIEventMask]) {
        unsafe { (self.xinput2.XISelectEvents)(
            self.display, self.root,
            &mut mask[0] as *mut xinput2::XIEventMask, mask.len() as i32
        ) };
    }

    fn recv_generic_event(&self, cookie: xlib::XGenericEventCookie) -> Option<io::Event> {
        // FIXME: Assert XInput2 extension in cookie.extension
        assert_eq!(cookie.evtype, xinput2::XI_RawMotion);

        let (x, y) = self.cursor_pos();
        let (last_x, last_y) = self.last_pos.get();
        let (dx, dy) = (x - last_x, y - last_y);
        self.last_pos.set((x, y));

        // Lock cursor to center when grabbed
        if self.cursor_grabbed.get() {
            let (width, height) = self.screen_size();
            let (x, y) = (width / 2, height / 2);
            self.send_position_event(io::PositionEvent { x: x, y: y });
        }

        Some(io::Event::Motion(io::MotionEvent { dx: dx, dy: dy }))
    }

    fn recv_button_event(&self, event: xlib::XButtonEvent, state: bool) -> Option<io::Event> {
        let button = match event.button {
            xlib::Button1 => io::Button::Left,
            xlib::Button2 => io::Button::Middle,
            xlib::Button3 => io::Button::Right,
            button => {
                println!("Unexpected button press: {}", button);
                return None
            },
        };

        Some(io::Event::Button(io::ButtonEvent {
            button: button,
            state: state,
        }))
    }

    fn recv_key_event(&self, event: xlib::XKeyEvent, state: bool) -> Option<io::Event> {
        let keysym = unsafe { (self.xlib.XKeycodeToKeysym)(self.display, event.keycode as u8, 0) };
        let key = match keysym as u32 {
            XK_Control_L => io::Key::ControlL,
            XK_Control_R => io::Key::ControlR,
            XK_Alt_L => io::Key::AltL,
            XK_Alt_R => io::Key::AltR,
            XK_Shift_L => io::Key::ShiftL,
            XK_Shift_R => io::Key::ShiftR,
            XK_Super_L => io::Key::SuperL,
            XK_Super_R => io::Key::SuperR,
            XK_Caps_Lock => io::Key::CapsLock,
            XK_space => io::Key::Space,
            XK_Return => io::Key::Enter,
            XK_Tab => io::Key::Tab,
            XK_BackSpace => io::Key::Backspace,
            XK_Delete => io::Key::Delete,
            XK_0 => io::Key::Num0,
            XK_1 => io::Key::Num1,
            XK_2 => io::Key::Num2,
            XK_3 => io::Key::Num3,
            XK_4 => io::Key::Num4,
            XK_5 => io::Key::Num5,
            XK_6 => io::Key::Num6,
            XK_7 => io::Key::Num7,
            XK_8 => io::Key::Num8,
            XK_9 => io::Key::Num9,
            XK_a => io::Key::A,
            XK_b => io::Key::B,
            XK_c => io::Key::C,
            XK_d => io::Key::D,
            XK_e => io::Key::E,
            XK_f => io::Key::F,
            XK_g => io::Key::G,
            XK_h => io::Key::H,
            XK_i => io::Key::I,
            XK_j => io::Key::J,
            XK_k => io::Key::K,
            XK_l => io::Key::L,
            XK_m => io::Key::M,
            XK_n => io::Key::N,
            XK_o => io::Key::O,
            XK_p => io::Key::P,
            XK_q => io::Key::Q,
            XK_r => io::Key::R,
            XK_s => io::Key::S,
            XK_t => io::Key::T,
            XK_u => io::Key::U,
            XK_v => io::Key::V,
            XK_w => io::Key::W,
            XK_y => io::Key::Y,
            XK_x => io::Key::X,
            XK_z => io::Key::Z,
            XK_F1 => io::Key::F1,
            XK_F2 => io::Key::F2,
            XK_F3 => io::Key::F3,
            XK_F4 => io::Key::F4,
            XK_F5 => io::Key::F5,
            XK_F6 => io::Key::F6,
            XK_F7 => io::Key::F7,
            XK_F8 => io::Key::F8,
            XK_F9 => io::Key::F9,
            XK_F10 => io::Key::F10,
            XK_F11 => io::Key::F11,
            XK_F12 => io::Key::F12,
            keysym => {
                panic!(format!("Mapping for key not yet implemented: {}", unsafe {
                    let key = (self.xlib.XKeysymToString)(keysym as u64);
                    CString::from_raw(key).to_str().unwrap()
                }))
            },
        };

        Some(io::Event::Key(io::KeyEvent {
            key: key,
            state: state,
        }))
    }

    pub fn send_position_event(&self, event: io::PositionEvent) {
        unsafe {
            self.last_pos.set((event.x, event.y));
            (self.xlib.XWarpPointer)(self.display, 0, self.root, 0, 0, 0, 0, event.x, event.y);
            (self.xlib.XFlush)(self.display);
        };
    }

    pub fn send_motion_event(&self, event: io::MotionEvent) {
        unsafe {
            let (last_x, last_y) = self.last_pos.get();
            self.last_pos.set((last_x + event.dx, last_y + event.dy));
            (self.xlib.XWarpPointer)(self.display, 0, 0, 0, 0, 0, 0, event.dx, event.dy);
            (self.xlib.XFlush)(self.display);
        };
    }

    pub fn send_button_event(&self, event: io::ButtonEvent) {
        let button = match event.button {
            io::Button::Left => xlib::Button1,
            io::Button::Middle => xlib::Button2,
            io::Button::Right => xlib::Button3,
        };

        unsafe {
            (self.xtest.XTestFakeButtonEvent)(self.display, button, event.state as i32, xlib::CurrentTime);
            (self.xlib.XFlush)(self.display);
        };
    }

    pub fn send_key_event(&self, event: io::KeyEvent) {
        let keysym = match event.key {
            io::Key::ControlL => XK_Control_L,
            io::Key::ControlR => XK_Control_R,
            io::Key::AltL => XK_Alt_L,
            io::Key::AltR => XK_Alt_R,
            io::Key::ShiftL => XK_Shift_L,
            io::Key::ShiftR => XK_Shift_R,
            io::Key::SuperR => XK_Super_R,
            io::Key::SuperL => XK_Super_L,
            io::Key::CapsLock => XK_Caps_Lock,
            io::Key::Space => XK_space,
            io::Key::Enter => XK_Return,
            io::Key::Tab => XK_Tab,
            io::Key::Backspace => XK_BackSpace,
            io::Key::Delete => XK_Delete,
            io::Key::Num0 => XK_0,
            io::Key::Num1 => XK_1,
            io::Key::Num2 => XK_2,
            io::Key::Num3 => XK_3,
            io::Key::Num4 => XK_4,
            io::Key::Num5 => XK_5,
            io::Key::Num6 => XK_6,
            io::Key::Num7 => XK_7,
            io::Key::Num8 => XK_8,
            io::Key::Num9 => XK_9,
            io::Key::A => XK_A,
            io::Key::B => XK_B,
            io::Key::C => XK_C,
            io::Key::D => XK_D,
            io::Key::E => XK_E,
            io::Key::F => XK_F,
            io::Key::G => XK_G,
            io::Key::H => XK_H,
            io::Key::I => XK_I,
            io::Key::J => XK_J,
            io::Key::K => XK_K,
            io::Key::L => XK_L,
            io::Key::M => XK_M,
            io::Key::N => XK_N,
            io::Key::O => XK_O,
            io::Key::P => XK_P,
            io::Key::Q => XK_Q,
            io::Key::R => XK_R,
            io::Key::S => XK_S,
            io::Key::T => XK_T,
            io::Key::U => XK_U,
            io::Key::V => XK_V,
            io::Key::W => XK_W,
            io::Key::Y => XK_Y,
            io::Key::X => XK_X,
            io::Key::Z => XK_Z,
            io::Key::F1 => XK_F1,
            io::Key::F2 => XK_F2,
            io::Key::F3 => XK_F3,
            io::Key::F4 => XK_F4,
            io::Key::F5 => XK_F5,
            io::Key::F6 => XK_F6,
            io::Key::F7 => XK_F7,
            io::Key::F8 => XK_F8,
            io::Key::F9 => XK_F9,
            io::Key::F10 => XK_F10,
            io::Key::F11 => XK_F11,
            io::Key::F12 => XK_F12,
        };

        unsafe {
            let keycode = (self.xlib.XKeysymToKeycode)(self.display, keysym as u64);
            (self.xtest.XTestFakeKeyEvent)(self.display, keycode as u32, event.state as i32, 0);
            (self.xlib.XFlush)(self.display);
        };
    }
}

impl HostInterface for X11Host {
    fn screen_size(&self) -> (i32, i32) {
        unsafe {
            let screen = &*(self.xlib.XDefaultScreenOfDisplay)(self.display);
            (screen.width, screen.height)
        }
    }

    fn cursor_pos(&self) -> (i32, i32) {
        unsafe {
            let mut root = mem::uninitialized();
            let mut child = mem::uninitialized();
            let mut root_x = mem::uninitialized();
            let mut root_y = mem::uninitialized();
            let mut child_x = mem::uninitialized();
            let mut child_y = mem::uninitialized();
            let mut mask = mem::uninitialized();

            (self.xlib.XQueryPointer)(self.display, self.root,
                &mut root, &mut child, &mut root_x, &mut root_y,
                &mut child_x, &mut child_y, &mut mask
            );

            (root_x, root_y)
        }
    }

    fn grab_cursor(&self) {
        if self.cursor_grabbed.get() {
            return;
        }

        unsafe {
            let mask = xlib::ButtonPressMask | xlib::ButtonReleaseMask;
            (self.xlib.XGrabPointer)(
                self.display, self.root, xlib::True, mask as u32,
                xlib::GrabModeAsync, xlib::GrabModeAsync, 0, 0, xlib::CurrentTime
            );
            (self.xfixes.XFixesHideCursor)(self.display, self.root);
        };

        self.cursor_grabbed.set(true);
    }

    fn ungrab_cursor(&self) {
        if !self.cursor_grabbed.get() {
            return;
        }

        unsafe {
            (self.xlib.XUngrabPointer)(self.display, xlib::CurrentTime);
            (self.xfixes.XFixesShowCursor)(self.display, self.root);
        }

        self.cursor_grabbed.set(false);
    }

    fn grab_keyboard(&self) {
        unsafe { (self.xlib.XGrabKeyboard)(
            self.display, self.root, xlib::True,
            xlib::GrabModeAsync, xlib::GrabModeAsync, xlib::CurrentTime
        ) };
    }

    fn ungrab_keyboard(&self) {
        unsafe { (self.xlib.XUngrabKeyboard)(self.display, xlib::CurrentTime) };
    }

    fn recv_event(&self) -> Option<io::Event> {
        let num_events = unsafe { (self.xlib.XPending)(self.display) };
        if num_events <= 0 {
            return None;
        }

        let event = self.next_event();
        match event.get_type() {
            xlib::GenericEvent => self.recv_generic_event(From::from(event)),
            xlib::ButtonPress => self.recv_button_event(From::from(event), true),
            xlib::ButtonRelease => self.recv_button_event(From::from(event), false),
            xlib::KeyPress => self.recv_key_event(From::from(event), true),
            xlib::KeyRelease => self.recv_key_event(From::from(event), false),
            xlib::MappingNotify => None,
            event => {
                println!("Unexpected X11 event: {}", event);
                None
            },
        }
    }

    fn send_event(&self, event: io::Event) {
        match event {
            io::Event::Position(event) => self.send_position_event(event),
            io::Event::Motion(event) => self.send_motion_event(event),
            io::Event::Button(event) => self.send_button_event(event),
            io::Event::Key(event) => self.send_key_event(event),
        }
    }
}

/*
 * FIXME(Future):
 * Method delegation: https://github.com/rust-lang/rfcs/pull/1406
 */
impl mio::Evented for X11Host {
    fn register(&self, selector: &mut mio::Selector, token: mio::Token, interest: mio::EventSet, opts: mio::PollOpt) -> ::std::io::Result<()> {
        selector.register(self.connection_number(), token, interest, opts)
    }

    fn reregister(&self, selector: &mut mio::Selector, token: mio::Token, interest: mio::EventSet, opts: mio::PollOpt) -> ::std::io::Result<()> {
        selector.reregister(self.connection_number(), token, interest, opts)
    }

    fn deregister(&self, selector: &mut mio::Selector) -> ::std::io::Result<()> {
        selector.deregister(self.connection_number())
    }
}

impl Drop for X11Host {
    fn drop(&mut self) {
        unsafe { (self.xlib.XCloseDisplay)(self.display) };
    }
}
