//! Wayland compositor with native TTY and optional nested development backends.

use std::os::unix::io::OwnedFd;
use std::path::PathBuf;
use std::process::{Child, Command};
#[cfg(feature = "wayland-nested")]
use std::sync::Arc;
#[cfg(feature = "wayland-nested")]
use std::time::{Duration, Instant};

#[cfg(feature = "wayland-nested")]
use smithay::backend::egl::context::GlAttributes;
#[cfg(feature = "wayland-nested")]
use smithay::backend::input::{
    AbsolutePositionEvent, ButtonState, Event, InputEvent, KeyboardKeyEvent, PointerButtonEvent,
};
#[cfg(feature = "wayland-nested")]
use smithay::backend::renderer::Renderer;
#[cfg(feature = "wayland-nested")]
use smithay::backend::renderer::element::surface::{
    WaylandSurfaceRenderElement, render_elements_from_surface_tree,
};
#[cfg(feature = "wayland-nested")]
use smithay::backend::renderer::utils::draw_render_elements;
use smithay::backend::renderer::utils::on_commit_buffer_handler;
use smithay::backend::renderer::{Color32F, Frame};
#[cfg(feature = "wayland-nested")]
use smithay::backend::renderer::{element::Kind, gles::GlesRenderer};
#[cfg(feature = "wayland-nested")]
use smithay::backend::winit::{self, WinitEvent};
#[cfg(feature = "wayland-nested")]
use smithay::input::keyboard::{FilterResult, xkb};
#[cfg(feature = "wayland-nested")]
use smithay::input::pointer::{ButtonEvent, MotionEvent};
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::output::Output;
#[cfg(feature = "wayland-nested")]
use smithay::output::{Mode, PhysicalProperties, Scale, Subpixel};
use smithay::reexports::wayland_protocols;
use smithay::reexports::wayland_server::protocol::wl_seat;
use smithay::reexports::wayland_server::protocol::wl_surface::{self, WlSurface};
use smithay::reexports::wayland_server::{
    Client,
    backend::{ClientData, ClientId, DisconnectReason},
    protocol::wl_buffer,
};
#[cfg(feature = "wayland-nested")]
use smithay::reexports::wayland_server::{Display, ListeningSocket};
#[cfg(feature = "wayland-nested")]
use smithay::reexports::winit as smithay_winit;
use smithay::utils::Rectangle;
#[cfg(feature = "wayland-nested")]
use smithay::utils::Transform;
use smithay::utils::{Logical, Point, Serial, Size};
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::compositor::{
    CompositorClientState, CompositorHandler, CompositorState, SurfaceAttributes, TraversalAction,
    with_surface_tree_downward,
};
use smithay::wayland::output::OutputHandler;
use smithay::wayland::selection::SelectionHandler;
use smithay::wayland::selection::data_device::{
    ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
};
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
};
use smithay::wayland::shm::{ShmHandler, ShmState};
use smithay::{
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
};

#[cfg(feature = "wayland-nested")]
use smithay_winit::platform::pump_events::PumpStatus;
#[cfg(feature = "wayland-nested")]
use smithay_winit::{dpi::LogicalSize, window::Window};
use wayland_protocols::xdg::shell::server::xdg_toplevel;

#[cfg(feature = "wayland-tty")]
#[path = "wayland_tty.rs"]
mod tty;

#[path = "config.rs"]
#[allow(dead_code)]
mod config;

struct Compositor {
    compositor_state: CompositorState,
    xdg_shell_state: XdgShellState,
    shm_state: ShmState,
    seat_state: SeatState<Self>,
    data_device_state: DataDeviceState,
    seat: Seat<Self>,
    config: config::Config,
    output: Output,
    output_size: (i32, i32),
    layout_dirty: bool,
    needs_redraw: bool,
    pointer_location: Point<f64, Logical>,
    serial: u32,
    running: bool,
    current_view: (i32, i32),
    surface_views: Vec<(WlSurface, (i32, i32))>,
    floating_surfaces: Vec<WlSurface>,
    floating_rects: Vec<(WlSurface, (i32, i32, i32, i32))>,
    configured_sizes: Vec<(WlSurface, (i32, i32))>,
    focused_index: usize,
    overview: bool,
    drag: Option<DragState>,
    pan_drag: Option<PanDragState>,
    pan_offset: (f64, f64),
    children: Vec<LaunchedCommand>,
}

