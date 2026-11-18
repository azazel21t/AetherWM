//! Small XRender compositor for shadows and per-window opacity.
//!
//! AetherWM remains a window manager when another compositor owns the X11
//! compositing selection. In that case this module stays inactive.

use crate::comp_config::CompositorConfig;
use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long, c_uint, c_ulong};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

type Display = libc::c_void;
type Picture = c_ulong;
type Pixmap = c_ulong;
type Region = c_ulong;

#[repr(C)]
struct Visual {
    _private: [u8; 0],
}
#[repr(C)]
struct XVisualInfo {
    visual: *mut Visual,
    visualid: c_ulong,
    screen: c_int,
    depth: c_int,
    class: c_int,
    red_mask: c_ulong,
    green_mask: c_ulong,
    blue_mask: c_ulong,
    colormap_size: c_int,
    bits_per_rgb: c_int,
}
#[repr(C)]
struct PictFormat {
    _private: [u8; 0],
}

#[repr(C)]
struct XWindowAttributes {
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border_width: c_int,
    depth: c_int,
    visual: *mut Visual,
    root: c_ulong,
    class: c_int,
    bit_gravity: c_int,
    win_gravity: c_int,
    backing_store: c_int,
    backing_planes: c_ulong,
    backing_pixel: c_ulong,
    save_under: c_int,
    colormap: c_ulong,
    map_installed: c_int,
    map_state: c_int,
    all_event_masks: c_long,
    your_event_mask: c_long,
    do_not_propagate_mask: c_long,
    override_redirect: c_int,
    screen: *mut libc::c_void,
}

