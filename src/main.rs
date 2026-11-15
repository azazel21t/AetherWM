// ==========================================
// AetherWM - Arch Rice Edition
// ==========================================

use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_ulong};
use std::ptr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

// Modules
mod config;

#[allow(non_camel_case_types)]
type XErrorHandler = Option<unsafe extern "C" fn(*mut libc::c_void, *mut XErrorEvent) -> libc::c_int>;

#[allow(dead_code)]
#[repr(C)]
struct XErrorEvent {
    type_: libc::c_int,
    display: *mut libc::c_void,
    resourceid: libc::c_ulong,
    serial: libc::c_ulong,
    error_code: libc::c_uchar,
    request_code: libc::c_uchar,
    minor_code: libc::c_uchar,
}

#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(display_name: *const c_char) -> *mut libc::c_void;
    fn XDefaultScreen(display: *mut libc::c_void) -> c_int;
    fn XRootWindow(display: *mut libc::c_void, screen_number: c_int) -> c_ulong;
    fn XDisplayWidth(display: *mut libc::c_void, screen_number: c_int) -> c_int;
    fn XDisplayHeight(display: *mut libc::c_void, screen_number: c_int) -> c_int;
    fn XSelectInput(display: *mut libc::c_void, w: c_ulong, event_mask: libc::c_long) -> c_int;
    fn XSync(display: *mut libc::c_void, discard: c_int) -> c_int;
    fn XNextEvent(display: *mut libc::c_void, event_return: *mut XEvent) -> c_int;
    fn XMapWindow(display: *mut libc::c_void, w: c_ulong) -> c_int;
    fn XUnmapWindow(display: *mut libc::c_void, w: c_ulong) -> c_int;
    fn XMoveResizeWindow(display: *mut libc::c_void, w: c_ulong, x: c_int, y: c_int, width: libc::c_uint, height: libc::c_uint) -> c_int;
    fn XConfigureWindow(display: *mut libc::c_void, w: c_ulong, value_mask: libc::c_uint, values: *mut XWindowChanges) -> c_int;
    fn XSetWindowBorderWidth(display: *mut libc::c_void, w: c_ulong, width: libc::c_uint) -> c_int;
    fn XSetWindowBorder(display: *mut libc::c_void, w: c_ulong, border_color: libc::c_ulong) -> c_int;
    fn XSetInputFocus(display: *mut libc::c_void, focus: c_ulong, revert_to: c_int, time: libc::c_ulong) -> c_int;
    fn XRaiseWindow(display: *mut libc::c_void, w: c_ulong) -> c_int;
    fn XStringToKeysym(string: *const c_char) -> libc::c_ulong;
    fn XKeysymToKeycode(display: *mut libc::c_void, keysym: libc::c_ulong) -> libc::c_uchar;
    fn XKeycodeToKeysym(display: *mut libc::c_void, keycode: libc::c_uchar, index: c_int) -> libc::c_ulong;
    fn XGrabKey(display: *mut libc::c_void, keycode: c_int, modifiers: libc::c_uint, grab_window: c_ulong, owner_events: c_int, pointer_mode: c_int, keyboard_mode: c_int) -> c_int;
    fn XUngrabKey(display: *mut libc::c_void, keycode: c_int, modifiers: libc::c_uint, grab_window: c_ulong) -> c_int;
    fn XGrabButton(display: *mut libc::c_void, button: libc::c_uint, modifiers: libc::c_uint, grab_window: c_ulong, owner_events: c_int, event_mask: libc::c_uint, pointer_mode: c_int, keyboard_mode: c_int, confine_to: c_ulong, cursor: c_ulong) -> c_int;
    #[allow(dead_code)]
    fn XUngrabKeyboard(display: *mut libc::c_void, time: libc::c_ulong) -> c_int;
    fn XAllowEvents(display: *mut libc::c_void, event_mode: c_int, time: libc::c_ulong) -> c_int;
    fn XGetClassHint(display: *mut libc::c_void, window: c_ulong, class_hints_return: *mut XClassHint) -> c_int;
    fn XInternAtom(display: *mut libc::c_void, atom_name: *const c_char, only_if_exists: c_int) -> libc::c_ulong;
    fn XChangeProperty(display: *mut libc::c_void, w: c_ulong, property: libc::c_ulong, type_: libc::c_ulong, format: c_int, mode: c_int, data: *const u8, nelements: c_int) -> c_int;
    fn XCreateSimpleWindow(display: *mut libc::c_void, parent: c_ulong, x: c_int, y: c_int, width: libc::c_uint, height: libc::c_uint, border_width: libc::c_uint, border: c_ulong, background: c_ulong) -> c_ulong;
    fn XSendEvent(display: *mut libc::c_void, window: c_ulong, propagate: c_int, event_mask: libc::c_long, event_send: *mut XEvent) -> c_int;
    fn XFree(data: *mut libc::c_void) -> c_int;
    fn XSetErrorHandler(handler: XErrorHandler) -> XErrorHandler;
}

