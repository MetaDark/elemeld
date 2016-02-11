use event::{Key, CursorButton};

use std::{ptr, mem, str};
use std::ffi::CStr;

use x11_dl::xlib;
pub use x11_dl::xlib::{
    // Events
    XKeyPressedEvent,
    XKeyReleasedEvent,
    XButtonPressedEvent,
    XButtonReleasedEvent,
    XMotionEvent,
    XEnterWindowEvent,
    XLeaveWindowEvent,
    XFocusInEvent,
    XFocusOutEvent,
    XKeymapEvent,
    XExposeEvent,
    XGraphicsExposeEvent,
    XNoExposeEvent,
    XVisibilityEvent,
    XCreateWindowEvent,
    XDestroyWindowEvent,
    XUnmapEvent,
    XMapEvent,
    XMapRequestEvent,
    XReparentEvent,
    XConfigureEvent,
    XConfigureRequestEvent,
    XGravityEvent,
    XResizeRequestEvent,
    XCirculateEvent,
    XCirculateRequestEvent,
    XPropertyEvent,
    XSelectionClearEvent,
    XSelectionRequestEvent,
    XSelectionEvent,
    XColormapEvent,
    XClientMessageEvent,
    XMappingEvent,
    XGenericEventCookie,

    // Event masks
    NoEventMask,
    KeyPressMask,
    KeyReleaseMask,
    ButtonPressMask,
    ButtonReleaseMask,
    EnterWindowMask,
    LeaveWindowMask,
    PointerMotionMask,
    PointerMotionHintMask,
    Button1MotionMask,
    Button2MotionMask,
    Button3MotionMask,
    Button4MotionMask,
    Button5MotionMask,
    ButtonMotionMask,
    KeymapStateMask,
    ExposureMask,
    VisibilityChangeMask,
    StructureNotifyMask,
    ResizeRedirectMask,
    SubstructureNotifyMask,
    SubstructureRedirectMask,
    FocusChangeMask,
    PropertyChangeMask,
    ColormapChangeMask,
    OwnerGrabButtonMask,

    // Key types
    KeyCode,
    KeySym,

    // Keyboard modifiers
    ShiftMask,   // Left?
    LockMask,    // Left?
    ControlMask, // Left?
    Mod1Mask as AltLMask,
    Mod2Mask,    // ?
    Mod3Mask,    // ?
    Mod4Mask as SuperLMask,
    Mod5Mask,    // ?

    // Cursor buttons
    Button1,
    Button2,
    Button3,
    Button4,
    Button5,
};

use x11_dl::xinput2;
pub use x11_dl::xinput2::{
    // Macros
    XISetMask,
    XIClearMask,
    XIMaskIsSet,

    // XInput objects
    XIAllDevices,
    XIAllMasterDevices,
    XIEventMask,

    // Events
    XI_DeviceChanged,
    XI_KeyPress,
    XI_KeyRelease,
    XI_ButtonPress,
    XI_ButtonRelease,
    XI_Motion,
    XI_Enter,
    XI_Leave,
    XI_FocusIn,
    XI_FocusOut,
    XI_HierarchyChanged,
    XI_PropertyEvent,
    XI_RawKeyPress,
    XI_RawKeyRelease,
    XI_RawButtonPress,
    XI_RawButtonRelease,
    XI_RawMotion,
    XI_TouchBegin,
    XI_TouchUpdate,
    XI_TouchEnd,
    XI_TouchOwnership,
    XI_RawTouchBegin,
    XI_RawTouchUpdate,
    XI_RawTouchEnd,
    XI_BarrierHit,
    XI_BarrierLeave,
    XI_LASTEVENT,
};

use x11_dl::xtest;

use xfixes;
pub use x11_dl::keysym::*;

pub struct Display {
    xlib: xlib::Xlib,
    xinput2: xinput2::XInput2,
    xtest: xtest::Xf86vmode,
    xfixes: xfixes::XFixes,
    display: *mut xlib::Display,
    root: xlib::Window,
}

