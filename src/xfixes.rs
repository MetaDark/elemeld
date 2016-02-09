#![allow(non_snake_case)]

use x11_dl::xlib::{Display, Window};

x11_link! { XFixes, ["libXfixes.so", "libXfixes.so.3"],
    pub fn XFixesHideCursor (dpy: *mut Display, win: Window) -> (),
    pub fn XFixesShowCursor (dpy: *mut Display, win: Window) -> (),
variadic:
globals:
}
