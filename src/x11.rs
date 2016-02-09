use std::{ptr, mem};

use xfixes;

use x11_dl::xlib;
pub use x11_dl::xlib::{
    // Structs
    Cursor,
    Screen,
    Time,
    Window,
    XColor,

    // Masks
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

    // Other
    GrabModeAsync,
    CurrentTime,
};

use x11_dl::xinput2;
pub use x11_dl::xinput2::{
    // Macros
    XISetMask,
    XIClearMask,
    XIMaskIsSet,

    // Structs
    XIEventMask,

    // Devices
    XIAllMasterDevices,

    // Events
    XI_RawMotion,
    XI_LASTEVENT,

    // Masks
    XI_DeviceChangedMask,
    XI_KeyPressMask,
    XI_KeyReleaseMask,
    XI_ButtonPressMask,
    XI_ButtonReleaseMask,
    XI_MotionMask,
    XI_EnterMask,
    XI_LeaveMask,
    XI_FocusInMask,
    XI_FocusOutMask,
    XI_HierarchyChangedMask,
    XI_PropertyEventMask,
    XI_RawKeyPressMask,
    XI_RawKeyReleaseMask,
    XI_RawButtonPressMask,
    XI_RawButtonReleaseMask,
    XI_RawMotionMask,
    XI_TouchBeginMask,
    XI_TouchEndMask,
    XI_TouchOwnershipChangedMask,
    XI_TouchUpdateMask,
    XI_RawTouchBeginMask,
    XI_RawTouchEndMask,
    XI_RawTouchUpdateMask,
    XI_BarrierHitMask,
    XI_BarrierLeaveMask,
};

pub struct Display {
    xlib: xlib::Xlib,
    xinput2: xinput2::XInput2,
    xfixes: xfixes::XFixes,
    display: *mut xlib::Display,
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

        Display {
            xlib: xlib,
            xinput2: xinput2,
            xfixes: xfixes,
            display: display
        }
    }

    // xlib interface
    pub fn connection_number(&self) -> i32 {
        unsafe { (self.xlib.XConnectionNumber)(self.display) }
    }

    pub fn default_root_window(&self) -> u64 {
        unsafe { (self.xlib.XDefaultRootWindow)(self.display) }
    }

    pub fn default_screen_of_display(&self) -> &Screen {
        unsafe { &*(self.xlib.XDefaultScreenOfDisplay)(self.display) }
    }

    pub fn flush(&self) {
        unsafe { (self.xlib.XFlush)(self.display) };
    }

    pub fn grab_pointer(&self, grab_window: Window, owner_events: bool,
                        event_mask: i64, pointer_mode: i32, keyboard_mode: i32,
                        confine_to: Window, cursor: Cursor, time: Time)
    {
        unsafe { (self.xlib.XGrabPointer)(
            self.display, grab_window, owner_events as i32, event_mask as u32,
            pointer_mode, keyboard_mode, confine_to, cursor, time
        ) };
    }

    pub fn next_event(&self) -> Event {
        let mut event: xlib::XEvent = unsafe { mem::uninitialized() };
        unsafe { (self.xlib.XNextEvent)(self.display, &mut event) };

        match event.get_type() {
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
        }
    }

    pub fn pending(&self) -> i32 {
        unsafe { (self.xlib.XPending)(self.display) }
    }

    pub fn query_pointer(&self) -> (Window, Window, i32, i32, i32, i32, u32) {
        unsafe {
            let root = self.default_root_window();
            let mut root_return: Window = mem::uninitialized();
            let mut child_return: Window = mem::uninitialized();
            let mut root_x: i32 = mem::uninitialized();
            let mut root_y: i32 = mem::uninitialized();
            let mut child_x: i32 = mem::uninitialized();
            let mut child_y: i32 = mem::uninitialized();
            let mut mask: u32 = mem::uninitialized();

            (self.xlib.XQueryPointer)(self.display, root,
                &mut root_return, &mut child_return, &mut root_x, &mut root_y,
                &mut child_x, &mut child_y, &mut mask
            );

            (root_return, child_return, root_x, root_y, child_x, child_y, mask)
        }
    }

    pub fn select_input(&self, w: Window, event_mask: i64) {
        unsafe { (self.xlib.XSelectInput)(self.display, w, event_mask) };
    }

    pub fn ungrab_pointer(&self, time: Time) {
        unsafe { (self.xlib.XUngrabPointer)(self.display, time) };
    }

    pub fn warp_pointer(&self, src_w: Window, dest_w: Window,
                        src_x: i32, src_y: i32, src_width: u32, src_height: u32,
                        dest_x: i32, dest_y: i32) {
        unsafe { (self.xlib.XWarpPointer)(
            self.display, src_w, dest_w, src_x, src_y,
            src_width, src_height, dest_x, dest_y
        ) };
    }

    // xinput2 interface
    pub fn xi_select_events(&self, win: Window, mask: &mut [XIEventMask]) {
        unsafe { (self.xinput2.XISelectEvents)(self.display, win, &mut mask[0] as *mut XIEventMask, mask.len() as i32) };
    }

    // xfixes
    pub fn show_cursor(&self, w: Window) {
        unsafe { (self.xfixes.XFixesShowCursor)(self.display, w) };
    }

    pub fn hide_cursor(&self, w: Window) {
        unsafe { (self.xfixes.XFixesHideCursor)(self.display, w) };
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe { (self.xlib.XCloseDisplay)(self.display) };
    }
}

pub enum Event {
    KeyPress(xlib::XKeyPressedEvent),
    KeyRelease(xlib::XKeyReleasedEvent),
    ButtonPress(xlib::XButtonPressedEvent),
    ButtonRelease(xlib::XButtonReleasedEvent),
    MotionNotify(xlib::XMotionEvent),
    EnterNotify(xlib::XEnterWindowEvent),
    LeaveNotify(xlib::XLeaveWindowEvent),
    FocusIn(xlib::XFocusInEvent),
    FocusOut(xlib::XFocusOutEvent),
    KeymapNotify(xlib::XKeymapEvent),
    Expose(xlib::XExposeEvent),
    GraphicsExpose(xlib::XGraphicsExposeEvent),
    NoExpose(xlib::XNoExposeEvent),
    VisibilityNotify(xlib::XVisibilityEvent),
    CreateNotify(xlib::XCreateWindowEvent),
    DestroyNotify(xlib::XDestroyWindowEvent),
    UnmapNotify(xlib::XUnmapEvent),
    MapNotify(xlib::XMapEvent),
    MapRequest(xlib::XMapRequestEvent),
    ReparentNotify(xlib::XReparentEvent),
    ConfigureNotify(xlib::XConfigureEvent),
    ConfigureRequest(xlib::XConfigureRequestEvent),
    GravityNotify(xlib::XGravityEvent),
    ResizeRequest(xlib::XResizeRequestEvent),
    CirculateNotify(xlib::XCirculateEvent),
    CirculateRequest(xlib::XCirculateRequestEvent),
    PropertyNotify(xlib::XPropertyEvent),
    SelectionClear(xlib::XSelectionClearEvent),
    SelectionRequest(xlib::XSelectionRequestEvent),
    SelectionNotify(xlib::XSelectionEvent),
    ColormapNotify(xlib::XColormapEvent),
    ClientMessage(xlib::XClientMessageEvent),
    MappingNotify(xlib::XMappingEvent),
    GenericEvent(xlib::XGenericEventCookie),
}
