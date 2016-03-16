#![allow(non_upper_case_globals)]

use io::*;

use x11_dl::keysym::*;
use x11_dl::xlib;
use x11_dl::xinput2;
use x11_dl::xtest;
use xfixes;
use mio::*;

use std::{io, ptr, mem};
use std::cell::Cell;

pub struct X11Interface {
    xlib: xlib::Xlib,
    xinput2: xinput2::XInput2,
    xtest: xtest::Xf86vmode,
    xfixes: xfixes::XFixes,
    display: *mut xlib::Display,
    root: xlib::Window,
    last_pos: Cell<(i32, i32)>,
    cursor_grabbed: Cell<bool>,
}

impl X11Interface {
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
        let host = X11Interface {
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

    fn recv_generic_event(&self, cookie: xlib::XGenericEventCookie) -> Option<HostEvent> {
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
            self.send_position_event(PositionEvent { x: x, y: y });
        }

        Some(HostEvent::Motion(MotionEvent { dx: dx, dy: dy }))
    }

    fn recv_button_event(&self, event: xlib::XButtonEvent, state: bool) -> Option<HostEvent> {
        Some(HostEvent::Button(ButtonEvent {
            button: event.button,
            state: state,
        }))
    }

    fn recv_key_event(&self, event: xlib::XKeyEvent, state: bool) -> Option<HostEvent> {
        let keysym = unsafe { (self.xlib.XKeycodeToKeysym)(self.display, event.keycode as u8, 0) };
        Some(HostEvent::Key(KeyEvent {
            key: keysym,
            state: state,
        }))
    }

    fn send_position_event(&self, event: PositionEvent) {
        unsafe {
            self.last_pos.set((event.x, event.y));
            (self.xlib.XWarpPointer)(self.display, 0, self.root, 0, 0, 0, 0, event.x, event.y);
            (self.xlib.XFlush)(self.display);
        };
    }

    fn send_motion_event(&self, event: MotionEvent) {
        unsafe {
            let (last_x, last_y) = self.last_pos.get();
            self.last_pos.set((last_x + event.dx, last_y + event.dy));
            (self.xlib.XWarpPointer)(self.display, 0, 0, 0, 0, 0, 0, event.dx, event.dy);
            (self.xlib.XFlush)(self.display);
        };
    }

    fn send_button_event(&self, event: ButtonEvent) {
        unsafe {
            (self.xtest.XTestFakeButtonEvent)(self.display, event.button, event.state as i32, xlib::CurrentTime);
            (self.xlib.XFlush)(self.display);
        };
    }

    fn send_key_event(&self, event: KeyEvent) {
        unsafe {
            let keycode = (self.xlib.XKeysymToKeycode)(self.display, event.key);
            (self.xtest.XTestFakeKeyEvent)(self.display, keycode as u32, event.state as i32, 0);
            (self.xlib.XFlush)(self.display);
        };
    }
}

impl HostInterface for X11Interface {
    fn screen_size(&self) -> (i32, i32) {
        unsafe {
            let screen = &*(self.xlib.XDefaultScreenOfDisplay)(self.display);
            assert!(screen.width > 0 && screen.height > 0);
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

    fn recv_event(&self) -> Option<HostEvent> {
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
                warn!("Unexpected X11 event: {}", event);
                None
            },
        }
    }

    fn send_event(&self, event: HostEvent) {
        match event {
            HostEvent::Position(event) => self.send_position_event(event),
            HostEvent::Motion(event) => self.send_motion_event(event),
            HostEvent::Button(event) => self.send_button_event(event),
            HostEvent::Key(event) => self.send_key_event(event),
        }
    }
}

/*
 * FIXME(Future):
 * Method delegation: https://github.com/rust-lang/rfcs/pull/1406
 */
impl Evented for X11Interface {
    fn register(&self, selector: &mut Selector, token: Token, interest: EventSet, opts: PollOpt) -> io::Result<()> {
        selector.register(self.connection_number(), token, interest, opts)
    }

    fn reregister(&self, selector: &mut Selector, token: Token, interest: EventSet, opts: PollOpt) -> io::Result<()> {
        selector.reregister(self.connection_number(), token, interest, opts)
    }

    fn deregister(&self, selector: &mut Selector) -> io::Result<()> {
        selector.deregister(self.connection_number())
    }
}

impl Drop for X11Interface {
    fn drop(&mut self) {
        unsafe { (self.xlib.XCloseDisplay)(self.display) };
    }
}