// X11 Constants & Atoms
const KEY_PRESS_MASK: libc::c_long = 1 << 0;
const ENTER_WINDOW_MASK: libc::c_long = 1 << 4;
const BUTTON_PRESS_MASK: libc::c_long = 1 << 2;
const BUTTON_RELEASE_MASK: libc::c_long = 1 << 3;
const POINTER_MOTION_MASK: libc::c_long = 1 << 6;
const STRUCTURE_NOTIFY_MASK: libc::c_long = 1 << 17;
const SUBSTRUCTURE_NOTIFY_MASK: libc::c_long = 1 << 19;
const SUBSTRUCTURE_REDIRECT_MASK: libc::c_long = 1 << 20;
const FOCUS_CHANGE_MASK: libc::c_long = 1 << 21;
const PROPERTY_CHANGE_MASK: libc::c_long = 1 << 22;

const CLIENT_EVENT_MASK: libc::c_long = 
    STRUCTURE_NOTIFY_MASK | PROPERTY_CHANGE_MASK | ENTER_WINDOW_MASK | FOCUS_CHANGE_MASK;

const MAP_REQUEST: i32 = 20;
const CONFIGURE_REQUEST: i32 = 23;
const UNMAP_NOTIFY: i32 = 18;
const DESTROY_NOTIFY: i32 = 17;
const KEY_PRESS: i32 = 2;
const BUTTON_PRESS: i32 = 4;

const LOCK_MASK: libc::c_uint = 1 << 1;
const MOD2_MASK: libc::c_uint = 1 << 4;
const GRAB_MODE_ASYNC: libc::c_int = 1;
const GRAB_MODE_SYNC: libc::c_int = 0;
const REVERT_TO_POINTER_ROOT: libc::c_int = 1;
const REPLAY_POINTER: libc::c_int = 2;
const ANY_BUTTON: libc::c_uint = 0;
const ANY_MODIFIER: libc::c_uint = 1 << 15;

const XA_WINDOW: libc::c_ulong = 33;
const PROP_MODE_REPLACE: c_int = 0;

static OTHER_WM: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn startup_err(_: *mut libc::c_void, _: *mut XErrorEvent) -> libc::c_int {
    OTHER_WM.store(true, Ordering::SeqCst);
    0
}

unsafe extern "C" fn runtime_err(_: *mut libc::c_void, _: *mut XErrorEvent) -> libc::c_int {
    0
}

#[allow(dead_code)]
#[repr(C)]
struct XClassHint {
    res_name: *mut c_char,
    res_class: *mut c_char,
}

#[allow(dead_code)]
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
    xconfigurerequest: XConfigureRequestEvent,
    xconfigure: XConfigureEvent,
    xunmap: XUnmapEvent,
    xdestroywindow: XDestroyWindowEvent,
    xclient: XClientMessageEvent,
    _pad: [u64; 24],
}

impl XEvent {
    fn new() -> Self {
        Self { _pad: [0; 24] }
    }
}