impl Display {
    pub fn open() -> Self {
        let xlib = xlib::Xlib::open().unwrap();
        let xinput2 = xinput2::XInput2::open().unwrap();
        let xtest = xtest::Xf86vmode::open().unwrap();
        let xfixes = xfixes::XFixes::open().unwrap();

        let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
        if display.is_null() {
            panic!("Failed to open display");
        }

        let root = unsafe { (xlib.XDefaultRootWindow)(display) };
        Display {
            xlib: xlib,
            xinput2: xinput2,
            xtest: xtest,
            xfixes: xfixes,
            display: display,
            root: root,
        }
    }

    // xlib interface
    pub fn connection_number(&self) -> i32 {
        unsafe { (self.xlib.XConnectionNumber)(self.display) }
    }

    pub fn grab_cursor(&self, event_mask: i64) {
        unsafe { (self.xlib.XGrabPointer)(
            self.display, self.root, xlib::True, event_mask as u32,
            xlib::GrabModeAsync, xlib::GrabModeAsync, 0, 0, xlib::CurrentTime
        ) };
    }

    pub fn grab_keyboard(&self) {
        unsafe { (self.xlib.XGrabKeyboard)(
            self.display, self.root, xlib::True,
            xlib::GrabModeAsync, xlib::GrabModeAsync, xlib::CurrentTime
        ) };
    }

    pub fn grab_key(&self, keycode: xlib::KeyCode, modifiers: u32) {
        unsafe {
            (self.xlib.XGrabKey)(
                self.display, keycode as i32, modifiers, self.root, xlib::True,
                xlib::GrabModeAsync, xlib::GrabModeAsync
            )
        };
    }

    pub fn keycode_to_keysym(&self, keycode: KeyCode,
                             index: i32) -> KeySym {
        unsafe { (self.xlib.XKeycodeToKeysym)(self.display, keycode, index) }
    }

    pub fn keysym_to_keycode(&self, keysym: KeySym) -> KeyCode {
        unsafe { (self.xlib.XKeysymToKeycode)(self.display, keysym) }
    }

    pub fn keysym_to_string(&self, keysym: KeySym) -> &str {
        unsafe { str::from_utf8_unchecked(
            CStr::from_ptr((self.xlib.XKeysymToString)(keysym)).to_bytes()
        )}
    }

    pub fn next_event(&self) -> Option<Event> {
        let num_events = unsafe { (self.xlib.XPending)(self.display) };
        if num_events <= 0 {
            return None;
        }

        let event = unsafe {
            let mut event = mem::uninitialized();
            (self.xlib.XNextEvent)(self.display, &mut event);
            event
        };

        Some(match event.get_type() {
            xlib::KeyPress => Event::KeyPress(From::from(event)),
            xlib::KeyRelease => Event::KeyRelease(From::from(event)),
            xlib::ButtonPress => Event::ButtonPress(From::from(event)),
            xlib::ButtonRelease => Event::ButtonRelease(From::from(event)),
            xlib::MotionNotify => Event::MotionNotify(From::from(event)),
            xlib::EnterNotify => Event::EnterNotify(From::from(event)),
            xlib::LeaveNotify => Event::LeaveNotify(From::from(event)),
            xlib::FocusIn => Event::FocusIn(From::from(event)),
            xlib::FocusOut => Event::FocusOut(From::from(event)),
            xlib::KeymapNotify => Event::KeymapNotify(From::from(event)),
            xlib::Expose => Event::Expose(From::from(event)),
            xlib::GraphicsExpose => Event::GraphicsExpose(From::from(event)),
            xlib::NoExpose => Event::NoExpose(From::from(event)),
            xlib::VisibilityNotify => Event::VisibilityNotify(From::from(event)),
            xlib::CreateNotify => Event::CreateNotify(From::from(event)),
            xlib::DestroyNotify => Event::DestroyNotify(From::from(event)),
            xlib::UnmapNotify => Event::UnmapNotify(From::from(event)),
            xlib::MapNotify => Event::MapNotify(From::from(event)),
            xlib::MapRequest => Event::MapRequest(From::from(event)),
            xlib::ReparentNotify => Event::ReparentNotify(From::from(event)),
            xlib::ConfigureNotify => Event::ConfigureNotify(From::from(event)),
            xlib::ConfigureRequest => Event::ConfigureRequest(From::from(event)),
            xlib::GravityNotify => Event::GravityNotify(From::from(event)),
            xlib::ResizeRequest => Event::ResizeRequest(From::from(event)),
            xlib::CirculateNotify => Event::CirculateNotify(From::from(event)),
            xlib::CirculateRequest => Event::CirculateRequest(From::from(event)),
            xlib::PropertyNotify => Event::PropertyNotify(From::from(event)),
            xlib::SelectionClear => Event::SelectionClear(From::from(event)),
            xlib::SelectionRequest => Event::SelectionRequest(From::from(event)),
            xlib::SelectionNotify => Event::SelectionNotify(From::from(event)),
            xlib::ColormapNotify => Event::ColormapNotify(From::from(event)),
            xlib::ClientMessage => Event::ClientMessage(From::from(event)),
            xlib::MappingNotify => Event::MappingNotify(From::from(event)),
            xlib::GenericEvent => Event::GenericEvent(From::from(event)),
            _ => unreachable!(),
        })
    }

