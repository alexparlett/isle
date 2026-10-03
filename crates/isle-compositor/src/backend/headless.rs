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
        isle.render_elements(&mut self.renderer, output, &mut |e| elements.push(e));
        let result = target
            .damage_tracker
            .render_output(&mut self.renderer, &mut framebuffer, age, &elements, CLEAR)
            .map_err(|e| format!("{e:?}"))?;
        let damaged = result.damage.is_some();
        let states = result.states;
        target.drawn = true;
        Ok(Rendered { damaged, states })
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
    headless.add_output(
        &mut state.isle,
        "HEADLESS-1",
        ("Isle", "Headless", "1"),
        (1920, 1080),
    )?;
    Ok(())
}
