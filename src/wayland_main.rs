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

    loop {
        reap_children(&mut compositor.children);
        let status = event_loop.dispatch_new_events(|event| match event {
            WinitEvent::Input(InputEvent::Keyboard { event }) => {
                let key_state = event.state();
                keyboard.input::<(), _>(
                    &mut compositor,
                    event.key_code(),
                    key_state,
                    0.into(),
                    start.elapsed().as_millis() as u32,
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
                                    && binding_modifiers_match(&binding.modifiers, modifiers)
                            })
                            .map(|binding| binding.action.clone());
                        if let Some(action) = action {
                            run_action(state, &action, &keyboard);
                            FilterResult::Intercept(())
                        } else {
                            FilterResult::Forward
                        }
                    },
                );
            }
            WinitEvent::Input(InputEvent::PointerMotionAbsolute { event }) => {
                let location = event.position_transformed(Size::<i32, Logical>::new(
                    compositor.output_size.0.max(1),
                    compositor.output_size.1.max(1),
                ));
                if compositor.pan_drag.is_some() {
                    update_pan(&mut compositor, location);
                } else {
                    compositor.pointer_location = location;
                    if let Some(drag) = compositor.drag.clone() {
                        let dx = (location.x - drag.start.x).round() as i32;
                        let dy = (location.y - drag.start.y).round() as i32;
                        let rect = match drag.kind {
                            DragKind::Move => (
                                drag.initial_rect.0 + dx,
                                drag.initial_rect.1 + dy,
                                drag.initial_rect.2,
                                drag.initial_rect.3,
                            ),
                            DragKind::Resize => (
                                drag.initial_rect.0,
                                drag.initial_rect.1,
                                (drag.initial_rect.2 + dx).max(100),
                                (drag.initial_rect.3 + dy).max(64),
                            ),
                        };
                        let output_size = compositor.output_size;
                        store_floating_rect(
                            &mut compositor,
                            &drag.surface,
                            clamp_rect(rect, output_size),
                        );
                        if matches!(drag.kind, DragKind::Resize) {
                            compositor.layout_dirty = true;
                        }
                    } else {
                        let focus = surface_at(&compositor, location);
                        if compositor.config.focus_follows_mouse {
                            if let Some((surface, _)) = focus.as_ref()
                                && let Some(index) = visible_surfaces(&compositor)
                                    .iter()
                                    .position(|candidate| candidate.wl_surface() == surface)
                            {
                                compositor.focused_index = index;
                            }
                        }
                        let time = event.time() as u32;
                        let serial = next_serial(&mut compositor);
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
                        if compositor.config.focus_follows_mouse {
                            if let Some((surface, _)) = focus {
                                keyboard.set_focus(&mut compositor, Some(surface), serial);
                            }
                        }
                    }
                }
                compositor.needs_redraw = true;
            }
            WinitEvent::Input(InputEvent::PointerButton { event }) => {
                let button_code = event.button_code();
                let button_state = event.state();
                let super_held = keyboard.modifier_state().logo;
                if button_state == ButtonState::Released && finish_pan(&mut compositor, button_code)
                {
                    let first = visible_surfaces(&compositor)
                        .first()
                        .map(|surface| surface.wl_surface().clone());
                    let serial = next_serial(&mut compositor);
                    keyboard.set_focus(&mut compositor, first, serial);
                } else if button_state == ButtonState::Pressed
                    && ((super_held && button_code == 0x112)
                        || (!super_held
                            && button_code == 0x110
                            && surface_at(&compositor, compositor.pointer_location).is_none()))
                {
                    begin_pan(&mut compositor, button_code);
                } else if let Some(drag) = compositor.drag.clone() {
                    if button_state == ButtonState::Released && button_code == drag.button {
                        compositor.drag = None;
                    }
                } else if button_state == ButtonState::Pressed
                    && keyboard.modifier_state().logo
                    && !compositor.overview
                    && matches!(button_code, 0x110 | 0x111)
                {
                    if let Some((surface, _)) = surface_at(&compositor, compositor.pointer_location)
                    {
                        let surfaces = visible_surfaces(&compositor);
                        if let Some(index) = surfaces
                            .iter()
                            .position(|candidate| candidate.wl_surface() == &surface)
                        {
                            let rect = window_boxes(&compositor, &surfaces)[index];
                            if !compositor
                                .floating_surfaces
                                .iter()
                                .any(|floating| floating == &surface)
                            {
                                compositor.floating_surfaces.push(surface.clone());
                            }
                            store_floating_rect(&mut compositor, &surface, rect);
                            compositor.focused_index = index;
                            compositor.drag = Some(DragState {
                                surface: surface.clone(),
                                button: button_code,
                                kind: if button_code == 0x110 {
                                    DragKind::Move
                                } else {
                                    DragKind::Resize
                                },
                                start: compositor.pointer_location,
                                initial_rect: rect,
                            });
                            let serial = next_serial(&mut compositor);
                            keyboard.set_focus(&mut compositor, Some(surface), serial);
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
                    let serial = next_serial(&mut compositor);
                    if let Some(pointer) = compositor.seat.get_pointer() {
                        pointer.button(
                            &mut compositor,
                            &ButtonEvent {
                                serial,
                                time: event.time() as u32,
                                button: button_code,
                                state: button_state,
                            },
                        );
                        pointer.frame(&mut compositor);
                    }
                }
            }
            WinitEvent::Resized { size, .. } => {
                compositor.output_size = (size.w.max(1), size.h.max(1));
                let mode = Mode {
                    size: (compositor.output_size.0, compositor.output_size.1).into(),
                    refresh: 60_000,
                };
                compositor.output.set_preferred(mode);
                compositor.output.change_current_state(
                    Some(mode),
                    Some(Transform::Normal),
                    Some(Scale::Integer(1)),
                    Some((0, 0).into()),
                );
                compositor.layout_dirty = true;
                compositor.needs_redraw = true;
            }
            WinitEvent::CloseRequested => compositor.running = false,
            _ => {}
        });
        if matches!(status, PumpStatus::Exit(_)) || !compositor.running {
            break;
        }

        let window_size = backend.window_size();
        if compositor.output_size != (window_size.w, window_size.h) {
            compositor.output_size = (window_size.w.max(1), window_size.h.max(1));
            let mode = Mode {
                size: (compositor.output_size.0, compositor.output_size.1).into(),
                refresh: 60_000,
            };
            compositor.output.set_preferred(mode);
            compositor.output.change_current_state(
                Some(mode),
                Some(Transform::Normal),
                Some(Scale::Integer(1)),
                Some((0, 0).into()),
            );
            compositor.layout_dirty = true;
            compositor.needs_redraw = true;
        }

        while let Some(stream) = socket.accept()? {
            let client = display
                .handle()
                .insert_client(stream, Arc::new(WaylandClientState::default()))?;
            clients.push(client);
        }

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

        if compositor.needs_redraw {
            let size = backend.window_size();
            let damage = Rectangle::from_size(size);
            let (renderer, mut framebuffer) = backend.bind()?;
            let surfaces = visible_surfaces(&compositor);
            let boxes = window_boxes(&compositor, &surfaces);
            let elements = surfaces
                .iter()
                .zip(boxes.iter().copied())
                .flat_map(|(surface, (x, y, _, _))| {
                    render_elements_from_surface_tree(
                        renderer,
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
            let _ = frame.finish()?;

            let frame_time = start.elapsed().as_millis() as u32;
            for surface in visible_surfaces(&compositor) {
                send_frame_callbacks(surface.wl_surface(), frame_time);
            }
            drop(framebuffer);
            backend.submit(Some(&[damage]))?;
            compositor.needs_redraw = false;
        }
        display.flush_clients()?;
        std::thread::sleep(Duration::from_millis(8));
    }

    drop(clients);
    Ok(())
}

fn send_frame_callbacks(surface: &wl_surface::WlSurface, time: u32) {
    with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| TraversalAction::DoChildren(()),
        |_surface, states, &()| {
            for callback in states
                .cached_state
                .get::<SurfaceAttributes>()
                .current()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, &()| true,
    );
}

fn tile_boxes(
    count: usize,
    output_size: (i32, i32),
    config: &config::Config,
    overview: bool,
) -> Vec<(i32, i32, i32, i32)> {
    if count == 0 {
        return Vec::new();
    }
    let (width, height) = (output_size.0.max(1), output_size.1.max(1));
    let outer = config.outer_gap.min(4096) as i32;
    let inner = config.inner_gap.min(4096) as i32;
    let x = outer;
    let y = outer;
    let available_width = (width - outer * 2).max(1);
    let available_height = (height - outer * 2).max(1);
    if overview {
        let columns = (count as f64).sqrt().ceil() as i32;
        let rows = (count as i32 + columns - 1) / columns;
        let tile_width = (available_width - inner * (columns - 1)).max(1) / columns;
        let tile_height = (available_height - inner * (rows - 1)).max(1) / rows;
        return (0..count)
            .map(|index| {
                let column = index as i32 % columns;
                let row = index as i32 / columns;
                (
                    x + column * (tile_width + inner),
                    y + row * (tile_height + inner),
                    tile_width.max(1),
                    tile_height.max(1),
                )
            })
            .collect();
    }
    if count == 1 {
        return vec![(x, y, available_width, available_height); count];
    }

    let master_width =
        (((available_width - inner).max(1) as f32) * config.master_ratio).round() as i32;
    let stack_width = (available_width - master_width - inner).max(1);
    let stack_count = count.saturating_sub(1) as i32;
    let stack_height =
        ((available_height - inner * (stack_count - 1).max(0)) / stack_count.max(1)).max(1);
    let mut boxes = Vec::with_capacity(count);
    boxes.push((x, y, master_width.max(1), available_height));
    for index in 0..stack_count {
        boxes.push((
            x + master_width + inner,
            y + index * (stack_height + inner),
            stack_width,
            stack_height,
        ));
    }
    boxes
}

pub(crate) fn draw_window_decorations<F: Frame>(
    frame: &mut F,
    config: &config::Config,
    boxes: &[(i32, i32, i32, i32)],
    focused_index: usize,
) -> Result<(), F::Error> {
    let rgb = |value: u64| {
        Color32F::new(
            ((value >> 16) & 0xff) as f32 / 255.0,
            ((value >> 8) & 0xff) as f32 / 255.0,
            (value & 0xff) as f32 / 255.0,
            1.0,
        )
    };
    let background = Color32F::new(0.035, 0.045, 0.07, 1.0);
    let border = config.border_width.min(32) as i32;
    let radius = config.corner_radius.min(48) as i32;
    for (index, &(x, y, width, height)) in boxes.iter().enumerate() {
        if border > 0 {
            let color = rgb(if index == focused_index {
                config.focused_border_color
            } else {
                config.border_color
            });
            let b = border.min(width / 2).min(height / 2).max(0);
            for (rx, ry, rw, rh) in [
                (x, y, width, b),
                (x, y + height - b, width, b),
                (x, y + b, b, (height - 2 * b).max(0)),
                (x + width - b, y + b, b, (height - 2 * b).max(0)),
            ] {
                frame.clear(color, &[Rectangle::new((rx, ry).into(), (rw, rh).into())])?;
            }
        }
        // Mask corner pixels with the desktop background. This works best with gaps.
        let r = radius.min(width / 2).min(height / 2);
        for dy in 0..r {
            let inset = r - ((r * r - (r - dy) * (r - dy)).max(0) as f64).sqrt().floor() as i32;
            for row_y in [y + dy, y + height - 1 - dy] {
                if inset > 0 {
                    for (rx, rw) in [(x, inset), (x + width - inset, inset)] {
                        frame.clear(
                            background,
                            &[Rectangle::new((rx, row_y).into(), (rw, 1).into())],
                        )?;
                    }
                }
            }
        }
        // The masks above also clip the square border strips. Restore the border
        // along the rounded left and right edges so the outline follows the arc.
        if border > 0 && r > 0 {
            let color = rgb(if index == focused_index {
                config.focused_border_color
            } else {
                config.border_color
            });
            let b = border.min(width / 2).max(0);
            for dy in 0..r {
                let inset = r - ((r * r - (r - dy) * (r - dy)).max(0) as f64).sqrt().floor() as i32;
                let row_width = b.min(width.saturating_sub(2 * inset));
                if row_width > 0 {
                    for row_y in [y + dy, y + height - 1 - dy] {
                        frame.clear(
                            color,
                            &[Rectangle::new(
                                (x + inset, row_y).into(),
                                (row_width, 1).into(),
                            )],
                        )?;
                        frame.clear(
                            color,
                            &[Rectangle::new(
                                (x + width - inset - row_width, row_y).into(),
                                (row_width, 1).into(),
                            )],
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn binding_modifiers_match(
    binding: &[String],
    modifiers: &smithay::input::keyboard::ModifiersState,
) -> bool {
    let has = |name: &str| binding.iter().any(|modifier| modifier == name);
    modifiers.logo == has("Mod4")
        && modifiers.alt == has("Mod1")
        && modifiers.ctrl == (has("Control") || has("Ctrl"))
        && modifiers.shift == has("Shift")
}

fn run_action(
    compositor: &mut Compositor,
    action: &str,
    keyboard: &smithay::input::keyboard::KeyboardHandle<Compositor>,
) {
    if let Some(command) = action.strip_prefix("exec ") {
        if let Some(child) = spawn_client_command(command) {
            compositor.children.push(child);
        }
        return;
    }

    match action {
        "exit" => compositor.running = false,
        "killclient" => {
            if let Some(surface) = visible_surfaces(compositor).get(compositor.focused_index) {
                surface.send_close();
            }
        }
        "reload_config" => {
            compositor.config = load_config();
            compositor.layout_dirty = true;
        }
        "togglefloating" => {
            let surfaces = visible_surfaces(compositor);
            if let Some(surface) = surfaces.get(compositor.focused_index) {
                let wl_surface = surface.wl_surface().clone();
                if compositor
                    .floating_surfaces
                    .iter()
                    .any(|floating| floating == &wl_surface)
                {
                    compositor
                        .floating_surfaces
                        .retain(|floating| floating != &wl_surface);
                } else {
                    let rect = window_boxes(compositor, &surfaces)
                        .get(compositor.focused_index)
                        .copied()
                        .unwrap_or_else(|| centered_window_rect(compositor.output_size));
                    compositor.floating_surfaces.push(wl_surface.clone());
                    store_floating_rect(compositor, &wl_surface, rect);
                }
            }
            compositor.layout_dirty = true;
        }
        "zoom_in" => {
            compositor.config.master_ratio = (compositor.config.master_ratio + 0.05).min(0.9)
        }
        "zoom_out" => {
            compositor.config.master_ratio = (compositor.config.master_ratio - 0.05).max(0.1)
        }
        "zoom_reset" => compositor.config.master_ratio = 0.5,
        "toggleoverview" => compositor.overview = !compositor.overview,
        "matrix_left" | "matrix_right" | "matrix_up" | "matrix_down" => {
            compositor.pan_drag = None;
            compositor.pan_offset = (0.0, 0.0);
            match action {
                "matrix_left" => {
                    compositor.current_view.0 = compositor.current_view.0.saturating_sub(1)
                }
                "matrix_right" => {
                    compositor.current_view.0 = compositor.current_view.0.saturating_add(1)
                }
                "matrix_up" => {
                    compositor.current_view.1 = compositor.current_view.1.saturating_sub(1)
                }
                _ => compositor.current_view.1 = compositor.current_view.1.saturating_add(1),
            }
            compositor.focused_index = 0;
            let first = visible_surfaces(compositor)
                .first()
                .map(|surface| surface.wl_surface().clone());
            let serial = next_serial(compositor);
            keyboard.set_focus(compositor, first, serial);
        }
        _ => return,
    }
    compositor.layout_dirty = true;
    compositor.needs_redraw = true;
}

fn spawn_client_command(command: &str) -> Option<LaunchedCommand> {
    match Command::new("sh")
        .arg("-c")
        .arg(command)
        .env("WAYLAND_DISPLAY", "wayland-aetherwm")
        .env("XDG_SESSION_TYPE", "wayland")
        .env_remove("DISPLAY")
        .spawn()
    {
        Ok(child) => Some(LaunchedCommand {
            command: command.to_string(),
            child,
        }),
        Err(error) => {
            eprintln!("AetherWM: failed to start command '{command}': {error}");
            None
        }
    }
}

pub(crate) fn reap_children(children: &mut Vec<LaunchedCommand>) {
    children.retain_mut(|process| match process.child.try_wait() {
        Ok(Some(status)) => {
            if !status.success() {
                eprintln!(
                    "AetherWM: command '{}' exited with {status}",
                    process.command
                );
            }
            false
        }
        Ok(None) => true,
        Err(error) => {
            eprintln!("AetherWM: could not reap child process: {error}");
            false
        }
    });
}

fn next_serial(compositor: &mut Compositor) -> Serial {
    compositor.serial = compositor.serial.wrapping_add(1).max(1);
    Serial::from(compositor.serial)
}

fn surface_at(
    compositor: &Compositor,
    location: Point<f64, Logical>,
) -> Option<(WlSurface, Point<f64, Logical>)> {
    let surfaces = visible_surfaces(compositor);
    let boxes = window_boxes(compositor, &surfaces);
    surfaces
        .iter()
        .zip(boxes)
        .rev()
        .find(|(_, (x, y, width, height))| {
            location.x >= *x as f64
                && location.y >= *y as f64
                && location.x < (*x + *width) as f64
                && location.y < (*y + *height) as f64
        })
        .map(|(surface, (x, y, _, _))| (surface.wl_surface().clone(), (x as f64, y as f64).into()))
}

fn visible_surfaces(compositor: &Compositor) -> Vec<ToplevelSurface> {
    let output_width = compositor.output_size.0.max(1) as f64;
    let output_height = compositor.output_size.1.max(1) as f64;
    compositor
        .xdg_shell_state
        .toplevel_surfaces()
        .iter()
        .filter(|surface| {
            compositor
                .surface_views
                .iter()
                .find(|(known, _)| known == surface.wl_surface())
                .is_some_and(|(_, view)| {
                    if compositor.overview {
                        return *view == compositor.current_view;
                    }
                    let x = (view.0.saturating_sub(compositor.current_view.0) as f64
                        * output_width)
                        + compositor.pan_offset.0;
                    let y = (view.1.saturating_sub(compositor.current_view.1) as f64
                        * output_height)
                        + compositor.pan_offset.1;
                    x < output_width
                        && x + output_width > 0.0
                        && y < output_height
                        && y + output_height > 0.0
                })
        })
        .map(Clone::clone)
        .collect()
}

fn window_boxes(
    compositor: &Compositor,
    surfaces: &[ToplevelSurface],
) -> Vec<(i32, i32, i32, i32)> {
    if compositor.overview {
        return tile_boxes(
            surfaces.len(),
            compositor.output_size,
            &compositor.config,
            true,
        );
    }

    let mut groups: Vec<((i32, i32), Vec<usize>)> = Vec::new();
    for (index, surface) in surfaces.iter().enumerate() {
        let view = compositor
            .surface_views
            .iter()
            .find(|(known, _)| known == surface.wl_surface())
            .map(|(_, view)| *view)
            .unwrap_or(compositor.current_view);
        if let Some((_, indexes)) = groups.iter_mut().find(|(known, _)| *known == view) {
            indexes.push(index);
        } else {
            groups.push((view, vec![index]));
        }
    }

    let mut result = vec![centered_window_rect(compositor.output_size); surfaces.len()];
    let (output_width, output_height) = (
        compositor.output_size.0.max(1),
        compositor.output_size.1.max(1),
    );
    for (view, indexes) in groups {
        let tiled_count = indexes
            .iter()
            .filter(|&&index| {
                !compositor
                    .floating_surfaces
                    .iter()
                    .any(|floating| floating == surfaces[index].wl_surface())
            })
            .count();
        let tiled_boxes = tile_boxes(
            tiled_count,
            compositor.output_size,
            &compositor.config,
            false,
        );
        let mut tiled_index = 0;
        let shift_x = view
            .0
            .saturating_sub(compositor.current_view.0)
            .saturating_mul(output_width) as f64
            + compositor.pan_offset.0;
        let shift_y = view
            .1
            .saturating_sub(compositor.current_view.1)
            .saturating_mul(output_height) as f64
            + compositor.pan_offset.1;
        for index in indexes {
            let surface = &surfaces[index];
            let mut rect = if compositor
                .floating_surfaces
                .iter()
                .any(|floating| floating == surface.wl_surface())
            {
                compositor
                    .floating_rects
                    .iter()
                    .find(|(known, _)| known == surface.wl_surface())
                    .map(|(_, rect)| clamp_rect(*rect, compositor.output_size))
                    .unwrap_or_else(|| centered_window_rect(compositor.output_size))
            } else {
                let rect = tiled_boxes
                    .get(tiled_index)
                    .copied()
                    .unwrap_or_else(|| centered_window_rect(compositor.output_size));
                tiled_index += 1;
                rect
            };
            rect.0 = rect.0.saturating_add(shift_x.round() as i32);
            rect.1 = rect.1.saturating_add(shift_y.round() as i32);
            result[index] = rect;
        }
    }
    result
}

pub(crate) fn configure_toplevel_size(
    compositor: &mut Compositor,
    surface: &ToplevelSurface,
    width: i32,
    height: i32,
) {
    let size = (width.max(1), height.max(1));
    if compositor
        .configured_sizes
        .iter()
        .any(|(known, configured)| known == surface.wl_surface() && *configured == size)
    {
        return;
    }
    surface.with_pending_state(|state| {
        state.size = Some(Size::<i32, Logical>::new(size.0, size.1));
    });
    surface.send_configure();
    if let Some((_, configured)) = compositor
        .configured_sizes
        .iter_mut()
        .find(|(known, _)| known == surface.wl_surface())
    {
        *configured = size;
    } else {
        compositor
            .configured_sizes
            .push((surface.wl_surface().clone(), size));
    }
}

fn begin_pan(compositor: &mut Compositor, button: u32) {
    if compositor.pan_drag.is_some() || compositor.overview {
        return;
    }
    compositor.pan_drag = Some(PanDragState {
        button,
        start: compositor.pointer_location,
        initial_offset: compositor.pan_offset,
    });
    // Include adjacent matrix cells while dragging so the desktop moves as a canvas.
    compositor.layout_dirty = true;
    compositor.needs_redraw = true;
}

fn update_pan(compositor: &mut Compositor, location: Point<f64, Logical>) {
    if let Some(drag) = compositor.pan_drag {
        let before = visible_view_set(compositor);
        compositor.pointer_location = location;
        compositor.pan_offset = (
            drag.initial_offset.0 + location.x - drag.start.x,
            drag.initial_offset.1 + location.y - drag.start.y,
        );
        if before != visible_view_set(compositor) {
            compositor.layout_dirty = true;
        }
        compositor.needs_redraw = true;
    }
}

fn visible_view_set(compositor: &Compositor) -> Vec<(i32, i32)> {
    let width = compositor.output_size.0.max(1) as f64;
    let height = compositor.output_size.1.max(1) as f64;
    let mut views = Vec::new();
    for (_, view) in &compositor.surface_views {
        let x = view.0.saturating_sub(compositor.current_view.0) as f64 * width
            + compositor.pan_offset.0;
        let y = view.1.saturating_sub(compositor.current_view.1) as f64 * height
            + compositor.pan_offset.1;
        if x < width && x + width > 0.0 && y < height && y + height > 0.0 && !views.contains(view) {
            views.push(*view);
        }
    }
    views
}

fn finish_pan(compositor: &mut Compositor, button: u32) -> bool {
    let Some(drag) = compositor.pan_drag else {
        return false;
    };
    if drag.button != button {
        return false;
    }
    let width = compositor.output_size.0.max(1) as f64;
    let height = compositor.output_size.1.max(1) as f64;
    let shift_x = (-compositor.pan_offset.0 / width).round() as i32;
    let shift_y = (-compositor.pan_offset.1 / height).round() as i32;
    compositor.current_view.0 = compositor.current_view.0.saturating_add(shift_x);
    compositor.current_view.1 = compositor.current_view.1.saturating_add(shift_y);
    compositor.pan_offset.0 += shift_x as f64 * width;
    compositor.pan_offset.1 += shift_y as f64 * height;
    compositor.pan_drag = None;
    compositor.focused_index = 0;
    compositor.layout_dirty = true;
    compositor.needs_redraw = true;
    true
}

fn store_floating_rect(
    compositor: &mut Compositor,
    surface: &WlSurface,
    mut rect: (i32, i32, i32, i32),
) {
    if let Some((_, view)) = compositor
        .surface_views
        .iter()
        .find(|(known, _)| known == surface)
    {
        rect.0 = rect.0.saturating_sub(
            view.0
                .saturating_sub(compositor.current_view.0)
                .saturating_mul(compositor.output_size.0.max(1))
                .saturating_add(compositor.pan_offset.0.round() as i32),
        );
        rect.1 = rect.1.saturating_sub(
            view.1
                .saturating_sub(compositor.current_view.1)
                .saturating_mul(compositor.output_size.1.max(1))
                .saturating_add(compositor.pan_offset.1.round() as i32),
        );
    }
    if let Some((_, stored)) = compositor
        .floating_rects
        .iter_mut()
        .find(|(known, _)| known == surface)
    {
        *stored = rect;
    } else {
        compositor.floating_rects.push((surface.clone(), rect));
    }
}

fn centered_window_rect(output_size: (i32, i32)) -> (i32, i32, i32, i32) {
    let (width, height) = (output_size.0.max(1), output_size.1.max(1));
    let window_width = (width * 3 / 4).max(1);
    let window_height = (height * 3 / 4).max(1);
    (
        (width - window_width) / 2,
        (height - window_height) / 2,
        window_width,
        window_height,
    )
}

fn clamp_rect(rect: (i32, i32, i32, i32), output_size: (i32, i32)) -> (i32, i32, i32, i32) {
    let (output_width, output_height) = (output_size.0.max(1), output_size.1.max(1));
    let width = rect.2.clamp(1, output_width);
    let height = rect.3.clamp(1, output_height);
    (
        rect.0.clamp(0, output_width - width),
        rect.1.clamp(0, output_height - height),
        width,
        height,
    )
}

fn load_config() -> config::Config {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    let config_dir = config_home.join("aetherwm");
    config::load_config(config_dir.join("config.ars"))
        .or_else(|_| config::load_config(config_dir.join("config.asr")))
        .unwrap_or_default()
}

delegate_xdg_shell!(Compositor);
delegate_compositor!(Compositor);
delegate_shm!(Compositor);
delegate_seat!(Compositor);
delegate_data_device!(Compositor);
delegate_output!(Compositor);