pub(crate) struct LaunchedCommand {
    command: String,
    child: Child,
}

#[derive(Clone, Copy)]
enum DragKind {
    Move,
    Resize,
}

#[derive(Clone)]
struct DragState {
    surface: WlSurface,
    button: u32,
    kind: DragKind,
    start: Point<f64, Logical>,
    initial_rect: (i32, i32, i32, i32),
}

#[derive(Clone, Copy)]
struct PanDragState {
    button: u32,
    start: Point<f64, Logical>,
    initial_offset: (f64, f64),
}

impl BufferHandler for Compositor {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl CompositorHandler for Compositor {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client
            .get_data::<WaylandClientState>()
            .expect("AetherWM Wayland client state missing")
            .compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        self.needs_redraw = true;
    }
}

impl XdgShellHandler for Compositor {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let wl_surface = surface.wl_surface().clone();
        self.surface_views
            .push((wl_surface.clone(), self.current_view));
        if self.config.default_layout == "floating" {
            self.floating_surfaces.push(wl_surface.clone());
        }
        self.floating_rects
            .push((wl_surface, centered_window_rect(self.output_size)));
        let initial_size =
            Size::<i32, Logical>::new(self.output_size.0.max(1), self.output_size.1.max(1));
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
            state.size = Some(initial_size);
        });
        surface.send_configure();
        self.layout_dirty = true;
        self.needs_redraw = true;
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {}

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
    }
}

impl ShmHandler for Compositor {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl OutputHandler for Compositor {}

impl SeatHandler for Compositor {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}

    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }
}

impl SelectionHandler for Compositor {
    type SelectionUserData = ();
}

impl DataDeviceHandler for Compositor {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for Compositor {}

impl ServerDndGrabHandler for Compositor {
    fn send(&mut self, _mime_type: String, _fd: OwnedFd, _seat: Seat<Self>) {}
}

#[derive(Default)]
struct WaylandClientState {
    compositor_state: CompositorClientState,
}

impl ClientData for WaylandClientState {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|arg| arg == "--nested") {
        #[cfg(feature = "wayland-nested")]
        return run_nested();
        #[cfg(not(feature = "wayland-nested"))]
        return Err("nested backend was disabled at build time".into());
    }

    #[cfg(feature = "wayland-tty")]
    return tty::run();

    #[cfg(all(not(feature = "wayland-tty"), feature = "wayland-nested"))]
    run_nested()
}

