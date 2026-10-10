//! Isle with no display: outputs drawn into textures, for tests (`--headless`).

use std::collections::HashMap;

use smithay::{
    backend::{
        allocator::{Fourcc, dmabuf::Dmabuf},
        drm::DrmNode,
        egl::{EGLContext, EGLDevice, EGLDisplay},
        renderer::{
            Bind, ImportDma, ImportMemWl, Offscreen,
            damage::OutputDamageTracker,
            gles::{GlesRenderer, GlesTexture},
        },
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::wayland_server::backend::GlobalId,
    utils::{Size, Transform},
    wayland::dmabuf::DmabufFeedbackBuilder,
};
use tracing::{info, warn};

use super::{Backend, CLEAR, Rendered};
use crate::state::{Isle, State};

pub struct Headless {
    pub renderer: GlesRenderer,
    render_node: Option<DrmNode>,
    targets: HashMap<String, Target>,
}

struct Target {
    texture: GlesTexture,
    damage_tracker: OutputDamageTracker,
    output: Output,
    global: GlobalId,
    drawn: bool,
}

impl Headless {
    /// A GLES renderer on an EGL device with no surface: a GPU's render node when there is
    /// one, else a software device.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut devices: Vec<EGLDevice> = EGLDevice::enumerate()?.collect();
        devices.sort_by_key(|d| d.is_software());
        let device = devices.into_iter().next().ok_or("no EGL device")?;
        let render_node = device.try_get_render_node().ok().flatten();
        info!(
            software = device.is_software(),
            ?render_node,
            "headless EGL device"
        );
        let display = unsafe { EGLDisplay::new(device)? };
        let context = EGLContext::new(&display)?;
        let renderer = unsafe { GlesRenderer::new(context)? };
        Ok(Self {
            renderer,
            render_node,
            targets: HashMap::new(),
        })
    }

    pub fn render(&mut self, isle: &mut Isle, output: &Output) -> Result<Rendered, String> {
        let setting = isle
            .settings
            .output
            .get(&Isle::output_identity(output))
            .map_or(isle_settings::schema::Vrr::Fullscreen, |c| c.vrr);
        crate::redraw::OutputState::of(output).borrow_mut().vrr = crate::present::wants_vrr(
            setting,
            crate::present::VrrCapability::OnDemand,
            isle.full_screen_front(output).is_some(),
        );
        let target = self
            .targets
            .get_mut(&output.name())
            .ok_or_else(|| format!("no target for {}", output.name()))?;
        let age = usize::from(target.drawn);
        let mut framebuffer = self
            .renderer
            .bind(&mut target.texture)
            .map_err(|e| e.to_string())?;
        let mut elements = Vec::new();
        isle.render_elements(&mut self.renderer, output, None, &mut |e| elements.push(e));
        let result = target
            .damage_tracker
            .render_output(&mut self.renderer, &mut framebuffer, age, &elements, CLEAR)
            .map_err(|e| format!("{e:?}"))?;
        let damaged = result.damage.is_some();
        let states = result.states;
        target.drawn = true;
        Ok(Rendered {
            damaged,
            states,
            planes: crate::present::Planes::COMPOSITED,
            retry_in: None,
        })
    }

    pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
        self.renderer.import_dmabuf(dmabuf, None).is_ok()
    }

    /// A virtual display; its make, model and serial key its settings like a real one's.
    pub fn add_output(
        &mut self,
        isle: &mut Isle,
        name: &str,
        (make, model, serial): (&str, &str, &str),
        size: (i32, i32),
    ) -> Result<Output, String> {
        if self.targets.contains_key(name) {
            return Err(format!("{name} exists"));
        }
        let output = Output::new(
            name.to_owned(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: make.into(),
                model: model.into(),
                serial_number: serial.into(),
            },
        );
        let mode = Mode {
            size: size.into(),
            refresh: 60_000,
        };
        output.change_current_state(Some(mode), Some(Transform::Normal), None, None);
        output.set_preferred(mode);
        // A screen that changes its refresh on demand, so the policy runs as on hardware.
        output
            .user_data()
            .insert_if_missing(|| crate::present::VrrCapability::OnDemand);
        let global = output.create_global::<State>(&isle.display_handle);
        let texture = Offscreen::<GlesTexture>::create_buffer(
            &mut self.renderer,
            Fourcc::Abgr8888,
            Size::from(size),
        )
        .map_err(|e| e.to_string())?;
        self.targets.insert(
            name.to_owned(),
            Target {
                texture,
                damage_tracker: OutputDamageTracker::from_output(&output),
                output: output.clone(),
                global,
                drawn: false,
            },
        );
        let x = isle
            .space
            .outputs()
            .filter_map(|o| isle.space.output_geometry(o))
            .map(|g| g.loc.x + g.size.w)
            .max()
            .unwrap_or(0);
        isle.add_output(&output, (x, 0).into());
        Ok(output)
    }

    pub fn remove_output(&mut self, isle: &mut Isle, name: &str) -> bool {
        let Some(target) = self.targets.remove(name) else {
            return false;
        };
        isle.remove_output(&target.output);
        isle.display_handle.remove_global::<State>(target.global);
        true
    }
}

/// Registers the dmabuf global and the first output.
pub fn setup(state: &mut State) -> Result<(), Box<dyn std::error::Error>> {
    let Backend::Headless(headless) = &mut state.backend else {
        unreachable!("setup for the headless backend");
    };
    state
        .isle
        .shm_state
        .update_formats(headless.renderer.shm_formats());
    let formats = headless.renderer.dmabuf_formats();
    match headless.render_node.and_then(|node| {
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
            warn!("no render node; serving dmabuf v3");
            state
                .isle
                .dmabuf_state
                .create_global::<State>(&state.isle.display_handle, formats);
        }
    }
    // Streams' and capture clients' dmabufs come from the render node, as on the hardware.
    if let Some(node) = headless.render_node
        && let Some(path) = node.dev_path()
        && let Ok(file) = std::fs::OpenOptions::new().read(true).write(true).open(&path)
    {
        use smithay::backend::{
            allocator::{format::FormatSet, gbm::GbmDevice},
            drm::DrmDeviceFd,
        };
        let fd = DrmDeviceFd::new(smithay::utils::DeviceFd::from(std::os::fd::OwnedFd::from(file)));
        state.isle.serve_syncobj(fd.clone());
        match GbmDevice::new(fd) {
            Ok(gbm) => {
                let render: FormatSet = headless
                    .renderer
                    .egl_context()
                    .dmabuf_render_formats()
                    .iter()
                    .copied()
                    .collect();
                state.isle.image_copy.set_dma(node, &render);
                state.isle.casting.gbm = Some((gbm, render));
            }
            Err(err) => warn!("no GBM on the render node: {err}"),
        }
    }
    let Backend::Headless(headless) = &mut state.backend else {
        unreachable!()
    };
    headless.add_output(
        &mut state.isle,
        "HEADLESS-1",
        ("Isle", "Headless", "1"),
        (1920, 1080),
    )?;
    Ok(())
}
