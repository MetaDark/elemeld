use std::{ptr, mem};

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

use xfixes;
pub use x11_dl::keysym::*;

pub struct Display {
    xlib: xlib::Xlib,
    xinput2: xinput2::XInput2,
    xfixes: xfixes::XFixes,
    display: *mut xlib::Display,
    root: xlib::Window,
}

impl Display {
    pub fn open() -> Self {
        let xlib = xlib::Xlib::open().unwrap();
        let xinput2 = xinput2::XInput2::open().unwrap();
        let xfixes = xfixes::XFixes::open().unwrap();

        let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
        if display.is_null() {
            panic!("Failed to open display");
        }

        let root = unsafe { (xlib.XDefaultRootWindow)(display) };
        Display {
            xlib: xlib,
            xinput2: xinput2,
            xfixes: xfixes,
            display: display,
            root: root,
        }
    }

    // xlib interface
    pub fn connection_number(&self) -> i32 {
        unsafe { (self.xlib.XConnectionNumber)(self.display) }
    }

    pub fn flush(&self) {
        unsafe { (self.xlib.XFlush)(self.display) };
    }

    pub fn grab_pointer(&self, event_mask: i64) {
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

    pub fn grab_key(&self, keycode: i32, modifiers: u32) {
        unsafe { (self.xlib.XGrabKey)(
            self.display, keycode, modifiers, self.root, xlib::True,
            xlib::GrabModeAsync, xlib::GrabModeAsync)
        };
    }

    pub fn next_event(&self) -> Option<Event> {
        // TODO: Would XEventsQueued with QueuedAlready make more sense?
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

    pub fn query_pointer(&self) -> (i32, i32) {
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

    pub fn query_screen(&self) -> (i32, i32) {
        let screen = unsafe {
            &*(self.xlib.XDefaultScreenOfDisplay)(self.display)
        };
        (screen.width, screen.height)
    }

    pub fn select_input(&self, event_mask: i64) {
        unsafe { (self.xlib.XSelectInput)(self.display, self.root, event_mask) };
    }

    pub fn ungrab_pointer(&self) {
        unsafe { (self.xlib.XUngrabPointer)(self.display, xlib::CurrentTime) };
    }

    pub fn warp_pointer(&self, x: i32, y: i32) {
        unsafe { (self.xlib.XWarpPointer)(
            self.display, 0, self.root, 0, 0, 0, 0, x, y
        ) };
    }

    // xinput2 interface
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
