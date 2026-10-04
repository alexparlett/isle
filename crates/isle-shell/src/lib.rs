//! The shell surface host: Freya trees drawn on the compositor's GL context (B7).

mod gl;
#[cfg(feature = "probe")]
pub mod probe;
mod surface;

use std::{
    borrow::Cow,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::Waker,
};

use freya::prelude::{PreferredTheme, State, WritableUtils};
pub use freya_embed;
use freya_embed::{AppComponent, EmbedConfig, Embedded, Fonts, Gpu};
use smithay::{
    backend::renderer::{
        element::{Kind, texture::TextureRenderElement},
        gles::{GlesRenderer, GlesTexture},
    },
    output::Output,
    reexports::calloop::ping::Ping,
    utils::{Logical, Rectangle},
};
pub use surface::{Layer, Scope, SurfaceDef};
use surface::{Instance, Root, Shown, Wakeup};
use tracing::{debug, warn};

/// What the compositor tells the shell about one output.
#[derive(Clone)]
pub struct OutputPlace {
    pub output: Output,
    /// The output's box in the global space.
    pub geometry: Rectangle<i32, Logical>,
    pub scale: f64,
}

/// A surface as `Shell1` lists it.
#[derive(Clone, Debug)]
pub struct SurfaceInfo {
    pub name: String,
    pub output: String,
    pub open: bool,
    /// Relative to the output, logical.
    pub geometry: Rectangle<i32, Logical>,
    pub panics: u32,
    pub disabled: bool,
    /// Updates run because the surface's waker fired.
    pub wakes: u64,
}

/// Every shell surface, and what they share: fonts and one Skia context.
pub struct Shell {
    fonts: Fonts,
    gpu: Option<Gpu>,
    defs: Vec<Rc<dyn SurfaceDef>>,
    instances: Vec<Instance>,
    outputs: Vec<OutputPlace>,
    ping: Ping,
    /// GL objects of released targets, freed at the next draw with the context current.
    released: Vec<gl::GlSurface>,
}

impl Shell {
    /// `ping` wakes the compositor's loop, which then calls [`Shell::dispatch`].
    pub fn new(ping: Ping, families: Vec<Cow<'static, str>>) -> Self {
        Self {
            fonts: Fonts::new(families),
            gpu: None,
            defs: Vec::new(),
            instances: Vec::new(),
            outputs: Vec::new(),
            ping,
            released: Vec::new(),
        }
    }

    pub fn register(&mut self, def: Rc<dyn SurfaceDef>) {
        self.defs.push(def);
        self.reconcile();
    }

    /// The outputs as they now are; the first is the main one.
    pub fn set_outputs(&mut self, outputs: Vec<OutputPlace>) {
        self.outputs = outputs;
        self.reconcile();
    }

    /// Opens every instance of `name`; false if no surface has that name.
    pub fn open(&mut self, name: &str) -> bool {
        self.set_open(name, true)
    }

    pub fn close(&mut self, name: &str) -> bool {
        self.set_open(name, false)
    }

    pub fn is_open(&self, name: &str) -> bool {
        self.instances
            .iter()
            .any(|i| i.def.name() == name && i.open)
    }

    pub fn surfaces(&self) -> Vec<SurfaceInfo> {
        self.instances
            .iter()
            .map(|i| SurfaceInfo {
                name: i.def.name().to_string(),
                output: i.output.name(),
                open: i.open,
                geometry: i.geometry,
                panics: i.panics.len() as u32,
                disabled: i.disabled,
                wakes: i.wakes,
            })
            .collect()
    }

    /// Runs the work of every surface whose waker fired; returns the outputs to redraw.
    pub fn dispatch(&mut self) -> Vec<Output> {
        let mut redraw = Vec::new();
        for instance in &mut self.instances {
            if !instance.wakeup.pending.swap(false, Ordering::AcqRel) {
                continue;
            }
            let Some(embedded) = instance.embedded.as_mut() else {
                continue;
            };
            instance.wakes += 1;
            let update = embedded.update(&mut self.fonts);
            if update.redraw && instance.open && !redraw.contains(&instance.output) {
                redraw.push(instance.output.clone());
            }
        }
        redraw
    }

