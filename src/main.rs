// ============================================================================
// AetherWM v2.3.1 - Infinite Spatial Canvas with Per-Monitor Tiling & Spawning
// ============================================================================

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_ulong};
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

mod comp_config;
mod compositor;
mod config;

// ============================================================================
// 1. FFI TYPES
// ============================================================================

#[allow(non_camel_case_types)]
pub(crate) type XErrorHandler =
    Option<unsafe extern "C" fn(*mut libc::c_void, *mut XErrorEvent) -> libc::c_int>;

#[allow(dead_code)]
#[repr(C)]
pub(crate) struct XErrorEvent {
    type_: libc::c_int,
    display: *mut libc::c_void,
    resourceid: libc::c_ulong,
    serial: libc::c_ulong,
    error_code: libc::c_uchar,
    request_code: libc::c_uchar,
    minor_code: libc::c_uchar,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XRRMonitorInfo {
    name: c_ulong,
    primary: c_int,
    automatic: c_int,
    noutput: c_int,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    mwidth: c_int,
    mheight: c_int,
    outputs: *mut c_ulong,
}

#[repr(C)]
struct XClassHint {
    res_name: *mut c_char,
    res_class: *mut c_char,
}

#[repr(C)]
struct XWindowChanges {
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border_width: c_int,
    sibling: libc::c_ulong,
    stack_mode: c_int,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XAnyEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    window: libc::c_ulong,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XKeyEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    window: libc::c_ulong,
    root: libc::c_ulong,
    subwindow: libc::c_ulong,
    time: libc::c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: libc::c_uint,
    keycode: libc::c_uint,
    same_screen: c_int,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XButtonEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    window: libc::c_ulong,
    root: libc::c_ulong,
    subwindow: libc::c_ulong,
    time: libc::c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: libc::c_uint,
    button: libc::c_uint,
    same_screen: c_int,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XMotionEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    window: libc::c_ulong,
    root: libc::c_ulong,
    subwindow: libc::c_ulong,
    time: libc::c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: libc::c_uint,
    is_hint: libc::c_uchar,
    same_screen: c_int,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XMapRequestEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    parent: libc::c_ulong,
    window: libc::c_ulong,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XMapEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    event: libc::c_ulong,
    window: libc::c_ulong,
    override_redirect: c_int,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XConfigureRequestEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    parent: libc::c_ulong,
    window: libc::c_ulong,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border_width: c_int,
    above: libc::c_ulong,
    detail: c_int,
    value_mask: libc::c_ulong,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XConfigureEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    event: libc::c_ulong,
    window: libc::c_ulong,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border_width: c_int,
    above: libc::c_ulong,
    override_redirect: c_int,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XUnmapEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    event: libc::c_ulong,
    window: libc::c_ulong,
    from_configure: c_int,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XDestroyWindowEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    event: libc::c_ulong,
    window: libc::c_ulong,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XPropertyEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    window: libc::c_ulong,
    atom: libc::c_ulong,
    time: libc::c_ulong,
    state: c_int,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
struct XClientMessageEvent {
    type_: c_int,
    serial: libc::c_ulong,
    send_event: c_int,
    display: *mut libc::c_void,
    window: libc::c_ulong,
    message_type: libc::c_ulong,
    format: c_int,
    data: [u64; 5],
}

#[allow(dead_code)]
#[repr(C)]
union XEvent {
    type_: c_int,
    xany: XAnyEvent,
    xkey: XKeyEvent,
    xbutton: XButtonEvent,
    xmotion: XMotionEvent,
    xmaprequest: XMapRequestEvent,
    xmap: XMapEvent,
    xconfigurerequest: XConfigureRequestEvent,
    xconfigure: XConfigureEvent,
    xunmap: XUnmapEvent,
    xdestroywindow: XDestroyWindowEvent,
    xproperty: XPropertyEvent,
    xclient: XClientMessageEvent,
    _pad: [u64; 24],
}

impl XEvent {
    fn new() -> Self {
        Self { _pad: [0; 24] }
    }
}

#[derive(Clone, Copy)]
enum Drag {
    Pan {
        sx: i32,
        sy: i32,
        off_x: i32,
        off_y: i32,
    },
    Move {
        win: c_ulong,
        sx: i32,
        sy: i32,
        ox: i32,
        oy: i32,
    },
    Resize {
        win: c_ulong,
        sx: i32,
        sy: i32,
        ow: i32,
        oh: i32,
    },
}

// ============================================================================
// 2. EXTERNAL FFI BINDINGS
// ============================================================================

#[link(name = "X11")]
#[link(name = "Xrandr")]
unsafe extern "C" {
    fn XOpenDisplay(display_name: *const c_char) -> *mut libc::c_void;
    fn XDefaultScreen(display: *mut libc::c_void) -> c_int;
    fn XDisplayWidth(display: *mut libc::c_void, screen_number: c_int) -> c_int;
    fn XDisplayHeight(display: *mut libc::c_void, screen_number: c_int) -> c_int;
    fn XRootWindow(display: *mut libc::c_void, screen_number: c_int) -> c_ulong;
    fn XSelectInput(display: *mut libc::c_void, w: c_ulong, event_mask: libc::c_long) -> c_int;
    fn XSync(display: *mut libc::c_void, discard: c_int) -> c_int;
    fn XFlush(display: *mut libc::c_void) -> c_int;
    fn XNextEvent(display: *mut libc::c_void, event_return: *mut XEvent) -> c_int;
    fn XMapWindow(display: *mut libc::c_void, w: c_ulong) -> c_int;
    fn XUnmapWindow(display: *mut libc::c_void, w: c_ulong) -> c_int;
    fn XMoveResizeWindow(
        display: *mut libc::c_void,
        w: c_ulong,
        x: c_int,
        y: c_int,
        width: libc::c_uint,
        height: libc::c_uint,
    ) -> c_int;
    fn XConfigureWindow(
        display: *mut libc::c_void,
        w: c_ulong,
        value_mask: libc::c_uint,
        values: *mut XWindowChanges,
    ) -> c_int;
    fn XSetWindowBorderWidth(display: *mut libc::c_void, w: c_ulong, width: libc::c_uint) -> c_int;
    fn XSetWindowBorder(
        display: *mut libc::c_void,
        w: c_ulong,
        border_color: libc::c_ulong,
    ) -> c_int;
    fn XSetInputFocus(
        display: *mut libc::c_void,
        focus: c_ulong,
        revert_to: c_int,
        time: libc::c_ulong,
    ) -> c_int;
    fn XRaiseWindow(display: *mut libc::c_void, w: c_ulong) -> c_int;
    fn XKillClient(display: *mut libc::c_void, w: c_ulong) -> c_int;
    fn XStringToKeysym(string: *const c_char) -> libc::c_ulong;
    fn XKeysymToKeycode(display: *mut libc::c_void, keysym: libc::c_ulong) -> libc::c_uchar;
    fn XGrabKey(
        display: *mut libc::c_void,
        keycode: c_int,
        modifiers: libc::c_uint,
        grab_window: c_ulong,
        owner_events: c_int,
        pointer_mode: c_int,
        keyboard_mode: c_int,
    ) -> c_int;
    fn XUngrabKey(
        display: *mut libc::c_void,
        keycode: c_int,
        modifiers: libc::c_uint,
        grab_window: c_ulong,
    ) -> c_int;
    fn XGrabButton(
        display: *mut libc::c_void,
        button: libc::c_uint,
        modifiers: libc::c_uint,
        grab_window: c_ulong,
        owner_events: c_int,
        event_mask: libc::c_uint,
        pointer_mode: c_int,
        keyboard_mode: c_int,
        confine_to: c_ulong,
        cursor: c_ulong,
    ) -> c_int;
    fn XGrabPointer(
        display: *mut libc::c_void,
        grab_window: c_ulong,
        owner_events: c_int,
        event_mask: libc::c_uint,
        pointer_mode: c_int,
        keyboard_mode: c_int,
        confine_to: c_ulong,
        cursor: c_ulong,
        time: libc::c_ulong,
    ) -> c_int;
    fn XUngrabPointer(display: *mut libc::c_void, time: libc::c_ulong) -> c_int;
    fn XAllowEvents(display: *mut libc::c_void, event_mode: c_int, time: libc::c_ulong) -> c_int;
    fn XQueryPointer(
        display: *mut libc::c_void,
        w: c_ulong,
        root_return: *mut c_ulong,
        child_return: *mut c_ulong,
        root_x_return: *mut c_int,
        root_y_return: *mut c_int,
        win_x_return: *mut c_int,
        win_y_return: *mut c_int,
        mask_return: *mut libc::c_uint,
    ) -> c_int;
    fn XGetClassHint(
        display: *mut libc::c_void,
        window: c_ulong,
        class_hints_return: *mut XClassHint,
    ) -> c_int;
    fn XGetWMProtocols(
        display: *mut libc::c_void,
        w: c_ulong,
        protocols: *mut *mut c_ulong,
        count: *mut c_int,
    ) -> c_int;
    fn XInternAtom(
        display: *mut libc::c_void,
        atom_name: *const c_char,
        only_if_exists: c_int,
    ) -> libc::c_ulong;
    fn XChangeProperty(
        display: *mut libc::c_void,
        w: c_ulong,
        property: libc::c_ulong,
        type_: libc::c_ulong,
        format: c_int,
        mode: c_int,
        data: *const u8,
        nelements: c_int,
    ) -> c_int;
    fn XGetWindowProperty(
        display: *mut libc::c_void,
        w: c_ulong,
        property: libc::c_ulong,
        long_offset: libc::c_long,
        long_length: libc::c_long,
        delete: c_int,
        req_type: libc::c_ulong,
        actual_type_return: *mut libc::c_ulong,
        actual_format_return: *mut c_int,
        nitems_return: *mut libc::c_ulong,
        bytes_after_return: *mut libc::c_ulong,
        prop_return: *mut *mut u8,
    ) -> c_int;
    fn XCreateSimpleWindow(
        display: *mut libc::c_void,
        parent: c_ulong,
        x: c_int,
        y: c_int,
        width: libc::c_uint,
        height: libc::c_uint,
        border_width: libc::c_uint,
        border: c_ulong,
        background: c_ulong,
    ) -> c_ulong;
    fn XCreateGC(
        display: *mut libc::c_void,
        drawable: c_ulong,
        valuemask: c_ulong,
        values: *mut libc::c_void,
    ) -> *mut libc::c_void;
    fn XSetForeground(
        display: *mut libc::c_void,
        gc: *mut libc::c_void,
        foreground: c_ulong,
    ) -> c_int;
    fn XDrawString(
        display: *mut libc::c_void,
        drawable: c_ulong,
        gc: *mut libc::c_void,
        x: c_int,
        y: c_int,
        string: *const c_char,
        length: c_int,
    ) -> c_int;
    fn XFreeGC(display: *mut libc::c_void, gc: *mut libc::c_void) -> c_int;
    fn XDestroyWindow(display: *mut libc::c_void, w: c_ulong) -> c_int;
    fn XSendEvent(
        display: *mut libc::c_void,
        window: c_ulong,
        propagate: c_int,
        event_mask: libc::c_long,
        event_send: *mut XEvent,
    ) -> c_int;
    fn XFree(data: *mut libc::c_void) -> c_int;
    pub(crate) fn XSetErrorHandler(handler: XErrorHandler) -> XErrorHandler;
    fn XRRGetMonitors(
        dpy: *mut libc::c_void,
        window: c_ulong,
        get_active: c_int,
        nmonitors: *mut c_int,
    ) -> *mut XRRMonitorInfo;
    fn XRRFreeMonitors(monitors: *mut XRRMonitorInfo);
}

#[repr(C)]
struct XRectangle {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

#[link(name = "Xext")]
unsafe extern "C" {
    fn XShapeCombineRectangles(
        display: *mut libc::c_void,
        window: c_ulong,
        dest_kind: c_int,
        x_off: c_int,
        y_off: c_int,
        rectangles: *mut XRectangle,
        n_rectangles: c_int,
        operation: c_int,
        ordering: c_int,
    );
}

// ============================================================================
// 3. CONSTANTS
// ============================================================================

const KEY_PRESS_MASK: libc::c_long = 1 << 0;
const BUTTON_PRESS_MASK: libc::c_long = 1 << 2;
const BUTTON_RELEASE_MASK: libc::c_long = 1 << 3;
const POINTER_MOTION_MASK: libc::c_long = 1 << 6;
const STRUCTURE_NOTIFY_MASK: libc::c_long = 1 << 17;
const SUBSTRUCTURE_NOTIFY_MASK: libc::c_long = 1 << 19;
const SUBSTRUCTURE_REDIRECT_MASK: libc::c_long = 1 << 20;
const FOCUS_CHANGE_MASK: libc::c_long = 1 << 21;
const PROPERTY_CHANGE_MASK: libc::c_long = 1 << 22;

const CLIENT_EVENT_MASK: libc::c_long =
    STRUCTURE_NOTIFY_MASK | PROPERTY_CHANGE_MASK | FOCUS_CHANGE_MASK;

const MAP_REQUEST: i32 = 20;
const MAP_NOTIFY: i32 = 19;
const UNMAP_NOTIFY: i32 = 18;
const DESTROY_NOTIFY: i32 = 17;
const KEY_PRESS: i32 = 2;
const BUTTON_PRESS: i32 = 4;
const MOTION_NOTIFY: i32 = 6;
const BUTTON_RELEASE: i32 = 5;
const PROPERTY_NOTIFY: i32 = 28;
const CONFIGURE_REQUEST: i32 = 23;

const LOCK_MASK: libc::c_uint = 1 << 1;
const MOD2_MASK: libc::c_uint = 1 << 4;
const SUPER_MASK: libc::c_uint = 1 << 6;
const GRAB_MODE_SYNC: libc::c_int = 0;
const GRAB_MODE_ASYNC: libc::c_int = 1;
const ASYNC_POINTER: libc::c_int = 0;
const REVERT_TO_POINTER_ROOT: libc::c_int = 1;
const REPLAY_POINTER: libc::c_int = 2;
const ANY_BUTTON: libc::c_uint = 0;
const ANY_MODIFIER: libc::c_uint = 1 << 15;

const XA_WINDOW: libc::c_ulong = 33;
const XA_ATOM: libc::c_ulong = 4;
const XA_CARDINAL: libc::c_ulong = 6;
const PROP_MODE_REPLACE: c_int = 0;

const MIN_ZOOM: f32 = 0.15;
const MAX_ZOOM: f32 = 1.0;
const SHAPE_BOUNDING: c_int = 0;
const SHAPE_SET: c_int = 0;
const UNSORTED: c_int = 0;

static OTHER_WM: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn startup_err(_: *mut libc::c_void, _: *mut XErrorEvent) -> libc::c_int {
    OTHER_WM.store(true, Ordering::SeqCst);
    0
}

unsafe extern "C" fn runtime_err(_: *mut libc::c_void, _: *mut XErrorEvent) -> libc::c_int {
    0
}

extern "C" fn reap(_: c_int) {
    unsafe {
        let saved = *libc::__errno_location();
        while libc::waitpid(-1, ptr::null_mut(), libc::WNOHANG) > 0 {}
        *libc::__errno_location() = saved;
    }
}

// ============================================================================
// 4. DATA STRUCTURES
// ============================================================================

#[derive(Clone, Copy)]
struct Monitor {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    primary: c_int,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Clone)]
struct Client {
    window: libc::c_ulong,
    pos: Point,
    floating: bool,
    mapped: bool,
    ignore_unmaps: u32,
    border_color: u64,
    border_width: u32,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    mux: i32,
    muy: i32,
    muw: i32,
    muh: i32,
}

#[derive(Copy, Clone)]
struct Struts {
    top: i32,
    bottom: i32,
    left: i32,
    right: i32,
}

struct Atoms {
    net_supporting: libc::c_ulong,
    net_wm_name: libc::c_ulong,
    utf8_string: libc::c_ulong,
    net_supported: libc::c_ulong,
    net_client_list: libc::c_ulong,
    net_active_window: libc::c_ulong,
    net_wm_window_type: libc::c_ulong,
    net_wm_window_type_dock: libc::c_ulong,
    net_wm_strut_partial: libc::c_ulong,
    wm_protocols: libc::c_ulong,
    wm_delete_window: libc::c_ulong,
}

struct KeyBinding {
    keycode: u32,
    modifiers: libc::c_uint,
    action: String,
}

// ============================================================================
// 5. HELPERS
// ============================================================================

unsafe fn get_monitors(
    display: *mut libc::c_void,
    root: libc::c_ulong,
    screen: c_int,
) -> Vec<Monitor> {
    let mut nmonitors = 0;
    let monitors_ptr = unsafe { XRRGetMonitors(display, root, 1, &mut nmonitors) };
    let mut monitors = Vec::new();

    if !monitors_ptr.is_null() {
        if nmonitors > 0 {
            for i in 0..nmonitors as usize {
                let m = unsafe { *monitors_ptr.add(i) };
                if m.width > 0 && m.height > 0 {
                    monitors.push(Monitor {
                        x: m.x,
                        y: m.y,
                        width: m.width,
                        height: m.height,
                        primary: m.primary,
                    });
                }
            }
        }
        unsafe {
            XRRFreeMonitors(monitors_ptr);
        }
    }

    if monitors.is_empty() {
        let w = unsafe { XDisplayWidth(display, screen) };
        let h = unsafe { XDisplayHeight(display, screen) };
        monitors.push(Monitor {
            x: 0,
            y: 0,
            width: w,
            height: h,
            primary: 1,
        });
    }
    monitors
}

unsafe fn primary_usable_area(
    display: *mut libc::c_void,
    root: libc::c_ulong,
    screen: c_int,
    s: &Struts,
) -> (i32, i32, i32, i32) {
    let monitors = unsafe { get_monitors(display, root, screen) };
    let mon = monitors
        .iter()
        .find(|m| m.primary != 0)
        .or_else(|| monitors.first())
        .copied()
        .unwrap_or(Monitor {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            primary: 1,
        });
    (
        mon.x + s.left,
        mon.y + s.top,
        (mon.width - s.left - s.right).max(1),
        (mon.height - s.top - s.bottom).max(1),
    )
}

/// Returns the usable area of the monitor where the mouse cursor currently resides.
unsafe fn monitor_usable_area_under_cursor(
    display: *mut libc::c_void,
    root: libc::c_ulong,
    screen: c_int,
    s: &Struts,
) -> (i32, i32, i32, i32) {
    let monitors = unsafe { get_monitors(display, root, screen) };
    let mut root_ret = 0;
    let mut child_ret = 0;
    let mut rx = 0;
    let mut ry = 0;
    let mut wx = 0;
    let mut wy = 0;
    let mut mask = 0;

    let cursor_on_monitor = unsafe {
        XQueryPointer(
            display,
            root,
            &mut root_ret,
            &mut child_ret,
            &mut rx,
            &mut ry,
            &mut wx,
            &mut wy,
            &mut mask,
        ) != 0
    };

    let mon = if cursor_on_monitor {
        monitors
            .iter()
            .find(|m| rx >= m.x && rx < m.x + m.width && ry >= m.y && ry < m.y + m.height)
            .or_else(|| monitors.iter().find(|m| m.primary != 0))
            .or_else(|| monitors.first())
            .copied()
    } else {
        None
    };

    let m = mon.unwrap_or(Monitor {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        primary: 1,
    });
    (
        m.x + s.left,
        m.y + s.top,
        (m.width - s.left - s.right).max(1),
        (m.height - s.top - s.bottom).max(1),
    )
}

fn zoom_rect(x: i32, y: i32, w: u32, h: u32, z: f32, cx: i32, cy: i32) -> (i32, i32, u32, u32) {
    let sx = cx + ((x - cx) as f32 * z) as i32;
    let sy = cy + ((y - cy) as f32 * z) as i32;
    (
        sx,
        sy,
        ((w as f32 * z) as u32).max(40),
        ((h as f32 * z) as u32).max(30),
    )
}

fn animate_zoom(
    from: f32,
    to: f32,
    enabled: bool,
    duration_ms: u32,
    mut draw_frame: impl FnMut(f32),
) {
    if !enabled || duration_ms == 0 {
        draw_frame(to);
        return;
    }
    let frames = duration_ms.div_ceil(12).clamp(1, 30);
    let frame_time = (duration_ms / frames).max(1);
    for frame in 1..=frames {
        let t = frame as f32 / frames as f32;
        let eased = t * t * (3.0 - 2.0 * t);
        draw_frame(from + (to - from) * eased);
        std::thread::sleep(std::time::Duration::from_millis(frame_time as u64));
    }
}

unsafe fn read_struts(display: *mut libc::c_void, win: c_ulong, atom: c_ulong) -> Option<Struts> {
    unsafe {
        let (mut t, mut f, mut n, mut b) = (0, 0, 0, 0);
        let mut data: *mut u8 = ptr::null_mut();
        if XGetWindowProperty(
            display,
            win,
            atom,
            0,
            12,
            0,
            XA_CARDINAL,
            &mut t,
            &mut f,
            &mut n,
            &mut b,
            &mut data,
        ) != 0
            || data.is_null()
        {
            return None;
        }
        let s = if n >= 4 {
            let v = std::slice::from_raw_parts(data as *const c_ulong, n as usize);
            Some(Struts {
                left: v[0] as i32,
                right: v[1] as i32,
                top: v[2] as i32,
                bottom: v[3] as i32,
            })
        } else {
            None
        };
        XFree(data as *mut libc::c_void);
        s
    }
}

unsafe fn supports_delete(display: *mut libc::c_void, win: c_ulong, delete_atom: c_ulong) -> bool {
    unsafe {
        let mut list: *mut c_ulong = ptr::null_mut();
        let mut n = 0;
        if XGetWMProtocols(display, win, &mut list, &mut n) == 0 || list.is_null() {
            return false;
        }
        let found = std::slice::from_raw_parts(list, n as usize).contains(&delete_atom);
        XFree(list as *mut libc::c_void);
        found
    }
}

unsafe fn show_config_warning(
    display: *mut libc::c_void,
    root: c_ulong,
    screen: c_int,
    warnings: &[String],
) -> c_ulong {
    if warnings.is_empty() {
        return 0;
    }
    let first = warnings[0].chars().take(120).collect::<String>();
    let message = if warnings.len() == 1 {
        first
    } else {
        format!("{} (and {} more)", first, warnings.len() - 1)
    };
    let text = std::ffi::CString::new(message.replace('\0', " ")).unwrap_or_default();
    let width = (unsafe { XDisplayWidth(display, screen) } - 24).max(360) as libc::c_uint;
    let window =
        unsafe { XCreateSimpleWindow(display, root, 12, 12, width, 34, 1, 0x8b3030, 0x241a1a) };
    let gc = unsafe { XCreateGC(display, window, 0, ptr::null_mut()) };
    if !gc.is_null() {
        unsafe {
            XSetForeground(display, gc, 0xffffff);
            XDrawString(
                display,
                window,
                gc,
                12,
                22,
                text.as_ptr(),
                text.as_bytes().len() as c_int,
            );
            XFreeGC(display, gc);
        }
    }
    unsafe {
        XMapWindow(display, window);
        XRaiseWindow(display, window);
        XSync(display, 0);
    }
    window
}

unsafe fn window_border_settings(
    display: *mut libc::c_void,
    window: c_ulong,
    cfg: &config::Config,
) -> (u64, u32) {
    let mut color = cfg.border_color;
    let mut width = cfg.border_width;
    let mut hint = XClassHint {
        res_name: ptr::null_mut(),
        res_class: ptr::null_mut(),
    };
    if unsafe { XGetClassHint(display, window, &mut hint) } != 0 {
        if !hint.res_class.is_null()
            && let Ok(class_name) = unsafe { CStr::from_ptr(hint.res_class) }.to_str()
        {
            for rule in &cfg.window_rules {
                if rule.selector_key == "class" && rule.selector_val == class_name {
                    if rule.property == "border_color" {
                        let clean = rule.value.strip_prefix("0x").unwrap_or(&rule.value);
                        if let Ok(value) = u64::from_str_radix(clean, 16)
                            && value <= u32::MAX as u64
                        {
                            color = value;
                        }
                    } else if rule.property == "border_width"
                        && let Ok(value) = rule.value.parse::<u32>()
                        && value <= u16::MAX as u32
                    {
                        width = value;
                    }
                }
            }
        }
        unsafe {
            if !hint.res_name.is_null() {
                XFree(hint.res_name as *mut libc::c_void);
            }
            if !hint.res_class.is_null() {
                XFree(hint.res_class as *mut libc::c_void);
            }
        }
    }
    (color, width)
}

unsafe fn update_ewmh_atoms(
    display: *mut libc::c_void,
    root: libc::c_ulong,
    clients: &[Client],
    focused_window: libc::c_ulong,
    atoms: &Atoms,
) {
    let wins: Vec<libc::c_ulong> = clients.iter().map(|c| c.window).collect();
    unsafe {
        let data = if wins.is_empty() {
            ptr::null()
        } else {
            wins.as_ptr() as *const u8
        };
        XChangeProperty(
            display,
            root,
            atoms.net_client_list,
            XA_WINDOW,
            32,
            PROP_MODE_REPLACE,
            data,
            wins.len() as c_int,
        );
        XChangeProperty(
            display,
            root,
            atoms.net_active_window,
            XA_WINDOW,
            32,
            PROP_MODE_REPLACE,
            &focused_window as *const libc::c_ulong as *const u8,
            1,
        );
    }
}

unsafe fn set_focus(
    display: *mut libc::c_void,
    clients: &[Client],
    win: libc::c_ulong,
    focused: &mut libc::c_ulong,
    cfg: &config::Config,
    root: libc::c_ulong,
    atoms: &Atoms,
) {
    unsafe {
        if *focused != 0
            && *focused != win
            && let Some(old) = clients.iter().find(|c| c.window == *focused)
        {
            XSetWindowBorder(display, old.window, old.border_color);
        }
        if win != 0 {
            XSetWindowBorder(display, win, cfg.focused_border_color);
            XSetInputFocus(display, win, REVERT_TO_POINTER_ROOT, 0);
            XRaiseWindow(display, win);
        } else {
            XSetInputFocus(display, root, REVERT_TO_POINTER_ROOT, 0);
        }
        *focused = win;
        update_ewmh_atoms(display, root, clients, win, atoms);
    }
}

fn parse_modifier(name: &str) -> libc::c_uint {
    match name.to_lowercase().as_str() {
        "mod4" | "super" => SUPER_MASK,
        "shift" => 1 << 0,
        "control" | "ctrl" => 1 << 2,
        "mod1" | "alt" => 1 << 3,
        _ => 0,
    }
}

// ============================================================================
// 6. MAIN
// ============================================================================

fn main() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = reap as *const () as usize;
        sa.sa_flags = libc::SA_RESTART | libc::SA_NOCLDSTOP;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGCHLD, &sa, ptr::null_mut());
    }

    let mut config_path = PathBuf::from(
        std::env::var("XDG_CONFIG_HOME")
            .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap_or_default())),
    );
    config_path.push("aetherwm");
    config_path.push("config.ars");

    let mut active_config = match config::load_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("AetherWM: no usable config ({}); using defaults", e);
            config::Config::default()
        }
    };
    let compositor_config_path = config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("comp.conf");
    let mut compositor_config = comp_config::load(&compositor_config_path).unwrap_or_default();

    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            eprintln!("AetherWM Fatal: Failed to open X display.");
            std::process::exit(1);
        }

        XSetErrorHandler(Some(startup_err));

        let screen = XDefaultScreen(display);
        let root_window = XRootWindow(display, screen);

        XSelectInput(
            display,
            root_window,
            SUBSTRUCTURE_REDIRECT_MASK
                | SUBSTRUCTURE_NOTIFY_MASK
                | KEY_PRESS_MASK
                | BUTTON_PRESS_MASK
                | BUTTON_RELEASE_MASK
                | POINTER_MOTION_MASK,
        );
        XSync(display, 0);

        if OTHER_WM.load(Ordering::SeqCst) {
            eprintln!("AetherWM Fatal: Another window manager is already running on this display.");
            std::process::exit(1);
        }

        XSetErrorHandler(Some(runtime_err));

        // --- Atoms ---
        let atom = |name: &[u8]| XInternAtom(display, name.as_ptr() as *const c_char, 0);
        let atoms = Atoms {
            net_supporting: atom(b"_NET_SUPPORTING_WM_CHECK\0"),
            net_wm_name: atom(b"_NET_WM_NAME\0"),
            utf8_string: atom(b"UTF8_STRING\0"),
            net_supported: atom(b"_NET_SUPPORTED\0"),
            net_client_list: atom(b"_NET_CLIENT_LIST\0"),
            net_active_window: atom(b"_NET_ACTIVE_WINDOW\0"),
            net_wm_window_type: atom(b"_NET_WM_WINDOW_TYPE\0"),
            net_wm_window_type_dock: atom(b"_NET_WM_WINDOW_TYPE_DOCK\0"),
            net_wm_strut_partial: atom(b"_NET_WM_STRUT_PARTIAL\0"),
            wm_protocols: atom(b"WM_PROTOCOLS\0"),
            wm_delete_window: atom(b"WM_DELETE_WINDOW\0"),
        };

        // --- EWMH root properties ---
        let supporting_window = XCreateSimpleWindow(display, root_window, 0, 0, 1, 1, 0, 0, 0);
        XChangeProperty(
            display,
            root_window,
            atoms.net_supporting,
            XA_WINDOW,
            32,
            PROP_MODE_REPLACE,
            &supporting_window as *const u64 as *const u8,
            1,
        );
        XChangeProperty(
            display,
            supporting_window,
            atoms.net_supporting,
            XA_WINDOW,
            32,
            PROP_MODE_REPLACE,
            &supporting_window as *const u64 as *const u8,
            1,
        );

        let wm_name = b"AetherWM\0";
        XChangeProperty(
            display,
            supporting_window,
            atoms.net_wm_name,
            atoms.utf8_string,
            8,
            PROP_MODE_REPLACE,
            wm_name.as_ptr(),
            (wm_name.len() - 1) as c_int,
        );

        let supported_atoms = [
            atoms.net_supporting,
            atoms.net_wm_name,
            atoms.net_supported,
            atoms.net_client_list,
            atoms.net_active_window,
        ];
        XChangeProperty(
            display,
            root_window,
            atoms.net_supported,
            XA_ATOM,
            32,
            PROP_MODE_REPLACE,
            supported_atoms.as_ptr() as *const u8,
            supported_atoms.len() as c_int,
        );

        let external_compositor_configured = active_config
            .autostart
            .iter()
            .any(|cmd| cmd.split_whitespace().next() == Some("picom"));
        let compositor = RefCell::new(if !compositor_config.enabled {
            eprintln!("AetherWM: built-in compositor disabled by comp.conf");
            None
        } else if external_compositor_configured {
            eprintln!("AetherWM: using configured external compositor instead of the built-in one");
            None
        } else {
            let started = compositor::Compositor::start(
                display,
                root_window,
                screen,
                supporting_window,
                &compositor_config,
            );
            if started.is_some() {
                eprintln!("AetherWM: built-in XRender compositor enabled");
            } else {
                eprintln!(
                    "AetherWM: built-in compositor unavailable (XComposite, XDamage, XFixes, XRender, or another compositor may be the cause)"
                );
            }
            started
        });
        if let Some(comp) = compositor.borrow().as_ref() {
            comp.repaint();
        }

        for cmd in &active_config.autostart {
            let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
        }

        // --- Key bindings ---
        let lock_modifiers = [0, LOCK_MASK, MOD2_MASK, LOCK_MASK | MOD2_MASK];
        let mut precomputed_bindings: Vec<KeyBinding> = Vec::new();

        let register_bindings = |disp: *mut libc::c_void,
                                 root: libc::c_ulong,
                                 cfg: &config::Config,
                                 bound: &mut Vec<KeyBinding>| {
            bound.clear();
            for binding in &cfg.bindings {
                let mut mod_mask = 0;
                for m in &binding.modifiers {
                    mod_mask |= parse_modifier(m);
                }
                let Ok(c_key) = std::ffi::CString::new(binding.key.clone()) else {
                    continue;
                };
                let keysym = XStringToKeysym(c_key.as_ptr());
                if keysym == 0 {
                    continue;
                }
                let keycode = XKeysymToKeycode(disp, keysym);
                if keycode != 0 {
                    for lock in lock_modifiers {
                        XGrabKey(
                            disp,
                            keycode as c_int,
                            mod_mask | lock,
                            root,
                            1,
                            GRAB_MODE_ASYNC,
                            GRAB_MODE_ASYNC,
                        );
                    }
                    bound.push(KeyBinding {
                        keycode: keycode as u32,
                        modifiers: mod_mask,
                        action: binding.action.clone(),
                    });
                }
            }
        };

        register_bindings(
            display,
            root_window,
            &active_config,
            &mut precomputed_bindings,
        );

        let root_btn_mask =
            (BUTTON_PRESS_MASK | BUTTON_RELEASE_MASK | POINTER_MOTION_MASK) as libc::c_uint;
        for lock in lock_modifiers {
            for btn in [1u32, 2, 3, 4, 5] {
                XGrabButton(
                    display,
                    btn,
                    SUPER_MASK | lock,
                    root_window,
                    1,
                    root_btn_mask,
                    GRAB_MODE_ASYNC,
                    GRAB_MODE_ASYNC,
                    0,
                    0,
                );
            }
        }

        let mut clients: Vec<Client> = Vec::new();
        let mut current_view = Point { x: 0, y: 0 };
        let mut cam_offset_x: i32 = 0;
        let mut cam_offset_y: i32 = 0;
        let mut focused_window: libc::c_ulong = 0;
        let mut struts = Struts {
            top: 0,
            bottom: 0,
            left: 0,
            right: 0,
        };
        let dock_window = Cell::new(0 as libc::c_ulong);
        let mut startup_warnings = active_config.warnings.clone();
        startup_warnings.extend(compositor_config.warnings.clone());
        let config_warning_window = Cell::new(show_config_warning(
            display,
            root_window,
            screen,
            &startup_warnings,
        ));
        let zoom = Cell::new(1.0f32);
        let zoom_center = Cell::new((0i32, 0i32));
        let corner_radius = Cell::new(if compositor_config.enabled {
            compositor_config.corner_radius
        } else {
            0
        });
        let animate_next_layout = Cell::new(false);
        let mut drag: Option<Drag> = None;
        let mut event = XEvent::new();

        let grab_ptr = |t: libc::c_ulong| {
            XGrabPointer(
                display,
                root_window,
                1,
                (POINTER_MOTION_MASK | BUTTON_RELEASE_MASK) as libc::c_uint,
                GRAB_MODE_ASYNC,
                GRAB_MODE_ASYNC,
                0,
                0,
                t,
            ) == 0
        };

        let resize_client = |disp: *mut libc::c_void,
                             win: libc::c_ulong,
                             x: i32,
                             y: i32,
                             w: u32,
                             h: u32,
                             bw: u32| {
            let actual_w = w.saturating_sub(2 * bw).max(1);
            let actual_h = h.saturating_sub(2 * bw).max(1);
            if animate_next_layout.get() {
                if let Some(comp) = compositor.borrow().as_ref() {
                    comp.animate_geometry(win, x, y, actual_w, actual_h);
                } else {
                    XMoveResizeWindow(disp, win, x, y, actual_w, actual_h);
                }
            } else {
                XMoveResizeWindow(disp, win, x, y, actual_w, actual_h);
            }

            let radius = corner_radius.get().min(actual_w.min(actual_h) / 2);
            let mut rounded_shape = Vec::with_capacity(actual_h as usize);
            for row in 0..actual_h {
                let inset = if radius == 0 || (row >= radius && row < actual_h - radius) {
                    0
                } else {
                    let edge_distance = if row < radius {
                        radius - row
                    } else {
                        row - (actual_h - radius) + 1
                    };
                    let r = radius as f64;
                    (r - (r * r - (r - edge_distance as f64).powi(2)).max(0.0).sqrt()).ceil() as u32
                };
                let left = inset.min(actual_w / 2);
                rounded_shape.push(XRectangle {
                    x: left.min(i16::MAX as u32) as i16,
                    y: row.min(i16::MAX as u32) as i16,
                    width: actual_w.saturating_sub(left * 2).min(u16::MAX as u32) as u16,
                    height: 1,
                });
            }
            XShapeCombineRectangles(
                disp,
                win,
                SHAPE_BOUNDING,
                0,
                0,
                rounded_shape.as_mut_ptr(),
                rounded_shape.len() as c_int,
                SHAPE_SET,
                UNSORTED,
            );
            let mut ce = XEvent::new();
            ce.xconfigure.type_ = 22;
            ce.xconfigure.display = disp;
            ce.xconfigure.event = win;
            ce.xconfigure.window = win;
            ce.xconfigure.x = x;
            ce.xconfigure.y = y;
            ce.xconfigure.width = actual_w as i32;
            ce.xconfigure.height = actual_h as i32;
            ce.xconfigure.border_width = bw as i32;
            XSendEvent(disp, win, 0, STRUCTURE_NOTIFY_MASK, &mut ce);
        };

        // Multi-monitor aware viewport application with per-monitor visibility clipping
        let apply_camera_viewport = |disp: *mut libc::c_void,
                                     cl_vec: &mut Vec<Client>,
                                     cur: Point,
                                     off_x: i32,
                                     off_y: i32,
                                     s: &Struts| {
            let monitors = get_monitors(disp, root_window, screen);
            let (ux, uy, uw, uh) = primary_usable_area(disp, root_window, screen, s);
            let z = zoom.get();
            let cx = ux + uw / 2;
            let cy = uy + uh / 2;
            zoom_center.set((cx, cy));

            for c in cl_vec.iter_mut() {
                let cell_dx = (c.pos.x - cur.x) * uw;
                let cell_dy = (c.pos.y - cur.y) * uh;
                let (sx, sy, zw, zh) = zoom_rect(
                    c.x + cell_dx + off_x,
                    c.y + cell_dy + off_y,
                    c.width,
                    c.height,
                    z,
                    cx,
                    cy,
                );
                let (w, h) = (zw as i32, zh as i32);

                let visible = monitors.iter().any(|mon| {
                    let mux = mon.x + s.left;
                    let muy = mon.y + s.top;
                    let muw = (mon.width - s.left - s.right).max(1);
                    let muh = (mon.height - s.top - s.bottom).max(1);
                    sx + w > mux && sx < mux + muw && sy + h > muy && sy < muy + muh
                });

                if visible {
                    if !c.mapped {
                        XMapWindow(disp, c.window);
                        c.mapped = true;
                    }
                    resize_client(disp, c.window, sx, sy, zw, zh, c.border_width);
                } else if c.mapped {
                    c.ignore_unmaps += 1;
                    XUnmapWindow(disp, c.window);
                    c.mapped = false;
                }
            }
            let dock = dock_window.get();
            if dock != 0 {
                XRaiseWindow(disp, dock);
            }
            if let Some(comp) = compositor.borrow().as_ref() {
                comp.repaint();
            } else {
                XFlush(disp);
            }
        };

        // Monitor-aware tile layout
        let tile_layout = |disp: *mut libc::c_void,
                           cl_vec: &mut Vec<Client>,
                           cur: Point,
                           cfg: &config::Config,
                           s: &Struts,
                           off_x: i32,
                           off_y: i32| {
            let og = cfg.outer_gap as i32;
            let ig = cfg.inner_gap as i32;

            let mut groups: HashMap<(i32, i32, i32, i32), Vec<usize>> = HashMap::new();
            for (i, c) in cl_vec.iter().enumerate() {
                if !c.floating && c.pos == cur {
                    groups
                        .entry((c.mux, c.muy, c.muw, c.muh))
                        .or_default()
                        .push(i);
                }
            }

            for ((mux, muy, muw, muh), idxs) in groups {
                let count = idxs.len();
                if count == 0 {
                    continue;
                }
                let avail_w = muw - og * 2;
                let avail_h = muh - og * 2;

                if count == 1 {
                    let c = &mut cl_vec[idxs[0]];
                    c.x = mux + og;
                    c.y = muy + og;
                    c.width = avail_w.max(1) as u32;
                    c.height = avail_h.max(1) as u32;
                } else {
                    let master_w = ((avail_w - ig) as f32 * cfg.master_ratio) as i32;
                    let stack_w = avail_w - master_w - ig;
                    let n = (count - 1) as i32;
                    let single_h = ((avail_h - ig * (n - 1)) / n).max(1);
                    let stack_x = mux + og + master_w + ig;

                    {
                        let m = &mut cl_vec[idxs[0]];
                        m.x = mux + og;
                        m.y = muy + og;
                        m.width = master_w.max(1) as u32;
                        m.height = avail_h.max(1) as u32;
                    }
                    for (k, &i) in idxs.iter().skip(1).enumerate() {
                        let sc = &mut cl_vec[i];
                        sc.x = stack_x;
                        sc.y = muy + og + (k as i32 * (single_h + ig));
                        sc.width = stack_w.max(1) as u32;
                        sc.height = single_h as u32;
                    }
                }
            }
            apply_camera_viewport(disp, cl_vec, cur, off_x, off_y, s);
        };

        let refocus =
            |cl: &Vec<Client>, view: Point, foc: &mut libc::c_ulong, cfg: &config::Config| {
                let next = cl
                    .iter()
                    .rev()
                    .find(|c| c.pos == view && c.mapped)
                    .map(|c| c.window)
                    .unwrap_or(0);
                set_focus(display, cl, next, foc, cfg, root_window, &atoms);
            };

        // ====================================================================
        // 7. EVENT LOOP
        // ====================================================================
        loop {
            XNextEvent(display, &mut event);
            let event_type = event.type_;

            match event_type {
                MAP_REQUEST => {
                    let win = event.xmaprequest.window;
                    if clients.iter().any(|c| c.window == win) {
                        continue;
                    }
                    if let Some(comp) = compositor.borrow_mut().as_mut() {
                        comp.watch_window(win);
                    }

                    let mut actual_type = 0;
                    let mut actual_format = 0;
                    let mut nitems = 0;
                    let mut bytes_after = 0;
                    let mut prop_data: *mut u8 = ptr::null_mut();

                    if XGetWindowProperty(
                        display,
                        win,
                        atoms.net_wm_window_type,
                        0,
                        32,
                        0,
                        XA_ATOM,
                        &mut actual_type,
                        &mut actual_format,
                        &mut nitems,
                        &mut bytes_after,
                        &mut prop_data,
                    ) == 0
                        && !prop_data.is_null()
                    {
                        let slice = std::slice::from_raw_parts(
                            prop_data as *const libc::c_ulong,
                            nitems as usize,
                        );
                        let is_dock = slice.contains(&atoms.net_wm_window_type_dock);
                        XFree(prop_data as *mut libc::c_void);

                        if is_dock {
                            dock_window.set(win);
                            XSelectInput(
                                display,
                                win,
                                PROPERTY_CHANGE_MASK | STRUCTURE_NOTIFY_MASK,
                            );
                            if let Some(s) = read_struts(display, win, atoms.net_wm_strut_partial) {
                                struts = s;
                            }
                            XMapWindow(display, win);
                            tile_layout(
                                display,
                                &mut clients,
                                current_view,
                                &active_config,
                                &struts,
                                cam_offset_x,
                                cam_offset_y,
                            );
                            continue;
                        }
                    }

                    XSelectInput(display, win, CLIENT_EVENT_MASK);

                    let (win_border_color, win_border_width) =
                        window_border_settings(display, win, &active_config);

                    XSetWindowBorderWidth(display, win, win_border_width);
                    XSetWindowBorder(display, win, win_border_color);

                    cam_offset_x = 0;
                    cam_offset_y = 0;

                    let (mux, muy, muw, muh) =
                        monitor_usable_area_under_cursor(display, root_window, screen, &struts);

                    clients.push(Client {
                        window: win,
                        pos: current_view,
                        floating: active_config.default_layout == "floating",
                        mapped: false,
                        ignore_unmaps: 0,
                        border_color: win_border_color,
                        border_width: win_border_width,
                        x: mux + 120,
                        y: muy + 120,
                        width: ((muw / 2).max(400)) as u32,
                        height: ((muh / 2).max(300)) as u32,
                        mux,
                        muy,
                        muw,
                        muh,
                    });

                    animate_next_layout.set(true);
                    tile_layout(
                        display,
                        &mut clients,
                        current_view,
                        &active_config,
                        &struts,
                        cam_offset_x,
                        cam_offset_y,
                    );
                    animate_next_layout.set(false);
                    XSync(display, 0);

                    XGrabButton(
                        display,
                        ANY_BUTTON,
                        ANY_MODIFIER,
                        win,
                        0,
                        BUTTON_PRESS_MASK as libc::c_uint,
                        GRAB_MODE_SYNC,
                        GRAB_MODE_ASYNC,
                        0,
                        0,
                    );
                    set_focus(
                        display,
                        &clients,
                        win,
                        &mut focused_window,
                        &active_config,
                        root_window,
                        &atoms,
                    );
                }

                MAP_NOTIFY => {
                    let ev = event.xmap;
                    if ev.event == root_window
                        && let Some(comp) = compositor.borrow_mut().as_mut()
                    {
                        comp.watch_window(ev.window);
                        comp.repaint();
                    }
                }

                CONFIGURE_REQUEST => {
                    let ev = event.xconfigurerequest;
                    let mut ch = XWindowChanges {
                        x: ev.x,
                        y: ev.y,
                        width: ev.width,
                        height: ev.height,
                        border_width: ev.border_width,
                        sibling: ev.above,
                        stack_mode: ev.detail,
                    };
                    match clients.iter().position(|c| c.window == ev.window) {
                        Some(i) if clients[i].floating => {
                            let bw2 = 2 * clients[i].border_width;
                            if ev.value_mask & 1 != 0 {
                                clients[i].x = ev.x - cam_offset_x;
                            }
                            if ev.value_mask & 2 != 0 {
                                clients[i].y = ev.y - cam_offset_y;
                            }
                            if ev.value_mask & 4 != 0 {
                                clients[i].width = ev.width as u32 + bw2;
                            }
                            if ev.value_mask & 8 != 0 {
                                clients[i].height = ev.height as u32 + bw2;
                            }
                            apply_camera_viewport(
                                display,
                                &mut clients,
                                current_view,
                                cam_offset_x,
                                cam_offset_y,
                                &struts,
                            );
                        }
                        Some(_) => {
                            tile_layout(
                                display,
                                &mut clients,
                                current_view,
                                &active_config,
                                &struts,
                                cam_offset_x,
                                cam_offset_y,
                            );
                        }
                        None => {
                            XConfigureWindow(
                                display,
                                ev.window,
                                (ev.value_mask & !16) as libc::c_uint,
                                &mut ch,
                            );
                        }
                    }
                }

                BUTTON_PRESS => {
                    let ev = event.xbutton;
                    let from_client_grab = ev.window != root_window;
                    let target_win = if ev.window == root_window {
                        ev.subwindow
                    } else {
                        ev.window
                    };
                    let sup = ev.state & SUPER_MASK != 0;
                    let pure_state = ev.state & !(LOCK_MASK | MOD2_MASK);
                    let on_client = clients.iter().any(|c| c.window == target_win);

                    if drag.is_some() {
                        // ignore presses mid-drag
                    } else if sup && (ev.button == 4 || ev.button == 5) {
                        let z = zoom.get();
                        let nz = if ev.button == 4 { z * 1.15 } else { z / 1.15 };
                        let target = nz.clamp(MIN_ZOOM, MAX_ZOOM);
                        animate_zoom(
                            z,
                            target,
                            compositor_config.animations,
                            compositor_config.animation_duration_ms,
                            |frame_zoom| {
                                zoom.set(frame_zoom);
                                apply_camera_viewport(
                                    display,
                                    &mut clients,
                                    current_view,
                                    cam_offset_x,
                                    cam_offset_y,
                                    &struts,
                                );
                            },
                        );
                    } else if (sup && ev.button == 2)
                        || (ev.window == root_window
                            && !on_client
                            && ev.button == 1
                            && pure_state == 0)
                    {
                        if grab_ptr(ev.time) {
                            drag = Some(Drag::Pan {
                                sx: ev.x_root,
                                sy: ev.y_root,
                                off_x: cam_offset_x,
                                off_y: cam_offset_y,
                            });
                        }
                    } else if let Some(c) = clients.iter().find(|c| c.window == target_win) {
                        if sup && ev.button == 1 {
                            if grab_ptr(ev.time) {
                                drag = Some(Drag::Move {
                                    win: target_win,
                                    sx: ev.x_root,
                                    sy: ev.y_root,
                                    ox: c.x,
                                    oy: c.y,
                                });
                            }
                        } else if sup && ev.button == 3 {
                            if grab_ptr(ev.time) {
                                drag = Some(Drag::Resize {
                                    win: target_win,
                                    sx: ev.x_root,
                                    sy: ev.y_root,
                                    ow: c.width as i32,
                                    oh: c.height as i32,
                                });
                            }
                        } else if !sup {
                            set_focus(
                                display,
                                &clients,
                                target_win,
                                &mut focused_window,
                                &active_config,
                                root_window,
                                &atoms,
                            );
                        }
                    }

                    if from_client_grab {
                        let mode = if drag.is_some() {
                            ASYNC_POINTER
                        } else {
                            REPLAY_POINTER
                        };
                        XAllowEvents(display, mode, ev.time);
                    }
                }

                MOTION_NOTIFY => {
                    let m = event.xmotion;
                    let z = zoom.get();
                    let (zcx, zcy) = zoom_center.get();
                    match drag {
                        Some(Drag::Pan {
                            sx,
                            sy,
                            off_x,
                            off_y,
                        }) => {
                            cam_offset_x = off_x + ((m.x_root - sx) as f32 / z) as i32;
                            cam_offset_y = off_y + ((m.y_root - sy) as f32 / z) as i32;
                            apply_camera_viewport(
                                display,
                                &mut clients,
                                current_view,
                                cam_offset_x,
                                cam_offset_y,
                                &struts,
                            );
                        }
                        Some(Drag::Move {
                            win,
                            sx,
                            sy,
                            ox,
                            oy,
                        }) => {
                            let (_, _, uw, uh) =
                                primary_usable_area(display, root_window, screen, &struts);
                            if let Some(c) = clients.iter_mut().find(|c| c.window == win) {
                                c.x = ox + ((m.x_root - sx) as f32 / z) as i32;
                                c.y = oy + ((m.y_root - sy) as f32 / z) as i32;
                                let cdx = (c.pos.x - current_view.x) * uw;
                                let cdy = (c.pos.y - current_view.y) * uh;
                                let (rx, ry, rw, rh) = zoom_rect(
                                    c.x + cdx + cam_offset_x,
                                    c.y + cdy + cam_offset_y,
                                    c.width,
                                    c.height,
                                    z,
                                    zcx,
                                    zcy,
                                );
                                resize_client(display, win, rx, ry, rw, rh, c.border_width);
                            }
                            if let Some(comp) = compositor.borrow().as_ref() {
                                comp.repaint();
                            }
                        }
                        Some(Drag::Resize {
                            win,
                            sx,
                            sy,
                            ow,
                            oh,
                        }) => {
                            let (_, _, uw, uh) =
                                primary_usable_area(display, root_window, screen, &struts);
                            if let Some(c) = clients.iter_mut().find(|c| c.window == win) {
                                c.width =
                                    (ow + ((m.x_root - sx) as f32 / z) as i32).max(120) as u32;
                                c.height =
                                    (oh + ((m.y_root - sy) as f32 / z) as i32).max(80) as u32;
                                let cdx = (c.pos.x - current_view.x) * uw;
                                let cdy = (c.pos.y - current_view.y) * uh;
                                let (rx, ry, rw, rh) = zoom_rect(
                                    c.x + cdx + cam_offset_x,
                                    c.y + cdy + cam_offset_y,
                                    c.width,
                                    c.height,
                                    z,
                                    zcx,
                                    zcy,
                                );
                                resize_client(display, win, rx, ry, rw, rh, c.border_width);
                            }
                            if let Some(comp) = compositor.borrow().as_ref() {
                                comp.repaint();
                            }
                        }
                        None => {}
                    }
                }

                BUTTON_RELEASE => {
                    if let Some(d) = drag.take() {
                        XUngrabPointer(display, 0);
                        let (ux, _uy, uw, uh) =
                            primary_usable_area(display, root_window, screen, &struts);

                        match d {
                            Drag::Pan { .. } => {
                                let sx = (-cam_offset_x as f32 / uw as f32).round() as i32;
                                let sy = (-cam_offset_y as f32 / uh as f32).round() as i32;
                                if sx != 0 || sy != 0 {
                                    current_view.x += sx;
                                    current_view.y += sy;
                                    cam_offset_x += sx * uw;
                                    cam_offset_y += sy * uh;
                                    tile_layout(
                                        display,
                                        &mut clients,
                                        current_view,
                                        &active_config,
                                        &struts,
                                        cam_offset_x,
                                        cam_offset_y,
                                    );
                                    let still = clients.iter().any(|c| {
                                        c.window == focused_window
                                            && c.pos == current_view
                                            && c.mapped
                                    });
                                    if !still {
                                        refocus(
                                            &clients,
                                            current_view,
                                            &mut focused_window,
                                            &active_config,
                                        );
                                    }
                                }
                            }
                            Drag::Move { win, .. } => {
                                if let Some(i) = clients.iter().position(|c| c.window == win) {
                                    let (_, uy, _, _) =
                                        primary_usable_area(display, root_window, screen, &struts);
                                    let c = &mut clients[i];
                                    let lx = c.x + (c.pos.x - current_view.x) * uw;
                                    let ly = c.y + (c.pos.y - current_view.y) * uh;
                                    let kx = (lx + c.width as i32 / 2 - ux).div_euclid(uw);
                                    let ky = (ly + c.height as i32 / 2 - uy).div_euclid(uh);
                                    c.pos.x = current_view.x + kx;
                                    c.pos.y = current_view.y + ky;
                                    c.x = lx - kx * uw;
                                    c.y = ly - ky * uh;
                                    c.floating = true;
                                    tile_layout(
                                        display,
                                        &mut clients,
                                        current_view,
                                        &active_config,
                                        &struts,
                                        cam_offset_x,
                                        cam_offset_y,
                                    );
                                }
                            }
                            Drag::Resize { win, .. } => {
                                if let Some(c) = clients.iter_mut().find(|c| c.window == win) {
                                    c.floating = true;
                                }
                                tile_layout(
                                    display,
                                    &mut clients,
                                    current_view,
                                    &active_config,
                                    &struts,
                                    cam_offset_x,
                                    cam_offset_y,
                                );
                            }
                        }
                    }
                }

                PROPERTY_NOTIFY => {
                    let ev = event.xproperty;
                    let dock = dock_window.get();
                    if dock != 0
                        && ev.window == dock
                        && ev.atom == atoms.net_wm_strut_partial
                        && let Some(s) = read_struts(display, dock, atoms.net_wm_strut_partial)
                    {
                        struts = s;
                        tile_layout(
                            display,
                            &mut clients,
                            current_view,
                            &active_config,
                            &struts,
                            cam_offset_x,
                            cam_offset_y,
                        );
                    }
                }

                UNMAP_NOTIFY => {
                    let ev = event.xunmap;
                    let win = ev.window;
                    if let Some(comp) = compositor.borrow_mut().as_mut() {
                        comp.finish_unmap(win);
                        comp.repaint();
                    }
                    if ev.event != ev.window {
                        continue;
                    }
                    if win == dock_window.get() {
                        dock_window.set(0);
                        struts = Struts {
                            top: 0,
                            bottom: 0,
                            left: 0,
                            right: 0,
                        };
                        tile_layout(
                            display,
                            &mut clients,
                            current_view,
                            &active_config,
                            &struts,
                            cam_offset_x,
                            cam_offset_y,
                        );
                    }
                    if let Some(pos) = clients.iter().position(|c| c.window == win) {
                        if clients[pos].ignore_unmaps > 0 {
                            clients[pos].ignore_unmaps -= 1;
                        } else {
                            clients.remove(pos);
                            if focused_window == win {
                                focused_window = 0;
                                refocus(
                                    &clients,
                                    current_view,
                                    &mut focused_window,
                                    &active_config,
                                );
                            }
                            tile_layout(
                                display,
                                &mut clients,
                                current_view,
                                &active_config,
                                &struts,
                                cam_offset_x,
                                cam_offset_y,
                            );
                            update_ewmh_atoms(
                                display,
                                root_window,
                                &clients,
                                focused_window,
                                &atoms,
                            );
                        }
                    }
                }

                DESTROY_NOTIFY => {
                    let win = event.xdestroywindow.window;
                    if let Some(comp) = compositor.borrow_mut().as_mut() {
                        comp.forget_window(win);
                    }
                    if win == dock_window.get() {
                        dock_window.set(0);
                        struts = Struts {
                            top: 0,
                            bottom: 0,
                            left: 0,
                            right: 0,
                        };
                        tile_layout(
                            display,
                            &mut clients,
                            current_view,
                            &active_config,
                            &struts,
                            cam_offset_x,
                            cam_offset_y,
                        );
                    }
                    if let Some(pos) = clients.iter().position(|c| c.window == win) {
                        clients.remove(pos);
                        if focused_window == win {
                            focused_window = 0;
                            refocus(&clients, current_view, &mut focused_window, &active_config);
                        }
                        tile_layout(
                            display,
                            &mut clients,
                            current_view,
                            &active_config,
                            &struts,
                            cam_offset_x,
                            cam_offset_y,
                        );
                        update_ewmh_atoms(display, root_window, &clients, focused_window, &atoms);
                    }
                }

                KEY_PRESS => {
                    let keycode = event.xkey.keycode;
                    let state = event.xkey.state & !(LOCK_MASK | MOD2_MASK);

                    let found = precomputed_bindings
                        .iter()
                        .find(|b| keycode == b.keycode && state == b.modifiers)
                        .map(|b| b.action.clone());

                    if let Some(action) = found {
                        match action.as_str() {
                            "killclient" => {
                                if focused_window != 0 {
                                    let animated_close =
                                        if let Some(comp) = compositor.borrow_mut().as_mut() {
                                            comp.animate_close(focused_window)
                                        } else {
                                            false
                                        };
                                    if supports_delete(
                                        display,
                                        focused_window,
                                        atoms.wm_delete_window,
                                    ) {
                                        let mut cm = XEvent::new();
                                        cm.type_ = 33;
                                        cm.xclient.window = focused_window;
                                        cm.xclient.message_type = atoms.wm_protocols;
                                        cm.xclient.format = 32;
                                        cm.xclient.data[0] = atoms.wm_delete_window;
                                        XSendEvent(display, focused_window, 0, 0, &mut cm);
                                        if animated_close {
                                            XSync(display, 0);
                                            std::thread::sleep(std::time::Duration::from_millis(
                                                90,
                                            ));
                                            if let Some(comp) = compositor.borrow_mut().as_mut()
                                                && comp.is_viewable(focused_window)
                                            {
                                                comp.cancel_close(focused_window);
                                            }
                                        }
                                    } else {
                                        XKillClient(display, focused_window);
                                    }
                                }
                            }
                            "togglefloating" => {
                                if let Some(c) =
                                    clients.iter_mut().find(|c| c.window == focused_window)
                                {
                                    c.floating = !c.floating;
                                    tile_layout(
                                        display,
                                        &mut clients,
                                        current_view,
                                        &active_config,
                                        &struts,
                                        cam_offset_x,
                                        cam_offset_y,
                                    );
                                }
                            }
                            "toggleoverview" | "zoom_in" | "zoom_out" | "zoom_reset" => {
                                let z = zoom.get();
                                let nz = match action.as_str() {
                                    "zoom_out" => z / 1.25,
                                    "zoom_in" => z * 1.25,
                                    "zoom_reset" => 1.0,
                                    _ => {
                                        if z < 1.0 {
                                            1.0
                                        } else {
                                            0.4
                                        }
                                    }
                                };
                                let target = nz.clamp(MIN_ZOOM, MAX_ZOOM);
                                animate_zoom(
                                    z,
                                    target,
                                    compositor_config.animations,
                                    compositor_config.animation_duration_ms,
                                    |frame_zoom| {
                                        zoom.set(frame_zoom);
                                        tile_layout(
                                            display,
                                            &mut clients,
                                            current_view,
                                            &active_config,
                                            &struts,
                                            cam_offset_x,
                                            cam_offset_y,
                                        );
                                    },
                                );
                            }
                            "reload_config" => match config::load_config(&config_path) {
                                Ok(new_cfg) => {
                                    XUngrabKey(display, 0, ANY_MODIFIER, root_window);
                                    active_config = new_cfg;
                                    compositor_config = match comp_config::load(
                                        &compositor_config_path,
                                    ) {
                                        Ok(cfg) => cfg,
                                        Err(error) => {
                                            eprintln!(
                                                "AetherWM compositor config reload error: {error}"
                                            );
                                            compositor_config.clone()
                                        }
                                    };
                                    corner_radius.set(if compositor_config.enabled {
                                        compositor_config.corner_radius
                                    } else {
                                        0
                                    });
                                    {
                                        let mut slot = compositor.borrow_mut();
                                        let should_run = compositor_config.enabled
                                            && !external_compositor_configured;
                                        match (
                                            should_run,
                                            slot.as_ref().map(|comp| comp.is_active()),
                                        ) {
                                            (false, Some(_)) => *slot = None,
                                            (true, Some(true)) => {
                                                if let Some(comp) = slot.as_mut() {
                                                    comp.update_effects(&compositor_config);
                                                }
                                            }
                                            (true, Some(false) | None) => {
                                                *slot = None;
                                                *slot = compositor::Compositor::start(
                                                    display,
                                                    root_window,
                                                    screen,
                                                    supporting_window,
                                                    &compositor_config,
                                                );
                                            }
                                            (false, None) => {}
                                        }
                                        if let Some(comp) = slot.as_mut() {
                                            for client in
                                                clients.iter().filter(|client| client.mapped)
                                            {
                                                comp.watch_window(client.window);
                                            }
                                            comp.repaint();
                                        }
                                    }
                                    let old_warning = config_warning_window.replace(0);
                                    if old_warning != 0 {
                                        XDestroyWindow(display, old_warning);
                                    }
                                    let mut reload_warnings = active_config.warnings.clone();
                                    reload_warnings.extend(compositor_config.warnings.clone());
                                    config_warning_window.set(show_config_warning(
                                        display,
                                        root_window,
                                        screen,
                                        &reload_warnings,
                                    ));
                                    register_bindings(
                                        display,
                                        root_window,
                                        &active_config,
                                        &mut precomputed_bindings,
                                    );
                                    for client in clients.iter_mut() {
                                        let (color, width) = window_border_settings(
                                            display,
                                            client.window,
                                            &active_config,
                                        );
                                        client.border_color = color;
                                        client.border_width = width;
                                        XSetWindowBorderWidth(display, client.window, width);
                                        XSetWindowBorder(display, client.window, color);
                                    }
                                    let focused = focused_window;
                                    set_focus(
                                        display,
                                        &clients,
                                        focused,
                                        &mut focused_window,
                                        &active_config,
                                        root_window,
                                        &atoms,
                                    );
                                    tile_layout(
                                        display,
                                        &mut clients,
                                        current_view,
                                        &active_config,
                                        &struts,
                                        cam_offset_x,
                                        cam_offset_y,
                                    );
                                    println!("AetherWM: config reloaded");
                                }
                                Err(e) => eprintln!("AetherWM config reload error: {}", e),
                            },
                            "matrix_left" | "matrix_right" | "matrix_up" | "matrix_down" => {
                                let (dx, dy) = match action.as_str() {
                                    "matrix_left" => (-1, 0),
                                    "matrix_right" => (1, 0),
                                    "matrix_up" => (0, -1),
                                    _ => (0, 1),
                                };
                                current_view.x += dx;
                                current_view.y += dy;
                                cam_offset_x = 0;
                                cam_offset_y = 0;
                                animate_next_layout.set(true);
                                tile_layout(
                                    display,
                                    &mut clients,
                                    current_view,
                                    &active_config,
                                    &struts,
                                    0,
                                    0,
                                );
                                animate_next_layout.set(false);
                                refocus(
                                    &clients,
                                    current_view,
                                    &mut focused_window,
                                    &active_config,
                                );
                            }
                            a if a.starts_with("exec ") => {
                                let cmd = &a["exec ".len()..];
                                let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
                            }
                            _ => {}
                        }
                    }
                }

                _ => {}
            }

            let opacity_changed = event_type == PROPERTY_NOTIFY
                && compositor
                    .borrow()
                    .as_ref()
                    .is_some_and(|comp| comp.is_opacity_property(event.xproperty.atom));
            if (opacity_changed
                || event_type == CONFIGURE_REQUEST
                || event_type == DESTROY_NOTIFY
                || compositor
                    .borrow()
                    .as_ref()
                    .is_some_and(|comp| comp.is_damage_event(event_type)))
                && let Some(comp) = compositor.borrow().as_ref()
            {
                comp.repaint();
            }
        }
    }
}