#[cfg(feature = "wayland-nested")]
fn run_nested() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();
    let attributes = Window::default_attributes()
        .with_inner_size(LogicalSize::new(1280.0, 720.0))
        .with_title("AetherWM Wayland");
    let (mut backend, mut event_loop) = winit::init_from_attributes_with_gl_attr::<GlesRenderer>(
        attributes,
        GlAttributes {
            version: (3, 0),
            profile: None,
            debug: cfg!(debug_assertions),
            vsync: config.vsync,
        },
    )?;
    let window_size = backend.window_size();
    let mut display: Display<Compositor> = Display::new()?;
    let display_handle = display.handle();

    let compositor_state = CompositorState::new::<Compositor>(&display_handle);
    let xdg_shell_state = XdgShellState::new::<Compositor>(&display_handle);
    let shm_state = ShmState::new::<Compositor>(&display_handle, Vec::new());
    let mut seat_state = SeatState::new();
    let seat = seat_state.new_wl_seat(&display_handle, "aetherwm");
    let output = Output::new(
        "AetherWM-0".into(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "AetherWM".into(),
            model: "Nested output".into(),
        },
    );
    let initial_mode = Mode {
        size: (window_size.w, window_size.h).into(),
        refresh: 60_000,
    };
    output.set_preferred(initial_mode);
    output.change_current_state(
        Some(initial_mode),
        Some(Transform::Normal),
        Some(Scale::Integer(1)),
        Some((0, 0).into()),
    );
    output.create_global::<Compositor>(&display_handle);
    let mut compositor = Compositor {
        compositor_state,
        xdg_shell_state,
        shm_state,
        seat_state,
        data_device_state: DataDeviceState::new::<Compositor>(&display_handle),
        seat,
        config,
        output,
        output_size: (window_size.w, window_size.h),
        layout_dirty: true,
        needs_redraw: true,
        pointer_location: (0.0, 0.0).into(),
        serial: 1,
        running: true,
        current_view: (0, 0),
        surface_views: Vec::new(),
        floating_surfaces: Vec::new(),
        floating_rects: Vec::new(),
        configured_sizes: Vec::new(),
        focused_index: 0,
        overview: false,
        drag: None,
        pan_drag: None,
        pan_offset: (0.0, 0.0),
        children: Vec::new(),
    };

    let keyboard = compositor.seat.add_keyboard(Default::default(), 200, 200)?;
    compositor.seat.add_pointer();
    let socket = ListeningSocket::bind("wayland-aetherwm")?;
    // SAFETY: this executable owns the process environment before spawning any
    // client processes from the compositor.
    unsafe { std::env::set_var("WAYLAND_DISPLAY", "wayland-aetherwm") };

    let start = Instant::now();
    let mut clients = Vec::new();
    eprintln!("AetherWM Wayland compositor listening on wayland-aetherwm");
    for command in &compositor.config.autostart {
        if let Some(child) = spawn_client_command(command) {
            compositor.children.push(child);
        }
    }

    let mut client_display = display.handle();
    let connected_clients = clients.clone();
    loop_handle.insert_source(socket, move |stream, _, _| {
        match client_display.insert_client(stream, Arc::new(WaylandClientState::default())) {
            Ok(client) => connected_clients.borrow_mut().push(client),
            Err(error) => eprintln!("AetherWM: could not accept Wayland client: {error}"),
        }
    })?;

    while compositor.running {
        super::reap_children(&mut compositor.children);
        event_loop.dispatch(Some(Duration::from_millis(8)), &mut compositor)?;
        display.dispatch_clients(&mut compositor)?;

        let current_surfaces = compositor.xdg_shell_state.toplevel_surfaces();
        let closed_surface = compositor.surface_views.iter().any(|(known, _)| {
            !current_surfaces
                .iter()
                .any(|surface| surface.wl_surface() == known)
        });
        if closed_surface {
            compositor.layout_dirty = true;
            compositor.needs_redraw = true;
        }
        compositor.surface_views.retain(|(known, _)| {
            current_surfaces
                .iter()
                .any(|surface| surface.wl_surface() == known)
        });
        compositor.floating_surfaces.retain(|known| {
            current_surfaces
                .iter()
                .any(|surface| surface.wl_surface() == known)
        });
        compositor.floating_rects.retain(|(known, _)| {
            current_surfaces
                .iter()
                .any(|surface| surface.wl_surface() == known)
        });
        compositor.configured_sizes.retain(|(known, _)| {
            current_surfaces
                .iter()
                .any(|surface| surface.wl_surface() == known)
        });

        if compositor.layout_dirty {
            let surfaces = visible_surfaces(&compositor);
            let visible_roots = surfaces
                .iter()
                .map(|surface| surface.wl_surface().clone())
                .collect::<Vec<_>>();
            for surface in compositor.xdg_shell_state.toplevel_surfaces() {
                if !visible_roots
                    .iter()
                    .any(|visible| visible == surface.wl_surface())
                {
                    compositor.output.leave(surface.wl_surface());
                }
            }
            let boxes = window_boxes(&compositor, &surfaces);
            for (surface, (_, _, width, height)) in surfaces.iter().zip(boxes) {
                compositor.output.enter(surface.wl_surface());
                configure_toplevel_size(&mut compositor, surface, width, height);
            }
            compositor.layout_dirty = false;
            compositor.needs_redraw = true;
        }

        if compositor.needs_redraw && session.is_active() {
            let size = compositor.output_size.into();
            let damage = Rectangle::from_size(size);
            let (mut dmabuf, _) = match output_surface.borrow_mut().next_buffer() {
                Ok(buffer) => buffer,
                Err(error) => {
                    // A swapchain can be temporarily exhausted until the next page flip.
                    eprintln!("AetherWM: waiting for a DRM output buffer: {error:?}");
                    display.flush_clients()?;
                    continue;
                }
            };
            let mut framebuffer = renderer.bind(&mut dmabuf)?;
            let surfaces = visible_surfaces(&compositor);
            let boxes = window_boxes(&compositor, &surfaces);
            let elements = surfaces
                .iter()
                .zip(boxes.iter().copied())
                .flat_map(|(surface, (x, y, _, _))| {
                    render_elements_from_surface_tree(
                        &mut renderer,
                        surface.wl_surface(),
                        (x, y),
                        1.0,
                        compositor.config.window_opacity as f32 / 100.0,
                        Kind::Unspecified,
                    )
                })
                .collect::<Vec<WaylandSurfaceRenderElement<GlesRenderer>>>();
            let mut frame = renderer.render(&mut framebuffer, size, Transform::Normal)?;
            frame.clear(Color32F::new(0.035, 0.045, 0.07, 1.0), &[damage])?;
            draw_render_elements(&mut frame, 1.0, &elements, &[damage])
                .map_err(|error| std::io::Error::other(format!("render surfaces: {error:?}")))?;
            draw_window_decorations(
                &mut frame,
                &compositor.config,
                &boxes,
                compositor.focused_index,
            )?;
            let cursor_x = compositor.pointer_location.x.round() as i32;
            let cursor_y = compositor.pointer_location.y.round() as i32;
            for (row, width) in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 10, 7, 5, 4]
                .into_iter()
                .enumerate()
            {
                let outline =
                    Rectangle::new((cursor_x, cursor_y + row as i32).into(), (width, 1).into());
                frame.clear(Color32F::new(0.02, 0.025, 0.04, 1.0), &[outline])?;
                if width > 2 {
                    let fill = Rectangle::new(
                        (cursor_x + 1, cursor_y + row as i32).into(),
                        (width - 2, 1).into(),
                    );
                    frame.clear(Color32F::new(0.95, 0.97, 1.0, 1.0), &[fill])?;
                }
            }
            let sync = frame.finish()?;

            let frame_time = start.elapsed().as_millis() as u32;
            for surface in super::visible_surfaces(&compositor) {
                super::send_frame_callbacks(surface.wl_surface(), frame_time);
            }
            drop(framebuffer);
            output_surface
                .borrow_mut()
                .queue_buffer(Some(sync), Some(vec![damage]), frame_time)?;
            compositor.needs_redraw = false;
        }
        display.flush_clients()?;
    }

    drop(clients);
    Ok(())
}