    /// Draws every open surface on `output` whose tree changed, before the output's frame.
    pub fn draw(&mut self, renderer: &mut GlesRenderer, output: &Output) {
        for released in self.released.drain(..) {
            released.release(renderer);
        }
        if !self
            .instances
            .iter()
            .any(|i| i.open && &i.output == output)
        {
            return;
        }
        let Some(gpu) = Self::gpu(&mut self.gpu, renderer) else {
            return;
        };
        for instance in &mut self.instances {
            if !instance.open || &instance.output != output {
                continue;
            }
            let Some(embedded) = instance.embedded.as_mut() else {
                continue;
            };
            let size = instance
                .geometry
                .size
                .to_f64()
                .to_physical(instance.scale)
                .to_i32_round();
            if instance.gl.as_ref().is_some_and(|gl| gl.size != size) {
                self.released.push(instance.gl.take().unwrap());
            }
            let fresh = instance.gl.is_none();
            if fresh {
                debug!(surface = instance.def.name(), ?size, scale = instance.scale, "new target");
                match gl::GlSurface::new(renderer, gpu, size) {
                    Ok(target) => instance.gl = Some(target),
                    Err(err) => {
                        warn!(surface = instance.def.name(), "no drawing target: {err}");
                        continue;
                    }
                }
                embedded.resize((size.w as f32, size.h as f32).into(), instance.scale);
                embedded.update(&mut self.fonts);
            }
            if !embedded.needs_render() && !fresh {
                continue;
            }
            let target = instance.gl.as_mut().unwrap();
            let fonts = &mut self.fonts;
            let drawn = renderer.with_context(|gl| {
                // Smithay has moved GL state since Skia last drew.
                gpu.context().reset(None);
                embedded.render(fonts, target.target.surface().canvas());
                gpu.context().flush_and_submit();
                unsafe { gl::restore(gl) };
            });
            if let Err(err) = drawn {
                warn!(surface = instance.def.name(), "drawing failed: {err}");
                continue;
            }
            let full = smithay::utils::Rectangle::from_size((size.w, size.h).into());
            let _ = target
                .buffer
                .render()
                .draw(|_| Ok::<_, std::convert::Infallible>(vec![full]));
            instance.drew = true;
        }
    }

    /// The surfaces of `layer` on `output`, front to back, positioned relative to the output.
    pub fn elements(
        &self,
        output: &Output,
        layer: Layer,
        push: &mut dyn FnMut(TextureRenderElement<GlesTexture>),
    ) {
        for instance in self.instances.iter().rev() {
            if !instance.open || &instance.output != output || instance.def.layer() != layer {
                continue;
            }
            let Some(target) = &instance.gl else {
                continue;
            };
            let location = instance.geometry.loc.to_f64().to_physical(instance.scale);
            push(TextureRenderElement::from_texture_render_buffer(
                location,
                &target.buffer,
                None,
                None,
                Some(instance.geometry.size),
                Kind::Unspecified,
            ));
        }
    }

    /// The output presented a frame: the surfaces drawn into it advance their animations.
    pub fn presented(&mut self, output: &Output) {
        for instance in &mut self.instances {
            if instance.drew && &instance.output == output {
                instance.drew = false;
                if let Some(embedded) = instance.embedded.as_mut() {
                    embedded.presented();
                }
            }
        }
    }