    pub fn select_input(&self, event_mask: i64) {
        unsafe { (self.xlib.XSelectInput)(self.display, self.root, event_mask) };
    }

    pub fn ungrab_cursor(&self) {
        unsafe { (self.xlib.XUngrabPointer)(self.display, xlib::CurrentTime) };
    }

    pub fn ungrab_keyboard(&self) {
        unsafe { (self.xlib.XUngrabKeyboard)(self.display, xlib::CurrentTime) };
    }

    pub fn ungrab_key(&self, keycode: xlib::KeyCode, modifiers: u32) {
        unsafe { (self.xlib.XUngrabKey)(
            self.display, keycode as i32, modifiers, self.root
        ) };
    }

    // xinput2
    pub fn xi_select_events(&self, mask: &mut [XIEventMask]) {
        unsafe { (self.xinput2.XISelectEvents)(
            self.display, self.root,
            &mut mask[0] as *mut XIEventMask, mask.len() as i32
        ) };
    }

    // xfixes
    pub fn show_cursor(&self) {
        unsafe { (self.xfixes.XFixesShowCursor)(self.display, self.root) };
    }

    pub fn hide_cursor(&self) {
        unsafe { (self.xfixes.XFixesHideCursor)(self.display, self.root) };
    }

    // general interface
    pub fn screen_size(&self) -> (i32, i32) {
        let screen = unsafe {
            &*(self.xlib.XDefaultScreenOfDisplay)(self.display)
        };
        (screen.width, screen.height)
    }