fn select_connected_output(
    device: &DrmDevice,
) -> Result<
    (
        control::connector::Handle,
        control::crtc::Handle,
        control::Mode,
        (u32, u32),
    ),
    Box<dyn std::error::Error>,
> {
    let resources = device.resource_handles()?;
    for connector_handle in resources.connectors() {
        let connector = device.get_connector(*connector_handle, true)?;
        if connector.state() != control::connector::State::Connected {
            continue;
        }
        let Some(mode) = connector.modes().iter().copied().max_by_key(|mode| {
            let (width, height) = mode.size();
            (
                mode.mode_type().contains(control::ModeTypeFlags::PREFERRED),
                u32::from(width) * u32::from(height),
                mode.vrefresh(),
            )
        }) else {
            continue;
        };
        let Some(encoder_handle) = connector
            .current_encoder()
            .or_else(|| connector.encoders().first().copied())
        else {
            continue;
        };
        let encoder = device.get_encoder(encoder_handle)?;
        let Some(crtc) = encoder.crtc().or_else(|| {
            resources
                .filter_crtcs(encoder.possible_crtcs())
                .into_iter()
                .next()
        }) else {
            continue;
        };
        let physical_size = connector.size().unwrap_or((0, 0));
        return Ok((*connector_handle, crtc, mode, physical_size));
    }

    Err(std::io::Error::other("no connected DRM output with a usable mode was found").into())
}