    fn gpu<'a>(slot: &'a mut Option<Gpu>, renderer: &mut GlesRenderer) -> Option<&'a mut Gpu> {
        if slot.is_none() {
            let made = renderer.with_context(|gl| {
                // Safety: the compositor's context is current; the shell abandons the Skia
                // context when it is dropped, which is after the renderer.
                let gpu = unsafe {
                    Gpu::new(|name| smithay::backend::egl::get_proc_address(name) as *const _)
                };
                unsafe { gl::restore(gl) };
                gpu
            });
            match made {
                Ok(Some(mut gpu)) => {
                    gpu.set_cache_limit(CACHE_LIMIT);
                    *slot = Some(gpu);
                }
                Ok(None) => warn!("Skia could not use the compositor's GL context"),
                Err(err) => warn!("no GL context for Skia: {err}"),
            }
        }
        slot.as_mut()
    }

    fn set_open(&mut self, name: &str, open: bool) -> bool {
        let mut found = false;
        for instance in &mut self.instances {
            if instance.def.name() != name || instance.disabled {
                continue;
            }
            found = true;
            if instance.open == open {
                continue;
            }
            instance.open = open;
            if let (Some(embedded), Some(shown)) = (instance.embedded.as_mut(), instance.shown) {
                embedded.with_root(|| {
                    let mut state = shown.0;
                    state.set(open);
                });
                embedded.set_focused(open && instance.def.takes_keyboard());
            }
            if !open {
                if let Some(target) = instance.gl.take() {
                    self.released.push(target);
                }
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.purge();
                }
            }
            std::task::Wake::wake_by_ref(&instance.wakeup);
        }
        found
    }

    /// Makes the set of instances match the definitions and outputs.
    fn reconcile(&mut self) {
        let main = self.outputs.first().cloned();
        let mut wanted: Vec<(Rc<dyn SurfaceDef>, OutputPlace)> = Vec::new();
        for def in &self.defs {
            match def.scope() {
                Scope::EveryOutput => {
                    for place in &self.outputs {
                        wanted.push((def.clone(), place.clone()));
                    }
                }
                Scope::MainOutput => {
                    if let Some(place) = &main {
                        wanted.push((def.clone(), place.clone()));
                    }
                }
            }
        }

        let mut kept = Vec::new();
        for mut instance in std::mem::take(&mut self.instances) {
            let index = wanted.iter().position(|(def, place)| {
                Rc::ptr_eq(def, &instance.def)
                    && (def.scope() == Scope::MainOutput || place.output == instance.output)
            });
            match index {
                Some(index) => {
                    let (_, place) = wanted.remove(index);
                    instance.output = place.output.clone();
                    instance.geometry = instance.def.geometry(place.geometry.size);
                    instance.scale = place.scale;
                    std::task::Wake::wake_by_ref(&instance.wakeup);
                    kept.push(instance);
                }
                None => {
                    debug!(surface = instance.def.name(), "instance removed");
                    if let Some(target) = instance.gl.take() {
                        self.released.push(target);
                    }
                }
            }
        }
        for (def, place) in wanted {
            kept.push(self.instantiate(def, &place));
        }
        self.instances = kept;
    }

    fn instantiate(&mut self, def: Rc<dyn SurfaceDef>, place: &OutputPlace) -> Instance {
        let wakeup = Arc::new(Wakeup {
            pending: AtomicBool::new(false),
            ping: self.ping.clone(),
        });
        let geometry = def.geometry(place.geometry.size);
        let mut instance = Instance {
            def,
            output: place.output.clone(),
            embedded: None,
            shown: None,
            open: false,
            wakeup,
            geometry,
            scale: place.scale,
            gl: None,
            drew: false,
            panics: Default::default(),
            wakes: 0,
            disabled: false,
        };
        self.build(&mut instance);
        instance
    }

    /// Builds the instance's tree from its definition.
    fn build(&mut self, instance: &mut Instance) {
        let size = instance
            .geometry
            .size
            .to_f64()
            .to_physical(instance.scale)
            .to_i32_round::<i32>();
        let def = instance.def.clone();
        let open = instance.open;
        let mut shown = None;
        let embedded = Embedded::new(
            EmbedConfig {
                app: AppComponent::new(Root(def.app())),
                size: (size.w as f32, size.h as f32).into(),
                scale_factor: instance.scale,
                name: def.name().to_string(),
                waker: Waker::from(instance.wakeup.clone()),
                preferred_theme: PreferredTheme::Dark,
                clipboard: None,
            },
            &mut self.fonts,
            |runner| {
                shown = Some(runner.provide_root_context(|| Shown(State::create(open))));
                def.provide(runner);
            },
        );
        instance.embedded = Some(embedded);
        instance.shown = shown;
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        // The renderer and its context may already be gone; Skia must not touch them.
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.abandon();
        }
    }
}

/// Skia's resource cache budget for every surface together, until 3.6 measures it.
const CACHE_LIMIT: usize = 64 * 1024 * 1024;