    pub fn cursor_pos(&self) -> (i32, i32) {
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

    unsafe fn consume_event(&self) {
        let mut event = mem::uninitialized();
        (self.xlib.XNextEvent)(self.display, &mut event);
    }

    pub fn move_cursor(&self, x: i32, y: i32) {
        unsafe {
            (self.xlib.XWarpPointer)(self.display, 0, self.root, 0, 0, 0, 0, x, y);
            self.consume_event();
        };
    }

    pub fn set_button(&self, button: CursorButton, state: bool) {
        println!("Fake click");
        let button = match button {
            CursorButton::Left => Button1,
            CursorButton::Middle => Button3, // guessing
            CursorButton::Right => Button2, // guessing
        };

        unsafe {
            (self.xtest.XTestFakeButtonEvent)(self.display, button, state as i32, xlib::CurrentTime);
            self.consume_event();
        };
    }

    pub fn set_key(&self, key: Key, state: bool) {
        let keysym = match key {
            Key::ControlL => XK_Control_L,
            Key::ControlR => XK_Control_R,
            Key::AltL => XK_Alt_L,
            Key::AltR => XK_Alt_R,
            Key::ShiftL => XK_Shift_L,
            Key::ShiftR => XK_Shift_R,
            Key::SuperL => XK_Super_L,
            Key::SuperR => XK_Super_L,
            Key::CapsLock => XK_Caps_Lock,
            Key::Space => XK_space,
            Key::Enter => XK_Return,
            Key::Tab => XK_Tab,
            Key::Backspace => XK_BackSpace,
            Key::Delete => XK_Delete,
            Key::Num0 => XK_0,
            Key::Num1 => XK_1,
            Key::Num2 => XK_2,
            Key::Num3 => XK_3,
            Key::Num4 => XK_4,
            Key::Num5 => XK_5,
            Key::Num6 => XK_6,
            Key::Num7 => XK_7,
            Key::Num8 => XK_8,
            Key::Num9 => XK_9,
            Key::A => XK_A,
            Key::B => XK_B,
            Key::C => XK_C,
            Key::D => XK_D,
            Key::E => XK_E,
            Key::F => XK_F,
            Key::G => XK_G,
            Key::H => XK_H,
            Key::I => XK_I,
            Key::J => XK_J,
            Key::K => XK_K,
            Key::L => XK_L,
            Key::M => XK_M,
            Key::N => XK_N,
            Key::O => XK_O,
            Key::P => XK_P,
            Key::Q => XK_Q,
            Key::R => XK_R,
            Key::S => XK_S,
            Key::T => XK_T,
            Key::U => XK_U,
            Key::V => XK_V,
            Key::W => XK_W,
            Key::Y => XK_Y,
            Key::X => XK_X,
            Key::Z => XK_Z,
            Key::F1 => XK_F1,
            Key::F2 => XK_F2,
            Key::F3 => XK_F3,
            Key::F4 => XK_F4,
            Key::F5 => XK_F5,
            Key::F6 => XK_F6,
            Key::F7 => XK_F7,
            Key::F8 => XK_F8,
            Key::F9 => XK_F9,
            Key::F10 => XK_F10,
            Key::F11 => XK_F11,
            Key::F12 => XK_F12,
        };

        unsafe {
            let keycode = (self.xlib.XKeysymToKeycode)(self.display, keysym as u64);
            (self.xtest.XTestFakeKeyEvent)(self.display, keycode as u32, state as i32, 0);
            self.consume_event();
        }
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe { (self.xlib.XCloseDisplay)(self.display) };
    }
}

pub enum Event {
    KeyPress(XKeyPressedEvent),
    KeyRelease(XKeyReleasedEvent),
    ButtonPress(XButtonPressedEvent),
    ButtonRelease(XButtonReleasedEvent),
    MotionNotify(XMotionEvent),
    EnterNotify(XEnterWindowEvent),
    LeaveNotify(XLeaveWindowEvent),
    FocusIn(XFocusInEvent),
    FocusOut(XFocusOutEvent),
    KeymapNotify(XKeymapEvent),
    Expose(XExposeEvent),
    GraphicsExpose(XGraphicsExposeEvent),
    NoExpose(XNoExposeEvent),
    VisibilityNotify(XVisibilityEvent),
    CreateNotify(XCreateWindowEvent),
    DestroyNotify(XDestroyWindowEvent),
    UnmapNotify(XUnmapEvent),
    MapNotify(XMapEvent),
    MapRequest(XMapRequestEvent),
    ReparentNotify(XReparentEvent),
    ConfigureNotify(XConfigureEvent),
    ConfigureRequest(XConfigureRequestEvent),
    GravityNotify(XGravityEvent),
    ResizeRequest(XResizeRequestEvent),
    CirculateNotify(XCirculateEvent),
    CirculateRequest(XCirculateRequestEvent),
    PropertyNotify(XPropertyEvent),
    SelectionClear(XSelectionClearEvent),
    SelectionRequest(XSelectionRequestEvent),
    SelectionNotify(XSelectionEvent),
    ColormapNotify(XColormapEvent),
    ClientMessage(XClientMessageEvent),
    MappingNotify(XMappingEvent),
    GenericEvent(XGenericEventCookie),
}
