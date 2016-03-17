#![allow(non_upper_case_globals)]

use io::*;

use x11_dl::xlib;
use x11_dl::xinput2;
use x11_dl::xtest;
use x11_dl::xfixes;
use mio::*;

use std::{io, ptr, mem};
use std::cell::Cell;
use std::ffi::CString;

pub struct X11Interface {
    xlib: xlib::Xlib,
    xinput2: xinput2::XInput2,
    xtest: xtest::Xf86vmode,
    xfixes: xfixes::XFixes,
    xfixes_event_base: i32,

    display: *mut xlib::Display,
    root: xlib::Window,
    clipboard: xlib::Atom,

    last_pos: Cell<(i32, i32)>,
    cursor_grabbed: Cell<bool>,
}

impl X11Interface {
    pub fn open() -> Self {
        let xlib = xlib::Xlib::open().unwrap();
        let xtest = xtest::Xf86vmode::open().unwrap();
        let xinput2 = xinput2::XInput2::open().unwrap();
        let xfixes = xfixes::XFixes::open().unwrap();

        let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
        if display.is_null() {
            panic!("Failed to open display");
        }

        // Query XInput2
        unsafe {
            let mut major_opcode = mem::uninitialized();
            let mut first_event = mem::uninitialized();
            let mut first_error = mem::uninitialized();
            if (xlib.XQueryExtension)(display,
                CString::new("XInputExtension").unwrap().as_ptr(),
                &mut major_opcode,
                &mut first_event,
                &mut first_error,
            ) == xlib::False {
                panic!("Failed to query XInputExtension");
            };
        }

        // Query XFixes
        let xfixes_event_base = unsafe {
            let mut event_base: i32 = mem::uninitialized();
            let mut error_base: i32 = mem::uninitialized();
            if (xfixes.XFixesQueryExtension)(display,
                &mut event_base,
                &mut error_base,
            ) == xlib::False {
                panic!("Failed to query XFixes");
            }

            event_base
        };

        let root = unsafe { (xlib.XDefaultRootWindow)(display) };
        let clipboard = unsafe { (xlib.XInternAtom)(
            display, CString::new("CLIPBOARD").unwrap().as_ptr(), 0
        ) };

        let host = X11Interface {
            xlib: xlib,
            xtest: xtest,
            xinput2: xinput2,
            xfixes: xfixes,
            xfixes_event_base: xfixes_event_base,

            display: display,
            root: root,
            clipboard: clipboard,

            last_pos: Cell::new((0, 0)),
            cursor_grabbed: Cell::new(false),
        };

        host.init();
        host
    }

    fn init(&self) {
        // Initialize last_pos for computing cursor deltas
        self.last_pos.set(self.cursor_pos());

        // Setup selection events
        self.select_selection_input(self.root, xlib::XA_PRIMARY, xfixes::XFixesSetSelectionOwnerNotifyMask);
        self.select_selection_input(self.root, self.clipboard, xfixes::XFixesSetSelectionOwnerNotifyMask);

        // Setup raw motion events
        let mut mask = [0u8; (xinput2::XI_LASTEVENT as usize + 7) / 8];
        xinput2::XISetMask(&mut mask, xinput2::XI_RawMotion);

        let mut events = [xinput2::XIEventMask {
            deviceid: xinput2::XIAllMasterDevices,
            mask_len: mask.len() as i32,
            mask: &mut mask[0] as *mut u8,
        }];

        self.select_events(self.root, &mut events);
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

    // xfixes
    fn select_selection_input(&self, window: xlib::Window, atom: xlib::Atom, event_mask: u64) {
        unsafe { (self.xfixes.XFixesSelectSelectionInput)(
            self.display, window, atom, event_mask
        ) };
    }

    // xinput2
    fn select_events(&self, window: xlib::Window, mask: &mut [xinput2::XIEventMask]) {
        unsafe { (self.xinput2.XISelectEvents)(
            self.display, window,
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

    fn recv_selection_event(&self, event: xfixes::XFixesSelectionNotifyEvent) -> Option<HostEvent> {
        match event.subtype {
            xfixes::XFixesSetSelectionOwnerNotify => {
                if event.selection == xlib::XA_PRIMARY {
                    Some(HostEvent::Selection(Selection::Primary))
                } else if event.selection == self.clipboard {
                    Some(HostEvent::Selection(Selection::Clipboard))
                } else {
                    warn!("Unexpected selection source: {}", event.selection);
                    None
                }
            },
            subtype => {
                warn!("Unexpected XFixesSelection sub event: {}", subtype);
                None
            }
        }
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
        let event_type = event.get_type();

        // Standard events
        match event_type {
            xlib::GenericEvent => return self.recv_generic_event(From::from(event)),
            xlib::ButtonPress => return self.recv_button_event(From::from(event), true),
            xlib::ButtonRelease => return self.recv_button_event(From::from(event), false),
            xlib::KeyPress => return self.recv_key_event(From::from(event), true),
            xlib::KeyRelease => return self.recv_key_event(From::from(event), false),
            xlib::MappingNotify => return None,
            _ => (),
        };

        // XFixes selection events
        match event_type - self.xfixes_event_base {
            xfixes::XFixesSelectionNotify => return self.recv_selection_event(From::from(event)),
            _ => (),
        };

        warn!("Unexpected X11 event: {}", event_type);
        None
    }

    fn send_event(&self, event: HostEvent) {
        match event {
            HostEvent::Position(event) => self.send_position_event(event),
            HostEvent::Motion(event) => self.send_motion_event(event),
            HostEvent::Button(event) => self.send_button_event(event),
            HostEvent::Key(event) => self.send_key_event(event),
            event => warn!("Unexpected host event: {:?}", event),
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