fn handle_input_event<B: InputBackend>(compositor: &mut Compositor, event: InputEvent<B>) {
    match event {
        InputEvent::Keyboard { event } => {
            let key_state = event.state();
            if let Some(keyboard) = compositor.seat.get_keyboard() {
                let time = event.time_msec();
                keyboard.input::<(), _>(
                    compositor,
                    event.key_code(),
                    key_state,
                    0.into(),
                    time,
                    |state, modifiers, key| {
                        if key_state != smithay::backend::input::KeyState::Pressed {
                            return FilterResult::Forward;
                        }
                        let keysym = key.modified_sym();
                        let action = state
                            .config
                            .bindings
                            .iter()
                            .find(|binding| {
                                let expected =
                                    xkb::keysym_from_name(&binding.key, xkb::KEYSYM_NO_FLAGS);
                                expected != xkb::Keysym::NoSymbol
                                    && expected == keysym
                                    && super::binding_modifiers_match(&binding.modifiers, modifiers)
                            })
                            .map(|binding| binding.action.clone());
                        if let Some(action) = action {
                            match action.as_str() {
                                "toggleoverview" => {
                                    compositor.overview = !compositor.overview;
                                    compositor.layout_dirty = true;
                                    compositor.needs_redraw = true;
                                }
                                _ => {}
                            }
                            super::run_action(state, &action, &keyboard);
                            FilterResult::Intercept(())
                        } else {
                            FilterResult::Forward
                        }
                    },
                );
            }
        }
        InputEvent::PointerMotion { event } => {
            let delta = event.delta();
            let location = (
                (compositor.pointer_location.x + delta.x)
                    .clamp(0.0, (compositor.output_size.0 - 1).max(0) as f64),
                (compositor.pointer_location.y + delta.y)
                    .clamp(0.0, (compositor.output_size.1 - 1).max(0) as f64),
            )
                .into();
            handle_pointer_motion(compositor, location, event.time_msec());
        }
        InputEvent::PointerMotionAbsolute { event } => {
            let location = event.position_transformed(
                (
                    compositor.output_size.0.max(1),
                    compositor.output_size.1.max(1),
                )
                    .into(),
            );
            handle_pointer_motion(compositor, location, event.time_msec());
        }
        InputEvent::PointerButton { event } => {
            let button_code = event.button_code();
            let button_state = event.state();
            let super_held = compositor
                .seat
                .get_keyboard()
                .is_some_and(|keyboard| keyboard.modifier_state().logo);
            if button_state == ButtonState::Released && super::finish_pan(compositor, button_code) {
                let first = visible_surfaces(compositor)
                    .first()
                    .map(|surface| surface.wl_surface().clone());
                let serial = super::next_serial(compositor);
                if let Some(keyboard) = compositor.seat.get_keyboard() {
                    keyboard.set_focus(compositor, first, serial);
                }
            } else if button_state == ButtonState::Pressed
                && ((super_held && button_code == 0x112)
                    || (!super_held
                        && button_code == 0x110
                        && super::surface_at(compositor, compositor.pointer_location).is_none()))
            {
                super::begin_pan(compositor, button_code);
            } else if let Some(drag) = compositor.drag.clone() {
                if button_state == ButtonState::Released && button_code == drag.button {
                    compositor.drag = None;
                }
            } else if button_state == ButtonState::Pressed
                && keyboard.modifier_state().logo
                && !compositor.overview
                && matches!(button_code, 0x110 | 0x111)
            {
                if let Some((surface, _)) =
                    super::surface_at(compositor, compositor.pointer_location)
                {
                    let surfaces = visible_surfaces(compositor);
                    if let Some(index) = surfaces
                        .iter()
                        .position(|candidate| candidate.wl_surface() == &surface)
                    {
                        let rect = super::window_boxes(compositor, &surfaces)[index];
                        if !compositor
                            .floating_surfaces
                            .iter()
                            .any(|floating| floating == &surface)
                        {
                            compositor.floating_surfaces.push(surface.clone());
                        }
                        super::store_floating_rect(compositor, &surface, rect);
                        compositor.focused_index = index;
                        compositor.drag = Some(super::DragState {
                            surface: surface.clone(),
                            button: button_code,
                            kind: if button_code == 0x110 {
                                super::DragKind::Move
                            } else {
                                super::DragKind::Resize
                            },
                            start: compositor.pointer_location,
                            initial_rect: rect,
                        });
                        let serial = super::next_serial(compositor);
                        if let Some(keyboard) = compositor.seat.get_keyboard() {
                            keyboard.set_focus(compositor, Some(surface), serial);
                        }
                        compositor.layout_dirty = true;
                        compositor.needs_redraw = true;
                    }
                }
            } else {
                if button_state == ButtonState::Pressed && compositor.overview {
                    compositor.overview = false;
                    compositor.layout_dirty = true;
                    compositor.needs_redraw = true;
                }
                let serial = super::next_serial(compositor);
                if let Some(pointer) = compositor.seat.get_pointer() {
                    pointer.button(
                        compositor,
                        &ButtonEvent {
                            serial,
                            time: event.time_msec(),
                            button: button_code,
                            state: button_state,
                        },
                    );
                    pointer.frame(compositor);
                }
            }
        }
        _ => {}
    }
}