#[repr(C)]
struct RenderColor {
    red: u16,
    green: u16,
    blue: u16,
    alpha: u16,
}
#[repr(C)]
struct XRectangle {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

#[link(name = "X11")]
unsafe extern "C" {
    fn XInternAtom(dpy: *mut Display, name: *const c_char, only_if_exists: c_int) -> c_ulong;
    fn XGetSelectionOwner(dpy: *mut Display, selection: c_ulong) -> c_ulong;
    fn XSetSelectionOwner(dpy: *mut Display, selection: c_ulong, owner: c_ulong, time: c_ulong);
    fn XQueryTree(
        dpy: *mut Display,
        w: c_ulong,
        root: *mut c_ulong,
        parent: *mut c_ulong,
        children: *mut *mut c_ulong,
        nchildren: *mut c_uint,
    ) -> c_int;
    fn XGetWindowAttributes(dpy: *mut Display, w: c_ulong, attrs: *mut XWindowAttributes) -> c_int;
    fn XGetWindowProperty(
        dpy: *mut Display,
        w: c_ulong,
        property: c_ulong,
        offset: c_long,
        length: c_long,
        delete: c_int,
        req_type: c_ulong,
        actual_type: *mut c_ulong,
        actual_format: *mut c_int,
        nitems: *mut c_ulong,
        after: *mut c_ulong,
        data: *mut *mut u8,
    ) -> c_int;
    fn XFree(data: *mut libc::c_void) -> c_int;
    fn XFreePixmap(dpy: *mut Display, pixmap: Pixmap) -> c_int;
    fn XSync(dpy: *mut Display, discard: c_int) -> c_int;
    fn XDisplayWidth(dpy: *mut Display, screen: c_int) -> c_int;
    fn XDisplayHeight(dpy: *mut Display, screen: c_int) -> c_int;
    fn XMoveResizeWindow(
        dpy: *mut Display,
        w: c_ulong,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    fn XCreatePixmap(
        dpy: *mut Display,
        drawable: c_ulong,
        width: c_uint,
        height: c_uint,
        depth: c_uint,
    ) -> Pixmap;
    fn XGetVisualInfo(
        dpy: *mut Display,
        mask: c_long,
        template: *mut XVisualInfo,
        count: *mut c_int,
    ) -> *mut XVisualInfo;
}

#[link(name = "Xcomposite")]
unsafe extern "C" {
    fn XCompositeQueryExtension(
        dpy: *mut Display,
        event_base: *mut c_int,
        error_base: *mut c_int,
    ) -> c_int;
    fn XCompositeRedirectSubwindows(dpy: *mut Display, w: c_ulong, update: c_int);
    fn XCompositeUnredirectSubwindows(dpy: *mut Display, w: c_ulong, update: c_int);
    fn XCompositeGetOverlayWindow(dpy: *mut Display, w: c_ulong) -> c_ulong;
    fn XCompositeReleaseOverlayWindow(dpy: *mut Display, w: c_ulong);
    fn XCompositeNameWindowPixmap(dpy: *mut Display, w: c_ulong) -> Pixmap;
}

#[link(name = "Xdamage")]
unsafe extern "C" {
    fn XDamageQueryExtension(
        dpy: *mut Display,
        event_base: *mut c_int,
        error_base: *mut c_int,
    ) -> c_int;
    fn XDamageCreate(dpy: *mut Display, drawable: c_ulong, level: c_int) -> c_ulong;
    fn XDamageDestroy(dpy: *mut Display, damage: c_ulong);
    fn XDamageSubtract(dpy: *mut Display, damage: c_ulong, repair: c_ulong, parts: c_ulong);
}

#[link(name = "Xfixes")]
unsafe extern "C" {
    fn XFixesQueryExtension(
        dpy: *mut Display,
        event_base: *mut c_int,
        error_base: *mut c_int,
    ) -> c_int;
    fn XFixesCreateRegion(dpy: *mut Display, rects: *const libc::c_void, nrects: c_int) -> Region;
    fn XFixesCreateRegionFromWindow(dpy: *mut Display, window: c_ulong, shape: c_int) -> Region;
    fn XFixesDestroyRegion(dpy: *mut Display, region: Region);
    fn XFixesSetWindowShapeRegion(
        dpy: *mut Display,
        w: c_ulong,
        shape: c_int,
        x: c_int,
        y: c_int,
        region: Region,
    );
    fn XFixesSetPictureClipRegion(
        dpy: *mut Display,
        picture: Picture,
        x: c_int,
        y: c_int,
        region: Region,
    );
}

#[link(name = "Xrender")]
unsafe extern "C" {
    fn XRenderQueryExtension(
        dpy: *mut Display,
        event_base: *mut c_int,
        error_base: *mut c_int,
    ) -> c_int;
    fn XRenderFindVisualFormat(dpy: *mut Display, visual: *mut Visual) -> *mut PictFormat;
    fn XRenderCreatePicture(
        dpy: *mut Display,
        drawable: c_ulong,
        format: *mut PictFormat,
        mask: c_ulong,
        attrs: *const libc::c_void,
    ) -> Picture;
    fn XRenderFreePicture(dpy: *mut Display, picture: Picture);
    fn XRenderComposite(
        dpy: *mut Display,
        op: c_int,
        src: Picture,
        mask: Picture,
        dst: Picture,
        sx: c_int,
        sy: c_int,
        mx: c_int,
        my: c_int,
        dx: c_int,
        dy: c_int,
        width: c_uint,
        height: c_uint,
    );
    fn XRenderCreateSolidFill(dpy: *mut Display, color: *const RenderColor) -> Picture;
    fn XRenderFillRectangle(
        dpy: *mut Display,
        op: c_int,
        dst: Picture,
        color: *const RenderColor,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    );
    fn XRenderSetPictureFilter(
        dpy: *mut Display,
        picture: Picture,
        filter: *const c_char,
        params: *mut c_int,
        nparams: c_int,
    );
}

const PICT_OP_SRC: c_int = 1;
const PICT_OP_OVER: c_int = 3;
const SHAPE_INPUT: c_int = 2;
const SHAPE_BOUNDING: c_int = 0;
const XA_CARDINAL: c_ulong = 6;
const VISUAL_SCREEN_MASK: c_long = 1 << 1;
const VISUAL_DEPTH_MASK: c_long = 1 << 2;
const VISUAL_CLASS_MASK: c_long = 1 << 3;
const TRUE_COLOR: c_int = 4;

static STARTUP_X_ERROR: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn compositor_startup_error(
    _display: *mut Display,
    _event: *mut crate::XErrorEvent,
) -> c_int {
    STARTUP_X_ERROR.store(true, Ordering::SeqCst);
    0
}

/// Find a Render format compatible with the root drawable. The root visual is
/// authoritative; the visual list fallback is restricted to the same screen
/// and depth so it cannot make XRenderCreatePicture issue BadMatch.
unsafe fn root_render_format(
    display: *mut Display,
    root: c_ulong,
    screen: c_int,
) -> Option<(*mut PictFormat, c_int)> {
    let mut attrs: XWindowAttributes = unsafe { std::mem::zeroed() };
    if unsafe { XGetWindowAttributes(display, root, &mut attrs) } == 0 || attrs.depth <= 0 {
        return None;
    }
    let format = unsafe { XRenderFindVisualFormat(display, attrs.visual) };
    if !format.is_null() {
        return Some((format, attrs.depth));
    }

    let mut template: XVisualInfo = unsafe { std::mem::zeroed() };
    template.screen = screen;
    template.depth = attrs.depth;
    template.class = TRUE_COLOR;
    let mut count = 0;
    let infos = unsafe {
        XGetVisualInfo(
            display,
            VISUAL_SCREEN_MASK | VISUAL_DEPTH_MASK | VISUAL_CLASS_MASK,
            &mut template,
            &mut count,
        )
    };
    if infos.is_null() {
        return None;
    }
    let mut found = ptr::null_mut();
    for index in 0..count as usize {
        let info = unsafe { &*infos.add(index) };
        let candidate = unsafe { XRenderFindVisualFormat(display, info.visual) };
        if !candidate.is_null() {
            found = candidate;
            break;
        }
    }
    unsafe { XFree(infos.cast()) };
    if found.is_null() {
        None
    } else {
        Some((found, attrs.depth))
    }
}

pub struct Compositor {
    display: *mut Display,
    root: c_ulong,
    overlay: c_ulong,
    root_picture: Picture,
    target_picture: Picture,
    scratch_pixmap: Pixmap,
    scratch_picture: Picture,
    opacity_atom: c_ulong,
    selection: c_ulong,
    shadow_opacity: u16,
    default_opacity: u32,
    screen_width: c_uint,
    screen_height: c_uint,
    damage_event_base: c_int,
    damages: HashMap<c_ulong, c_ulong>,
    opacity_overrides: HashMap<c_ulong, u32>,
    blur_enabled: bool,
    shadow_enabled: bool,
    animations_enabled: bool,
    animation_duration_ms: u32,
    active: Cell<bool>,
}

impl Compositor {
    /// Claim the screen compositor selection and initialize XComposite/XRender.
    pub unsafe fn start(
        display: *mut Display,
        root: c_ulong,
        screen: c_int,
        owner: c_ulong,
        config: &CompositorConfig,
    ) -> Option<Self> {
        // X errors are asynchronous: a nonzero resource ID does not prove the
        // server accepted its creation. Trap startup errors and synchronize
        // before returning so unsupported visuals/effects disable composition
        // without leaving the whole window manager in a bad state.
        unsafe { XSync(display, 0) };
        STARTUP_X_ERROR.store(false, Ordering::SeqCst);
        let previous_handler = unsafe { crate::XSetErrorHandler(Some(compositor_startup_error)) };
        let result = unsafe { Self::start_inner(display, root, screen, owner, config) };
        unsafe { XSync(display, 0) };
        let had_x_error = STARTUP_X_ERROR.swap(false, Ordering::SeqCst);
        unsafe { crate::XSetErrorHandler(previous_handler) };
        if had_x_error {
            drop(result);
            eprintln!(
                "AetherWM: X server rejected compositor initialization; continuing without compositor"
            );
            None
        } else {
            result
        }
    }

    unsafe fn start_inner(
        display: *mut Display,
        root: c_ulong,
        screen: c_int,
        owner: c_ulong,
        config: &CompositorConfig,
    ) -> Option<Self> {
        let (mut event_base, mut error_base) = (0, 0);
        if unsafe { XCompositeQueryExtension(display, &mut event_base, &mut error_base) } == 0
            || unsafe { XFixesQueryExtension(display, &mut event_base, &mut error_base) } == 0
            || unsafe { XRenderQueryExtension(display, &mut event_base, &mut error_base) } == 0
        {
            return None;
        }
        let mut damage_event_base = 0;
        let mut damage_error_base = 0;
        if unsafe { XDamageQueryExtension(display, &mut damage_event_base, &mut damage_error_base) }
            == 0
        {
            return None;
        }
        let (root_format, root_depth) = (unsafe { root_render_format(display, root, screen) })?;

        let selection_name = CString::new(format!("_NET_WM_CM_S{screen}")).ok()?;
        let selection = unsafe { XInternAtom(display, selection_name.as_ptr(), 0) };
        if unsafe { XGetSelectionOwner(display, selection) } != 0 {
            return None;
        }
        unsafe {
            XSetSelectionOwner(display, selection, owner, 0);
            XSync(display, 0);
        }
        if unsafe { XGetSelectionOwner(display, selection) } != owner {
            return None;
        }

        unsafe {
            XCompositeRedirectSubwindows(display, root, 0);
        }
        let overlay = unsafe { XCompositeGetOverlayWindow(display, root) };
        if overlay == 0 {
            unsafe {
                XCompositeUnredirectSubwindows(display, root, 0);
                XSetSelectionOwner(display, selection, 0, 0);
                XSync(display, 0);
            }
            return None;
        }
        let empty_region = unsafe { XFixesCreateRegion(display, ptr::null(), 0) };
        unsafe {
            XFixesSetWindowShapeRegion(display, overlay, SHAPE_INPUT, 0, 0, empty_region);
            XFixesDestroyRegion(display, empty_region);
        }
        let root_picture =
            unsafe { XRenderCreatePicture(display, root, root_format, 0, ptr::null()) };
        let mut overlay_attrs: XWindowAttributes = unsafe { std::mem::zeroed() };
        let overlay_ok = unsafe { XGetWindowAttributes(display, overlay, &mut overlay_attrs) } != 0
            && overlay_attrs.depth == root_depth;
        let target_picture = if overlay_ok {
            unsafe { XRenderCreatePicture(display, overlay, root_format, 0, ptr::null()) }
        } else {
            0
        };
        let scratch_pixmap = unsafe {
            XCreatePixmap(
                display,
                root,
                XDisplayWidth(display, screen).max(1) as c_uint,
                XDisplayHeight(display, screen).max(1) as c_uint,
                root_depth as c_uint,
            )
        };
        let scratch_picture = if scratch_pixmap != 0 {
            unsafe { XRenderCreatePicture(display, scratch_pixmap, root_format, 0, ptr::null()) }
        } else {
            0
        };
        unsafe { XSync(display, 0) };
        if root_picture == 0 || target_picture == 0 || scratch_picture == 0 {
            unsafe {
                if root_picture != 0 {
                    XRenderFreePicture(display, root_picture);
                }
                if target_picture != 0 {
                    XRenderFreePicture(display, target_picture);
                }
                if scratch_picture != 0 {
                    XRenderFreePicture(display, scratch_picture);
                }
                if scratch_pixmap != 0 {
                    XFreePixmap(display, scratch_pixmap);
                }
                XCompositeReleaseOverlayWindow(display, root);
                XCompositeUnredirectSubwindows(display, root, 0);
                XSetSelectionOwner(display, selection, 0, 0);
                XSync(display, 0);
            }
            return None;
        };
        let opacity_name = CString::new("_NET_WM_WINDOW_OPACITY").ok()?;
        let opacity_atom = unsafe { XInternAtom(display, opacity_name.as_ptr(), 0) };
        Some(Self {
            display,
            root,
            overlay,
            root_picture,
            target_picture,
            scratch_pixmap,
            scratch_picture,
            opacity_atom,
            selection,
            shadow_opacity: ((config.shadow_opacity.min(70) as u64 * 65_535) / 100) as u16,
            default_opacity: ((config.opacity.clamp(20, 100) as u64 * u32::MAX as u64) / 100)
                as u32,
            screen_width: unsafe { XDisplayWidth(display, screen) }.max(1) as c_uint,
            screen_height: unsafe { XDisplayHeight(display, screen) }.max(1) as c_uint,
            damage_event_base,
            damages: HashMap::new(),
            opacity_overrides: HashMap::new(),
            blur_enabled: config.blur,
            shadow_enabled: config.shadow,
            animations_enabled: config.animations,
            animation_duration_ms: config.animation_duration_ms,
            active: Cell::new(true),
        })
    }

    pub fn update_effects(&mut self, config: &CompositorConfig) {
        self.default_opacity =
            ((config.opacity.clamp(20, 100) as u64 * u32::MAX as u64) / 100) as u32;
        self.shadow_opacity = ((config.shadow_opacity.min(70) as u64 * 65_535) / 100) as u16;
        self.blur_enabled = config.blur;
        self.shadow_enabled = config.shadow;
        self.animations_enabled = config.animations;
        self.animation_duration_ms = config.animation_duration_ms;
    }

    pub fn is_active(&self) -> bool {
        self.active.get()
    }

    pub fn animate_geometry(
        &self,
        window: c_ulong,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) {
        if !self.animations_enabled || self.animation_duration_ms == 0 {
            return;
        }
        let mut attrs: XWindowAttributes = unsafe { std::mem::zeroed() };
        if unsafe { XGetWindowAttributes(self.display, window, &mut attrs) } == 0
            || attrs.map_state != 2
        {
            return;
        }
        let (sx, sy, sw, sh) = (attrs.x, attrs.y, attrs.width.max(1), attrs.height.max(1));
        if sx == x && sy == y && sw as c_uint == width.max(1) && sh as c_uint == height.max(1) {
            return;
        }
        let frames = (self.animation_duration_ms.div_ceil(10)).clamp(1, 30);
        let frame_time = (self.animation_duration_ms / frames).max(1);
        for frame in 1..=frames {
            let t = frame as f64 / frames as f64;
            let eased = t * t * (3.0 - 2.0 * t);
            let lerp = |a: f64, b: f64| (a + (b - a) * eased).round() as c_int;
            unsafe {
                XMoveResizeWindow(
                    self.display,
                    window,
                    lerp(sx as f64, x as f64),
                    lerp(sy as f64, y as f64),
                    lerp(sw as f64, width.max(1) as f64).max(1) as c_uint,
                    lerp(sh as f64, height.max(1) as f64).max(1) as c_uint,
                );
            }
            self.repaint();
            std::thread::sleep(std::time::Duration::from_millis(frame_time as u64));
        }
    }

    pub fn animate_close(&mut self, window: c_ulong) -> bool {
        if !self.animations_enabled || self.animation_duration_ms == 0 || !self.is_viewable(window)
        {
            return false;
        }
        let start = self.read_opacity(window);
        let frames = (self.animation_duration_ms.div_ceil(10)).clamp(1, 30);
        let frame_time = (self.animation_duration_ms / frames).max(1);
        for frame in 1..=frames {
            let remaining = frames - frame;
            self.opacity_overrides.insert(
                window,
                ((start as u64 * remaining as u64) / frames as u64) as u32,
            );
            self.repaint();
            std::thread::sleep(std::time::Duration::from_millis(frame_time as u64));
        }
        true
    }

    pub fn is_viewable(&self, window: c_ulong) -> bool {
        let mut attrs: XWindowAttributes = unsafe { std::mem::zeroed() };
        unsafe {
            XGetWindowAttributes(self.display, window, &mut attrs) != 0 && attrs.map_state == 2
        }
    }

    pub fn cancel_close(&mut self, window: c_ulong) {
        self.opacity_overrides.remove(&window);
        self.repaint();
    }

    pub fn watch_window(&mut self, window: c_ulong) {
        if self.damages.contains_key(&window) {
            return;
        }
        let damage = unsafe { XDamageCreate(self.display, window, 3) };
        if damage != 0 {
            self.damages.insert(window, damage);
        }
    }

    pub fn forget_window(&mut self, window: c_ulong) {
        if let Some(damage) = self.damages.remove(&window) {
            unsafe {
                XDamageDestroy(self.display, damage);
            }
        }
        self.opacity_overrides.remove(&window);
    }

    pub fn finish_unmap(&mut self, window: c_ulong) {
        self.opacity_overrides.remove(&window);
    }

    pub fn is_damage_event(&self, event_type: c_int) -> bool {
        event_type == self.damage_event_base
    }

    pub fn is_opacity_property(&self, atom: c_ulong) -> bool {
        atom == self.opacity_atom
    }

    pub fn repaint(&self) {
        if !self.active.get() {
            return;
        }
        STARTUP_X_ERROR.store(false, Ordering::SeqCst);
        let previous_handler = unsafe { crate::XSetErrorHandler(Some(compositor_startup_error)) };
        self.repaint_inner();
        unsafe { XSync(self.display, 0) };
        let had_x_error = STARTUP_X_ERROR.swap(false, Ordering::SeqCst);
        unsafe { crate::XSetErrorHandler(previous_handler) };
        if had_x_error {
            eprintln!("AetherWM: X server rejected compositor drawing; disabling compositor");
            unsafe {
                XCompositeUnredirectSubwindows(self.display, self.root, 0);
                XCompositeReleaseOverlayWindow(self.display, self.root);
                XSetSelectionOwner(self.display, self.selection, 0, 0);
                XSync(self.display, 0);
            }
            self.active.set(false);
        }
    }

    fn repaint_inner(&self) {
        unsafe {
            // Restore the wallpaper, then paint mapped children in bottom-to-top order.
            XRenderComposite(
                self.display,
                PICT_OP_SRC,
                self.root_picture,
                0,
                self.target_picture,
                0,
                0,
                0,
                0,
                0,
                0,
                self.screen_width,
                self.screen_height,
            );
            let mut tree_root = 0;
            let mut parent = 0;
            let mut children: *mut c_ulong = ptr::null_mut();
            let mut count = 0;
            if XQueryTree(
                self.display,
                self.root,
                &mut tree_root,
                &mut parent,
                &mut children,
                &mut count,
            ) == 0
            {
                if !children.is_null() {
                    XFree(children.cast());
                }
                return;
            }

            if !children.is_null() {
                for &window in std::slice::from_raw_parts(children, count as usize) {
                    if window == self.overlay {
                        continue;
                    }
                    let mut attrs: XWindowAttributes = std::mem::zeroed();
                    if XGetWindowAttributes(self.display, window, &mut attrs) == 0
                        || attrs.map_state != 2
                        || attrs.width <= 0
                        || attrs.height <= 0
                    {
                        continue;
                    }
                    let pixmap = XCompositeNameWindowPixmap(self.display, window);
                    if pixmap == 0 {
                        continue;
                    }
                    let format = XRenderFindVisualFormat(self.display, attrs.visual);
                    if format.is_null() {
                        XFreePixmap(self.display, pixmap);
                        continue;
                    }
                    let picture =
                        XRenderCreatePicture(self.display, pixmap, format, 0, ptr::null());
                    if picture == 0 {
                        XFreePixmap(self.display, pixmap);
                        continue;
                    }
                    let shape = XFixesCreateRegionFromWindow(self.display, window, SHAPE_BOUNDING);
                    // This picture is backed by the window's named pixmap, whose
                    // origin is (0, 0). The shape region is also window-local;
                    // offsetting it by the window's root position clips the
                    // source away for every window not located at the top-left.
                    XFixesSetPictureClipRegion(self.display, picture, 0, 0, shape);

                    // A translucent drop shadow. The window's X Shape mask keeps
                    // rounded corners transparent while the shadow remains visible.
                    if self.shadow_enabled {
                        for (padding, alpha_scale) in [(10, 10u32), (6, 18), (2, 28)] {
                            let sw = (attrs.width + padding * 2).clamp(1, u16::MAX as c_int) as u32;
                            let sh =
                                (attrs.height + padding * 2).clamp(1, u16::MAX as c_int) as u32;
                            let radius = (12 + padding as u32).min(sw.min(sh) / 2);
                            let rects = rounded_rectangles(sw, sh, radius);
                            let region = XFixesCreateRegion(
                                self.display,
                                rects.as_ptr() as *const libc::c_void,
                                rects.len() as c_int,
                            );
                            XFixesSetPictureClipRegion(
                                self.display,
                                self.target_picture,
                                attrs.x - padding,
                                attrs.y - padding + 3,
                                region,
                            );
                            let alpha = ((self.shadow_opacity as u32 * alpha_scale) / 100)
                                .min(u16::MAX as u32)
                                as u16;
                            let shadow = RenderColor {
                                red: 0,
                                green: 0,
                                blue: 0,
                                alpha,
                            };
                            XRenderFillRectangle(
                                self.display,
                                PICT_OP_OVER,
                                self.target_picture,
                                &shadow,
                                attrs.x - padding,
                                attrs.y - padding + 3,
                                sw,
                                sh,
                            );
                            XFixesSetPictureClipRegion(self.display, self.target_picture, 0, 0, 0);
                            XFixesDestroyRegion(self.display, region);
                        }
                    }

                    let opacity = self
                        .opacity_overrides
                        .get(&window)
                        .copied()
                        .unwrap_or_else(|| self.read_opacity(window));
                    if self.blur_enabled && opacity < u32::MAX {
                        // Blur a snapshot of the scene beneath translucent windows.
                        XRenderComposite(
                            self.display,
                            PICT_OP_SRC,
                            self.target_picture,
                            0,
                            self.scratch_picture,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            self.screen_width,
                            self.screen_height,
                        );
                        let filter = CString::new("convolution").unwrap();
                        let one_ninth = (65_536 / 9) as c_int;
                        let mut kernel = vec![3 * 65_536, 3 * 65_536, 0, 0];
                        kernel.extend([one_ninth; 9]);
                        XRenderSetPictureFilter(
                            self.display,
                            self.scratch_picture,
                            filter.as_ptr(),
                            kernel.as_mut_ptr(),
                            kernel.len() as c_int,
                        );
                        XRenderComposite(
                            self.display,
                            PICT_OP_SRC,
                            self.scratch_picture,
                            0,
                            self.target_picture,
                            attrs.x,
                            attrs.y,
                            0,
                            0,
                            attrs.x,
                            attrs.y,
                            attrs.width as c_uint,
                            attrs.height as c_uint,
                        );
                    }
                    let alpha = RenderColor {
                        red: 0,
                        green: 0,
                        blue: 0,
                        alpha: ((opacity as u64 * 65_535) / u32::MAX as u64) as u16,
                    };
                    let mask = XRenderCreateSolidFill(self.display, &alpha);
                    XRenderComposite(
                        self.display,
                        PICT_OP_OVER,
                        picture,
                        mask,
                        self.target_picture,
                        0,
                        0,
                        0,
                        0,
                        attrs.x,
                        attrs.y,
                        attrs.width as c_uint,
                        attrs.height as c_uint,
                    );
                    XRenderFreePicture(self.display, mask);
                    XFixesDestroyRegion(self.display, shape);
                    XRenderFreePicture(self.display, picture);
                    XFreePixmap(self.display, pixmap);
                }
            }
            if !children.is_null() {
                XFree(children as *mut libc::c_void);
            }
            for damage in self.damages.values() {
                XDamageSubtract(self.display, *damage, 0, 0);
            }
        }
    }

    fn read_opacity(&self, window: c_ulong) -> u32 {
        unsafe {
            let (mut actual, mut format, mut nitems, mut after) = (0, 0, 0, 0);
            let mut data: *mut u8 = ptr::null_mut();
            if XGetWindowProperty(
                self.display,
                window,
                self.opacity_atom,
                0,
                1,
                0,
                XA_CARDINAL,
                &mut actual,
                &mut format,
                &mut nitems,
                &mut after,
                &mut data,
            ) == 0
                && !data.is_null()
                && nitems > 0
            {
                let opacity = *(data as *const c_ulong) as u32;
                XFree(data as *mut libc::c_void);
                opacity
            } else {
                if !data.is_null() {
                    XFree(data as *mut libc::c_void);
                }
                self.default_opacity
            }
        }
    }
}

fn rounded_rectangles(width: u32, height: u32, radius: u32) -> Vec<XRectangle> {
    let mut rectangles = Vec::with_capacity((radius * 2 + 1) as usize);
    let radius = radius.min(width.min(height) / 2);
    if radius == 0 {
        rectangles.push(XRectangle {
            x: 0,
            y: 0,
            width: width as u16,
            height: height as u16,
        });
        return rectangles;
    }
    for y in 0..radius {
        let r = radius as f64;
        let distance = r - y as f64;
        let inset = (r - (r * r - distance * distance).max(0.0).sqrt()).ceil() as u32;
        let span = width.saturating_sub(inset * 2).max(1);
        rectangles.push(XRectangle {
            x: inset as i16,
            y: y as i16,
            width: span as u16,
            height: 1,
        });
        rectangles.push(XRectangle {
            x: inset as i16,
            y: (height - 1 - y) as i16,
            width: span as u16,
            height: 1,
        });
    }
    if height > radius * 2 {
        rectangles.push(XRectangle {
            x: 0,
            y: radius as i16,
            width: width as u16,
            height: (height - radius * 2) as u16,
        });
    }
    rectangles
}

impl Drop for Compositor {
    fn drop(&mut self) {
        unsafe {
            XRenderFreePicture(self.display, self.root_picture);
            XRenderFreePicture(self.display, self.target_picture);
            XRenderFreePicture(self.display, self.scratch_picture);
            XFreePixmap(self.display, self.scratch_pixmap);
            for damage in self.damages.values() {
                XDamageDestroy(self.display, *damage);
            }
            if self.active.get() {
                XCompositeUnredirectSubwindows(self.display, self.root, 0);
                XCompositeReleaseOverlayWindow(self.display, self.root);
                XSetSelectionOwner(self.display, self.selection, 0, 0);
            }
            XSync(self.display, 0);
        }
    }
}
