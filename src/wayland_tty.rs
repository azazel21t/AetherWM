use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use smithay::backend::allocator::Fourcc;
use smithay::backend::allocator::gbm::{GbmAllocator, GbmBufferFlags, GbmDevice};
use smithay::backend::drm::{DrmDevice, DrmDeviceFd, DrmEvent, GbmBufferedSurface};
use smithay::backend::egl::{EGLContext, EGLDevice, EGLDisplay};
use smithay::backend::input::{
    AbsolutePositionEvent, ButtonState, Event, InputBackend, InputEvent, KeyboardKeyEvent,
    PointerButtonEvent, PointerMotionEvent,
};
use smithay::backend::libinput::{LibinputInputBackend, LibinputSessionInterface};
use smithay::backend::renderer::element::surface::{
    WaylandSurfaceRenderElement, render_elements_from_surface_tree,
};
use smithay::backend::renderer::utils::draw_render_elements;
use smithay::backend::renderer::{Bind, Color32F, Frame, Renderer};
use smithay::backend::renderer::{element::Kind, gles::GlesRenderer};
use smithay::backend::session::{Event as SessionEvent, Session, libseat::LibSeatSession};
use smithay::backend::udev::primary_gpu;
use smithay::input::keyboard::{FilterResult, xkb};
use smithay::input::pointer::{ButtonEvent, MotionEvent};
use smithay::output::{Mode as OutputMode, Output, PhysicalProperties, Scale, Subpixel};
use smithay::reexports::calloop::EventLoop;
use smithay::reexports::drm::control::{self, Device as ControlDevice};
use smithay::reexports::input::Libinput;
use smithay::reexports::rustix::fs::OFlags;
use smithay::reexports::wayland_server::{Display, ListeningSocket};
use smithay::utils::{DeviceFd, Rectangle, Transform};
use smithay::wayland::compositor::CompositorState;
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::wayland::shm::ShmState;

use super::{Compositor, WaylandClientState};

pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = super::load_config();
    let (mut session, session_notifier) = LibSeatSession::new().map_err(|error| {
        std::io::Error::other(format!(
            "unable to open a seat session through libseat; launch AetherWM from a local TTY or a Wayland-capable display manager: {error:?}"
        ))
    })?;
    let seat_name = session.seat();
    let gpu_path = primary_gpu(&seat_name)?
        .ok_or_else(|| std::io::Error::other(format!("no DRM GPU found for seat {seat_name}")))?;

    let drm_owned_fd = session.open(&gpu_path, OFlags::RDWR).map_err(|error| {
        std::io::Error::other(format!(
            "cannot open {} through libseat: {error:?}",
            gpu_path.display()
        ))
    })?;
    let device_fd = DeviceFd::from(drm_owned_fd);
    let drm_fd = DrmDeviceFd::new(device_fd.clone());
    let (mut drm_device, drm_notifier) = DrmDevice::new(drm_fd, true)?;
    let (connector, crtc, drm_mode, physical_size) = select_connected_output(&drm_device)?;
    let mode_size = drm_mode.size();
    let output_size = (i32::from(mode_size.0), i32::from(mode_size.1));
    let drm_surface = drm_device.create_surface(crtc, drm_mode, &[connector])?;

    let egl_device = EGLDevice::enumerate()?
        .find(|device| device.drm_device_path().is_ok_and(|path| path == gpu_path))
        .ok_or_else(|| {
            std::io::Error::other(format!(
                "no EGL device matches {}; check that the GPU driver provides EGL device support",
                gpu_path.display()
            ))
        })?;
    // SAFETY: Smithay owns and keeps the EGL device alive for the display lifetime.
    let egl_display = unsafe { EGLDisplay::new(egl_device)? };
    let renderer_formats = egl_display.dmabuf_render_formats().clone();
    let egl_context = EGLContext::new(&egl_display)?;
    // SAFETY: the context was created from Smithay's live EGL display above.
    let mut renderer = unsafe { GlesRenderer::new(egl_context)? };
    let gbm_device = GbmDevice::new(device_fd)?;
    let allocator = GbmAllocator::new(gbm_device, GbmBufferFlags::RENDERING);
    let output_surface = GbmBufferedSurface::new(
        drm_surface,
        allocator,
        &[Fourcc::Xrgb8888, Fourcc::Argb8888],
        renderer_formats,
    )?;
    let output_surface = Rc::new(RefCell::new(output_surface));
    let drm_device = Rc::new(RefCell::new(drm_device));

    let mut display: Display<Compositor> = Display::new()?;
    let display_handle = display.handle();
    let compositor_state = CompositorState::new::<Compositor>(&display_handle);
    let xdg_shell_state = XdgShellState::new::<Compositor>(&display_handle);
    let shm_state = ShmState::new::<Compositor>(&display_handle, Vec::new());
    let mut seat_state = smithay::input::SeatState::new();
    let seat = seat_state.new_wl_seat(&display_handle, "aetherwm");
    let output = Output::new(
        "AetherWM-0".into(),
        PhysicalProperties {
            size: (physical_size.0 as i32, physical_size.1 as i32).into(),
            subpixel: Subpixel::Unknown,
            make: "AetherWM".into(),
            model: "DRM output".into(),
        },
    );
    let output_mode = OutputMode {
        size: output_size.into(),
        refresh: drm_mode.vrefresh().saturating_mul(1000) as i32,
    };
    output.set_preferred(output_mode);
    output.change_current_state(
        Some(output_mode),
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
        output_size,
        layout_dirty: true,
        needs_redraw: true,
        pointer_location: (0.0, 0.0).into(),
        serial: 1,
        running: true,
        current_view: (0, 0),
        surface_views: Vec::new(),
        floating_surfaces: Vec::new(),
        floating_rects: Vec::new(),
        focused_index: 0,
        overview: false,
        drag: None,
    };
    compositor.seat.add_keyboard(Default::default(), 200, 200)?;
    compositor.seat.add_pointer();

    let mut libinput = Libinput::new_with_udev(LibinputSessionInterface::from(session.clone()));
    libinput.udev_assign_seat(&seat_name).map_err(|_| {
        std::io::Error::other(format!("libinput could not attach to seat {seat_name}"))
    })?;

    let mut event_loop = EventLoop::<Compositor>::try_new()?;
    let loop_handle = event_loop.handle();
    loop_handle.insert_source(
        LibinputInputBackend::new(libinput),
        |event, _, compositor| handle_input_event(compositor, event),
    )?;

    let output_surface_for_events = output_surface.clone();
    loop_handle.insert_source(drm_notifier, move |event, _, _compositor| match event {
        DrmEvent::VBlank(_) => {
            if let Err(error) = output_surface_for_events.borrow_mut().frame_submitted() {
                eprintln!("AetherWM: DRM page flip completion failed: {error:?}");
            }
        }
        DrmEvent::Error(error) => eprintln!("AetherWM: DRM event error: {error:?}"),
    })?;

    let drm_device_for_session = drm_device.clone();
    loop_handle.insert_source(session_notifier, move |event, _, compositor| match event {
        SessionEvent::PauseSession => {
            drm_device_for_session.borrow_mut().pause();
        }
        SessionEvent::ActivateSession => {
            if let Err(error) = drm_device_for_session.borrow_mut().activate(true) {
                eprintln!("AetherWM: could not reactivate the DRM device: {error:?}");
                compositor.running = false;
            } else {
                compositor.needs_redraw = true;
            }
        }
    })?;

    let socket = ListeningSocket::bind("wayland-aetherwm")?;
    // SAFETY: the compositor sets its client environment before spawning children.
    unsafe {
        std::env::set_var("WAYLAND_DISPLAY", "wayland-aetherwm");
        std::env::set_var("XDG_SESSION_TYPE", "wayland");
    }

    let start = Instant::now();
    let mut clients = Vec::new();
    eprintln!(
        "AetherWM native Wayland compositor on {} at {}x{} (seat {})",
        gpu_path.display(),
        output_size.0,
        output_size.1,
        seat_name
    );
    for command in &compositor.config.autostart {
        super::spawn_client_command(command);
    }

    while compositor.running {
        event_loop.dispatch(Some(Duration::from_millis(8)), &mut compositor)?;

        while let Some(stream) = socket.accept()? {
            let client = display
                .handle()
                .insert_client(stream, Arc::new(WaylandClientState::default()))?;
            clients.push(client);
        }
        display.dispatch_clients(&mut compositor)?;

        let current_surfaces = compositor.xdg_shell_state.toplevel_surfaces();
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

        if compositor.layout_dirty {
            let surfaces = super::visible_surfaces(&compositor);
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
            let boxes = super::window_boxes(&compositor, &surfaces);
            for (surface, (_, _, width, height)) in surfaces.iter().zip(boxes) {
                compositor.output.enter(surface.wl_surface());
                surface.with_pending_state(|state| {
                    state.size = Some((width, height).into());
                });
                surface.send_configure();
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
            let surfaces = super::visible_surfaces(&compositor);
            let boxes = super::window_boxes(&compositor, &surfaces);
            let elements = surfaces
                .iter()
                .zip(boxes)
                .flat_map(|(surface, (x, y, _, _))| {
                    render_elements_from_surface_tree(
                        &mut renderer,
                        surface.wl_surface(),
                        (x, y),
                        1.0,
                        1.0,
                        Kind::Unspecified,
                    )
                })
                .collect::<Vec<WaylandSurfaceRenderElement<GlesRenderer>>>();
            let mut frame = renderer.render(&mut framebuffer, size, Transform::Normal)?;
            frame.clear(Color32F::new(0.035, 0.045, 0.07, 1.0), &[damage])?;
            draw_render_elements(&mut frame, 1.0, &elements, &[damage])
                .map_err(|error| std::io::Error::other(format!("render surfaces: {error:?}")))?;
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
            if let Some(drag) = compositor.drag.clone() {
                if button_state == ButtonState::Released && button_code == drag.button {
                    compositor.drag = None;
                }
            } else if button_state == ButtonState::Pressed
                && compositor
                    .seat
                    .get_keyboard()
                    .is_some_and(|keyboard| keyboard.modifier_state().logo)
                && !compositor.overview
                && matches!(button_code, 0x110 | 0x111)
            {
                if let Some((surface, _)) =
                    super::surface_at(compositor, compositor.pointer_location)
                {
                    let surfaces = super::visible_surfaces(compositor);
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
    location: smithay::utils::Point<f64, smithay::utils::Logical>,
    time: u32,
) {
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
            && let Some(index) = super::visible_surfaces(compositor)
                .iter()
                .position(|candidate| candidate.wl_surface() == surface)
        {
            compositor.focused_index = index;
        }
        let serial = super::next_serial(compositor);
        if let Some(pointer) = compositor.seat.get_pointer() {
            pointer.motion(
                compositor,
                focus.clone(),
                &MotionEvent {
                    location,
                    serial,
                    time,
                },
            );
            pointer.frame(compositor);
        }
        if let Some((surface, _)) = focus
            && let Some(keyboard) = compositor.seat.get_keyboard()
        {
            keyboard.set_focus(compositor, Some(surface), serial);
        }
    }
    compositor.needs_redraw = true;
}
