//! Isle nested in a window on another compositor, for development (`--winit`).

use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        egl::{EGLDevice, ffi},
        renderer::{ImportDma, ImportMemWl, damage::OutputDamageTracker, gles::GlesRenderer},
        winit::{self, WinitEvent, WinitEventLoop, WinitGraphicsBackend},
    },
    input::pointer::CursorImageStatus,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::winit::{cursor::Cursor, dpi::LogicalSize, window::WindowAttributes},
    utils::Transform,
    wayland::dmabuf::DmabufFeedbackBuilder,
};
use tracing::{info, warn};

use super::{Backend, CLEAR, Rendered};
use crate::state::{Isle, State};

pub struct Winit {
    backend: WinitGraphicsBackend<GlesRenderer>,
    output: Output,
    damage_tracker: OutputDamageTracker,
    /// Frames still to draw whole, after a resize leaves the buffers' contents unknown.
    full_redraw: u8,
    swap_interval_set: bool,
}

impl Winit {
    pub fn new() -> Result<(Self, WinitEventLoop), Box<dyn std::error::Error>> {
        let (backend, events) = winit::init_from_attributes::<GlesRenderer>(
            WindowAttributes::default()
                .with_title("Isle")
                .with_surface_size(LogicalSize::new(1600.0, 1000.0)),
        )?;
        let mode = Mode {
            size: backend.window_size(),
            refresh: 60_000,
        };
        let output = Output::new(
            "winit".into(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "Isle".into(),
                model: "Winit".into(),
                serial_number: "0".into(),
            },
        );
        output.change_current_state(
            Some(mode),
            Some(Transform::Flipped180),
            None,
            Some((0, 0).into()),
        );
        output.set_preferred(mode);
        let damage_tracker = OutputDamageTracker::from_output(&output);
        Ok((
            Self {
                backend,
                output,
                damage_tracker,
                full_redraw: 0,
                swap_interval_set: false,
            },
            events,
        ))
    }

    pub fn render(&mut self, isle: &mut Isle, output: &Output) -> Result<Rendered, String> {
        // NVIDIA's EGL refuses this query now and then; a full redraw covers it.
        let age = if self.full_redraw > 0 {
            self.full_redraw -= 1;
            0
        } else {
            self.backend.buffer_age().unwrap_or(0)
        };
        let (damage, states) = {
            let (renderer, mut framebuffer) = self.backend.bind().map_err(|e| e.to_string())?;
            let mut elements = Vec::new();
            isle.render_elements(renderer, output, None, &mut |e| elements.push(e));
            let result = self
                .damage_tracker
                .render_output(renderer, &mut framebuffer, age, &elements, CLEAR)
                .map_err(|e| format!("{e:?}"))?;
            if !self.swap_interval_set {
                // Smithay never sets it, and NVIDIA's default of 1 makes every swap wait
                // for the host's frame callback inside the loop. The surface is current
                // once something was drawn to it.
                let display = renderer.egl_context().display().get_display_handle();
                let ok = unsafe { ffi::egl::SwapInterval(display.handle, 0) };
                tracing::debug!(ok, "swap interval 0");
                self.swap_interval_set = true;
            }
            (result.damage.cloned(), result.states)
        };
        if let Some(damage) = &damage {
            let swap = std::time::Instant::now();
            self.backend
                .submit(Some(damage))
                .map_err(|e| e.to_string())?;
            tracing::trace!("swap took {:.1} ms", swap.elapsed().as_secs_f64() * 1000.0);
            self.backend.window().request_redraw();
        }
        Ok(Rendered {
            damaged: damage.is_some(),
            states,
            planes: crate::present::Planes::COMPOSITED,
            retry_in: None,
        })
    }

    pub fn renderer(&mut self) -> &mut GlesRenderer {
        self.backend.renderer()
    }

    pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
        self.backend.renderer().import_dmabuf(dmabuf, None).is_ok()
    }

    /// The host draws named cursors; Isle draws a client's cursor surface itself.
    pub fn update_cursor(&mut self, isle: &Isle) {
        let window = self.backend.window();
        match &isle.cursor_status {
            CursorImageStatus::Hidden | CursorImageStatus::Surface(_) => {
                window.set_cursor_visible(false)
            }
            CursorImageStatus::Named(icon) => {
                window.set_cursor_visible(true);
                window.set_cursor(Cursor::Icon(*icon));
            }
        }
    }
}

/// Registers the output, the dmabuf global and the window's events, once the state exists.
pub fn setup(state: &mut State, events: WinitEventLoop) -> Result<(), Box<dyn std::error::Error>> {
    let Backend::Winit(winit) = &mut state.backend else {
        unreachable!("setup for the winit backend");
    };
    let output = winit.output.clone();
    let _global = output.create_global::<State>(&state.isle.display_handle);

    let renderer = winit.backend.renderer();
    state.isle.shm_state.update_formats(renderer.shm_formats());
    let formats = renderer.dmabuf_formats();
    let node = EGLDevice::device_for_display(renderer.egl_context().display())
        .ok()
        .and_then(|device| device.try_get_render_node().ok().flatten());
    match node.and_then(|node| {
        DmabufFeedbackBuilder::new(node.dev_id(), formats.clone())
            .build()
            .ok()
    }) {
        Some(feedback) => {
            state
                .isle
                .dmabuf_state
                .create_global_with_default_feedback::<State>(
                    &state.isle.display_handle,
                    &feedback,
                );
        }
        None => {
            warn!("no render node for the host's EGL display; serving dmabuf v3");
            state
                .isle
                .dmabuf_state
                .create_global::<State>(&state.isle.display_handle, formats);
        }
    }

    state.isle.add_output(&output, (0, 0).into());
    info!(
        size = ?winit.backend.window_size(),
        scale = winit.backend.scale_factor(),
        "winit output"
    );

    state
        .isle
        .loop_handle
        .clone()
        .insert_source(events, move |event, _, state| {
            let _beat = state.isle.heartbeat.beat("winit");
            match event {
                WinitEvent::Resized { size, .. } => {
                    let mode = Mode {
                        size,
                        refresh: 60_000,
                    };
                    output.change_current_state(Some(mode), None, None, None);
                    output.set_preferred(mode);
                    let Backend::Winit(winit) = &mut state.backend else {
                        return;
                    };
                    winit.damage_tracker = OutputDamageTracker::from_output(&output);
                    winit.full_redraw = 4;
                    state.isle.queue_redraw(&output);
                }
                WinitEvent::Input(event) => state.process_input_event(event),
                WinitEvent::Redraw => state.on_vblank(&output, None),
                WinitEvent::CloseRequested => state.isle.loop_signal.stop(),
                WinitEvent::Focus(_) => {}
            }
        })
        .map_err(|e| e.error)?;
    Ok(())
}