#[derive(Clone)]
struct Client {
    window: libc::c_ulong,
    tag: usize,
    floating: bool,
    mapped: bool,
    ignore_unmaps: u32,
    border_color: u64,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

unsafe fn set_focus(
    display: *mut libc::c_void,
    clients: &[Client],
    win: libc::c_ulong,
    focused: &mut libc::c_ulong,
    cfg: &config::Config,
) {
    unsafe {
        if *focused != 0 && *focused != win {
            if let Some(old) = clients.iter().find(|c| c.window == *focused) {
                XSetWindowBorder(display, old.window, old.border_color);
            }
        }
        XSetWindowBorder(display, win, cfg.focused_border_color);
        XSetInputFocus(display, win, REVERT_TO_POINTER_ROOT, 0);
        XRaiseWindow(display, win);
    }
    *focused = win;
}

fn parse_modifier(name: &str) -> libc::c_uint {
    match name {
        "Mod4" | "Super" => 1 << 6,
        "Shift" => 1 << 0,
        "Control" | "Ctrl" => 1 << 2,
        "Mod1" | "Alt" => 1 << 3,
        _ => 0,
    }
}

fn main() {
    unsafe {
        libc::signal(libc::SIGCHLD, libc::SIG_IGN);
    }

    let mut config_path = PathBuf::from(
        std::env::var("XDG_CONFIG_HOME")
            .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap()))
    );
    config_path.push("aetherwm");
    config_path.push("config.ars");

    let mut active_config = match config::load_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("config error: {:?}; using default config", e);
            config::Config::default()
        }
    };

    println!("AetherWM starting up...");

    // Execute autostart commands from config once at startup
    for cmd in &active_config.autostart {
        if let Err(e) = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .spawn()
        {
            eprintln!("Failed to run autostart '{}': {}", cmd, e);
        }
    }

    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            eprintln!("AetherWM Error: Failed to open X display.");
            std::process::exit(1);
        }

        XSetErrorHandler(Some(startup_err));

        let screen = XDefaultScreen(display);
        let root_window = XRootWindow(display, screen);
        let screen_width = XDisplayWidth(display, screen);
        let screen_height = XDisplayHeight(display, screen);

        XSelectInput(
            display,
            root_window,
            SUBSTRUCTURE_REDIRECT_MASK | SUBSTRUCTURE_NOTIFY_MASK | KEY_PRESS_MASK,
        );
        XSync(display, 0);

        if OTHER_WM.load(Ordering::SeqCst) {
            eprintln!("AetherWM Error: Another window manager is already running on this display.");
            std::process::exit(1);
        }

        XSetErrorHandler(Some(runtime_err));

        // --- EWMH Supporting Window Initialization ---
        let supporting_window = XCreateSimpleWindow(display, root_window, 0, 0, 1, 1, 0, 0, 0);
        let atom_net_supporting_wm_check = XInternAtom(display, b"_NET_SUPPORTING_WM_CHECK\0".as_ptr() as *const c_char, 0);
        let atom_net_wm_name = XInternAtom(display, b"_NET_WM_NAME\0".as_ptr() as *const c_char, 0);
        let utf8_string = XInternAtom(display, b"UTF8_STRING\0".as_ptr() as *const c_char, 0);

        XChangeProperty(display, root_window, atom_net_supporting_wm_check, XA_WINDOW, 32, PROP_MODE_REPLACE, &supporting_window as *const u64 as *const u8, 1);
        XChangeProperty(display, supporting_window, atom_net_supporting_wm_check, XA_WINDOW, 32, PROP_MODE_REPLACE, &supporting_window as *const u64 as *const u8, 1);
        
        let wm_name = b"AetherWM\0";
        XChangeProperty(display, supporting_window, atom_net_wm_name, utf8_string, 8, PROP_MODE_REPLACE, wm_name.as_ptr(), (wm_name.len() - 1) as c_int);

        let lock_modifiers = [0, LOCK_MASK, MOD2_MASK, LOCK_MASK | MOD2_MASK];

        // Grab Initial Keybindings
        for binding in &active_config.bindings {
            let mut mod_mask = 0;
            for m in &binding.modifiers {
                mod_mask |= parse_modifier(m);
            }

            let c_key = std::ffi::CString::new(binding.key.clone()).unwrap();
            let keysym = XStringToKeysym(c_key.as_ptr());
            let keycode = XKeysymToKeycode(display, keysym);

            if keycode != 0 {
                for lock in lock_modifiers {
                    XGrabKey(
                        display,
                        keycode as c_int,
                        mod_mask | lock,
                        root_window,
                        1,
                        GRAB_MODE_ASYNC,
                        GRAB_MODE_ASYNC,
                    );
                }
            }
        }

        // Grab Mouse Buttons for Focus on Root (Mod4 + Click)
        for lock in lock_modifiers {
            XGrabButton(
                display,
                1,
                (1 << 6) | lock,
                root_window,
                1,
                (BUTTON_PRESS_MASK | BUTTON_RELEASE_MASK | POINTER_MOTION_MASK) as libc::c_uint,
                GRAB_MODE_ASYNC,
                GRAB_MODE_ASYNC,
                0,
                0,
            );
        }

        let mut clients: Vec<Client> = Vec::new();
        let mut current_tag: usize = 1;
        let mut focused_window: libc::c_ulong = 0;
        let mut event = XEvent::new();

        let resize_client = |disp: *mut libc::c_void, win: libc::c_ulong, x: i32, y: i32, w: u32, h: u32, bw: u32| {
            let actual_w = w.saturating_sub(2 * bw).max(1);
            let actual_h = h.saturating_sub(2 * bw).max(1);
            XMoveResizeWindow(disp, win, x, y, actual_w, actual_h);
            
            let mut ce = XEvent::new();
            ce.xconfigure.type_ = 22; // ConfigureNotify
            ce.xconfigure.display = disp;
            ce.xconfigure.event = win;
            ce.xconfigure.window = win;
            ce.xconfigure.x = x;
            ce.xconfigure.y = y;
            ce.xconfigure.width = actual_w as i32;
            ce.xconfigure.height = actual_h as i32;
            ce.xconfigure.border_width = bw as i32;
            ce.xconfigure.above = 0;
            ce.xconfigure.override_redirect = 0;
            XSendEvent(disp, win, 0, STRUCTURE_NOTIFY_MASK, &mut ce);
        };

        let tile_layout = |disp: *mut libc::c_void, cl_vec: &mut Vec<Client>, cur_tag: usize, sw: i32, sh: i32, cfg: &config::Config| {
            let mut tag_clients: Vec<&mut Client> = cl_vec
                .iter_mut()
                .filter(|c| c.tag == cur_tag && !c.floating)
                .collect();

            let count = tag_clients.len();
            if count == 0 { return; }

            let og = cfg.outer_gap as i32;
            let ig = cfg.inner_gap as i32;
            let bw = cfg.border_width;

            let avail_w = sw - (og * 2);
            let avail_h = sh - (og * 2);

            if count == 1 {
                let c = &mut *tag_clients[0];
                c.x = og;
                c.y = og;
                c.width = avail_w as u32;
                c.height = avail_h as u32;
                resize_client(disp, c.window, c.x, c.y, c.width, c.height, bw);
            } else {
                let master_w = ((avail_w - ig) as f32 * cfg.master_ratio) as i32;
                let stack_w = avail_w - master_w - ig;

                let m = &mut *tag_clients[0];
                m.x = og;
                m.y = og;
                m.width = master_w as u32;
                m.height = avail_h as u32;
                resize_client(disp, m.window, m.x, m.y, m.width, m.height, bw);

                let stack_count = (count - 1) as i32;
                let total_stack_h = avail_h - (ig * (stack_count - 1));
                let single_stack_h = total_stack_h / stack_count;
                let stack_x = og + master_w + ig;

                for (i, c_ref) in tag_clients.into_iter().skip(1).enumerate() {
                    c_ref.x = stack_x;
                    c_ref.y = og + (i as i32 * (single_stack_h + ig));
                    c_ref.width = stack_w as u32;
                    c_ref.height = single_stack_h as u32;
                    resize_client(disp, c_ref.window, c_ref.x, c_ref.y, c_ref.width, c_ref.height, bw);
                }
            }
            XSync(disp, 0);
        };

        loop {
            XNextEvent(display, &mut event);
            let event_type = event.type_;

            match event_type {
                MAP_REQUEST => {
                    let req = event.xmaprequest;
                    let win = req.window;
                    if clients.iter().any(|c| c.window == win) { continue; }

                    XSelectInput(display, win, CLIENT_EVENT_MASK);

                    let mut win_border_width = active_config.border_width;
                    let mut win_border_color = active_config.border_color;
                    let mut class_hint = XClassHint { res_name: ptr::null_mut(), res_class: ptr::null_mut() };

                    if XGetClassHint(display, win, &mut class_hint) != 0 {
                        if !class_hint.res_class.is_null() {
                            if let Ok(c_str) = CStr::from_ptr(class_hint.res_class).to_str() {
                                let class_name = c_str.to_string();
                                for rule in &active_config.window_rules {
                                    if rule.selector_key == "class" && rule.selector_val == class_name {
                                        if rule.property == "border_width" {
                                            if let Ok(val) = rule.value.parse::<u32>() { win_border_width = val; }
                                        } else if rule.property == "border_color" {
                                            let clean_val = rule.value.strip_prefix("0x").unwrap_or(&rule.value);
                                            if let Ok(val) = u64::from_str_radix(clean_val, 16) { win_border_color = val; }
                                        }
                                    }
                                }
                            }
                        }
                        if !class_hint.res_name.is_null() { XFree(class_hint.res_name as *mut libc::c_void); }
                        if !class_hint.res_class.is_null() { XFree(class_hint.res_class as *mut libc::c_void); }
                    }

                    XSetWindowBorderWidth(display, win, win_border_width);
                    XSetWindowBorder(display, win, win_border_color);

                    clients.push(Client {
                        window: win,
                        tag: current_tag,
                        floating: false,
                        mapped: true,
                        ignore_unmaps: 0,
                        border_color: win_border_color,
                        x: 0, y: 0, width: 800, height: 600,
                    });

                    tile_layout(display, &mut clients, current_tag, screen_width, screen_height, &active_config);
                    XMapWindow(display, win);

                    // Grab button with GRAB_MODE_SYNC so clicks can focus and be replayed cleanly
                    XGrabButton(
                        display, ANY_BUTTON, ANY_MODIFIER, win, 0,
                        BUTTON_PRESS_MASK as libc::c_uint, GRAB_MODE_SYNC, GRAB_MODE_ASYNC, 0, 0,
                    );

                    set_focus(display, &clients, win, &mut focused_window, &active_config);
                }
                CONFIGURE_REQUEST => {
                    let req = event.xconfigurerequest;
                    if clients.iter().any(|c| c.window == req.window) {
                        tile_layout(display, &mut clients, current_tag, screen_width, screen_height, &active_config);
                    } else {
                        let mut changes = XWindowChanges {
                            x: req.x, y: req.y, width: req.width, height: req.height,
                            border_width: req.border_width, sibling: req.above, stack_mode: req.detail,
                        };
                        XConfigureWindow(display, req.window, req.value_mask as u32, &mut changes);
                    }
                }
                UNMAP_NOTIFY | DESTROY_NOTIFY => {
                    let win = if event_type == UNMAP_NOTIFY { event.xunmap.window } else { event.xdestroywindow.window };

                    if let Some(pos) = clients.iter().position(|c| c.window == win) {
                        let remove = if event_type == UNMAP_NOTIFY {
                            if clients[pos].ignore_unmaps > 0 {
                                clients[pos].ignore_unmaps -= 1;
                                false
                            } else { true }
                        } else { true };

                        if remove {
                            clients.remove(pos);
                            if focused_window == win {
                                focused_window = 0;
                                if let Some(c) = clients.iter().rev().find(|c| c.tag == current_tag && c.mapped) {
                                    let w = c.window;
                                    set_focus(display, &clients, w, &mut focused_window, &active_config);
                                }
                            }
                            tile_layout(display, &mut clients, current_tag, screen_width, screen_height, &active_config);
                        }
                    }
                }
                BUTTON_PRESS => {
                    let win = event.xbutton.window;
                    if let Some(c) = clients.iter().find(|c| c.window == win) {
                        let w = c.window;
                        set_focus(display, &clients, w, &mut focused_window, &active_config);
                    }
                    // Replay the pointer event so applications receive the click after focus
                    XAllowEvents(display, REPLAY_POINTER, event.xbutton.time);
                }
                KEY_PRESS => {
                    let keycode = event.xkey.keycode as libc::c_uchar;
                    let raw_state = event.xkey.state;
                    let state = raw_state & !(LOCK_MASK | MOD2_MASK);
                    let pressed_keysym = XKeycodeToKeysym(display, keycode, 0);

                    for binding in &active_config.bindings {
                        let mut target_mod = 0;
                        for m in &binding.modifiers { target_mod |= parse_modifier(m); }

                        let c_key = std::ffi::CString::new(binding.key.clone()).unwrap();
                        let target_keysym = XStringToKeysym(c_key.as_ptr());

                        if pressed_keysym == target_keysym && state == target_mod {
                            if binding.action == "killclient" && focused_window != 0 {
                                let wm_protocols = XInternAtom(display, b"WM_PROTOCOLS\0".as_ptr() as *const c_char, 0);
                                let wm_delete_atom = XInternAtom(display, b"WM_DELETE_WINDOW\0".as_ptr() as *const c_char, 0);
                                let mut cm_event = XEvent::new();
                                cm_event.type_ = 33; // ClientMessage
                                cm_event.xclient.window = focused_window;
                                cm_event.xclient.message_type = wm_protocols;
                                cm_event.xclient.format = 32;
                                cm_event.xclient.data[0] = wm_delete_atom;
                                cm_event.xclient.data[1] = 0;
                                XSendEvent(display, focused_window, 0, 0, &mut cm_event);
                            } else if binding.action == "reload_config" {
                                match config::load_config(&config_path) {
                                    Ok(new_cfg) => {
                                        for old_binding in &active_config.bindings {
                                            let mut mod_mask = 0;
                                            for m in &old_binding.modifiers { mod_mask |= parse_modifier(m); }
                                            let c_key = std::ffi::CString::new(old_binding.key.clone()).unwrap();
                                            let keysym = XStringToKeysym(c_key.as_ptr());
                                            let keycode = XKeysymToKeycode(display, keysym);
                                            if keycode != 0 {
                                                for lock in lock_modifiers {
                                                    XUngrabKey(display, keycode as c_int, mod_mask | lock, root_window);
                                                }
                                            }
                                        }

                                        active_config = new_cfg;

                                        for new_binding in &active_config.bindings {
                                            let mut mod_mask = 0;
                                            for m in &new_binding.modifiers { mod_mask |= parse_modifier(m); }
                                            let c_key = std::ffi::CString::new(new_binding.key.clone()).unwrap();
                                            let keysym = XStringToKeysym(c_key.as_ptr());
                                            let keycode = XKeysymToKeycode(display, keysym);
                                            if keycode != 0 {
                                                for lock in lock_modifiers {
                                                    XGrabKey(display, keycode as c_int, mod_mask | lock, root_window, 1, GRAB_MODE_ASYNC, GRAB_MODE_ASYNC);
                                                }
                                            }
                                        }

                                        for c in &clients {
                                            XSetWindowBorderWidth(display, c.window, active_config.border_width);
                                            if c.window == focused_window {
                                                XSetWindowBorder(display, c.window, active_config.focused_border_color);
                                            } else {
                                                XSetWindowBorder(display, c.window, active_config.border_color);
                                            }
                                        }

                                        tile_layout(display, &mut clients, current_tag, screen_width, screen_height, &active_config);
                                        println!("AetherWM: Config reloaded successfully!");
                                    }
                                    Err(e) => eprintln!("config reload error: {:?}", e),
                                }
                            } else if binding.action.starts_with("view_tag_") {
                                if let Ok(t) = binding.action.strip_prefix("view_tag_").unwrap().parse::<usize>() {
                                    current_tag = t;
                                    focused_window = 0;
                                    for c in &mut clients {
                                        if c.tag == current_tag {
                                            if !c.mapped {
                                                XMapWindow(display, c.window);
                                                c.mapped = true;
                                            }
                                        } else {
                                            if c.mapped {
                                                c.ignore_unmaps += 1;
                                                XUnmapWindow(display, c.window);
                                                c.mapped = false;
                                            }
                                        }
                                    }
                                    tile_layout(display, &mut clients, current_tag, screen_width, screen_height, &active_config);
                                    if let Some(c) = clients.iter().rev().find(|c| c.tag == current_tag && c.mapped) {
                                        let w = c.window;
                                        set_focus(display, &clients, w, &mut focused_window, &active_config);
                                    }
                                }
                            } else if binding.action.starts_with("exec ") {
                                let cmd = binding.action.strip_prefix("exec ").unwrap();
                                if let Err(e) = std::process::Command::new("sh")
                                    .arg("-c")
                                    .arg(cmd)
                                    .spawn()
                                {
                                    eprintln!("Failed to exec '{}': {}", cmd, e);
                                }
                            }
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