fn handle_pointer_motion(
    compositor: &mut Compositor,
    location: smithay::utils::Point<f64, Logical>,
    time: u32,
) {
    if compositor.pan_drag.is_some() {
        super::update_pan(compositor, location);
        return;
    }
    compositor.pointer_location = location;
    if let Some(drag) = compositor.drag.clone() {
        let dx = (location.x - drag.start.x).round() as i32;
        let dy = (location.y - drag.start.y).round() as i32;
        let rect = match drag.kind {
            super::DragKind::Move => (
                drag.initial_rect.0 + dx,
                drag.initial_rect.1 + dy,
                drag.initial_rect.2,
                drag.initial_rect.3,
            ),
            super::DragKind::Resize => (
                drag.initial_rect.0,
                drag.initial_rect.1,
                (drag.initial_rect.2 + dx).max(100),
                (drag.initial_rect.3 + dy).max(64),
            ),
        };
        let output_size = compositor.output_size;
        super::store_floating_rect(
            compositor,
            &drag.surface,
            super::clamp_rect(rect, output_size),
        );
        if matches!(drag.kind, super::DragKind::Resize) {
            compositor.layout_dirty = true;
        }
    } else {
        let focus = super::surface_at(compositor, location);
        if let Some((surface, _)) = focus.as_ref()
            && let Some(index) = visible_surfaces(compositor)
                .iter()
                .position(|candidate| candidate.wl_surface() == surface)
        {
            compositor.focused_index = index;
        }
        let serial = super::next_serial(compositor);
        if let Some(pointer) = compositor.seat.get_pointer() {
            pointer.motion(
                &mut compositor,
                focus.clone(),
                &MotionEvent {
                    location,
                    serial,
                    time,
                },
            );
            pointer.frame(&mut compositor);
        }
        if let Some((surface, _)) = focus
            && let Some(keyboard) = compositor.seat.get_keyboard()
        {
            keyboard.set_focus(&mut compositor, Some(surface), serial);
        }
    }
    compositor.needs_redraw = true;
}
