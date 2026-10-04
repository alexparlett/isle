//! The shell surface host: Freya trees drawn on the compositor's GL context (B7).

mod a11y;
pub mod boundary;
mod gl;
mod keys;
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
use freya::prelude::{MouseButton, WheelSource};
use freya_embed::{
    AppComponent, EmbedConfig, Embedded, Fonts, Gpu, KeyboardEventName, MouseEventName,
    PlatformEvent, WheelEventName,
};
use smithay::{
    backend::renderer::{
        element::{Kind, texture::TextureRenderElement},
        gles::{GlesRenderer, GlesTexture},
    },
    input::keyboard::{ModifiersState, xkb::Keysym},
    output::Output,
    reexports::calloop::ping::Ping,
    utils::{Logical, Point, Rectangle},
};
pub use surface::{Action, Actions, Layer, Scope, SurfaceDef};
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

/// A shell surface as the seat's pointer or keyboard focus.
#[derive(Clone, Debug)]
pub struct ShellFocus {
    pub id: u64,
    alive: Arc<AtomicBool>,
}

impl PartialEq for ShellFocus {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl ShellFocus {
    pub fn alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }
}

/// What one [`Shell::dispatch`] found.
#[derive(Default)]
pub struct Dispatched {
    pub redraw: Vec<Output>,
    /// Each with the name of the surface that asked and its focus.
    pub actions: Vec<(String, ShellFocus, Action)>,
    /// Surfaces whose pointer icon changed.
    pub cursors: Vec<(ShellFocus, freya::prelude::CursorIcon)>,
    /// Surfaces turned off after panicking too often; the compositor closes them.
    pub disabled: Vec<String>,
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
    next_id: u64,
    modifiers: freya::prelude::Modifiers,
    a11y: a11y::Requests,
    theme: Rc<isle_tokens::Theme>,
}

impl Shell {
    /// `ping` wakes the compositor's loop, which then calls [`Shell::dispatch`].
    pub fn new(ping: Ping, theme: Rc<isle_tokens::Theme>) -> Self {
        // The tokens' families, then fallbacks for emoji and CJK the design leaves to the system.
        let mut families: Vec<Cow<'static, str>> = theme
            .tokens
            .type_family
            .split(',')
            .map(|f| Cow::<str>::Owned(f.trim().to_string()))
            .filter(|f| !f.is_empty())
            .collect();
        families.extend(["Noto Sans", "Noto Color Emoji", "Noto Sans CJK JP"].map(Cow::Borrowed));
        Self {
            theme,
            a11y: a11y::Requests::new(ping.clone()),
            fonts: Fonts::new(families),
            gpu: None,
            defs: Vec::new(),
            instances: Vec::new(),
            outputs: Vec::new(),
            ping,
            released: Vec::new(),
            next_id: 1,
            modifiers: freya::prelude::Modifiers::empty(),
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

    /// Puts `theme` in force in every tree.
    pub fn set_theme(&mut self, theme: Rc<isle_tokens::Theme>) {
        if *theme == *self.theme {
            return;
        }
        self.theme = theme;
        for instance in &mut self.instances {
            let (Some(embedded), Some(state)) = (instance.embedded.as_mut(), instance.theme) else {
                continue;
            };
            let theme = self.theme.clone();
            let ran = boundary::guard(instance.def.name(), || {
                embedded.with_root(|| {
                    let mut state = state;
                    state.set(theme);
                })
            });
            if let Err(message) = ran {
                instance.crash(&message);
            }
            std::task::Wake::wake_by_ref(&instance.wakeup);
        }
        self.recover();
    }

    pub fn theme(&self) -> &Rc<isle_tokens::Theme> {
        &self.theme
    }

    /// The accessibility tree of `name`'s first instance, as JSON.
    pub fn tree(&mut self, name: &str) -> Option<String> {
        let instance = self.instances.iter_mut().find(|i| i.def.name() == name)?;
        let embedded = instance.embedded.as_mut()?;
        let snapshot = match boundary::guard(instance.def.name(), || embedded.accessibility_snapshot())
        {
            Ok(snapshot) => snapshot,
            Err(message) => {
                instance.crash(&message);
                self.recover();
                return None;
            }
        };
        // Taking a snapshot restarts the tree's diff, so an active adapter takes it whole too.
        if let Some(adapter) = instance.adapter.as_mut() {
            let whole = snapshot.clone();
            adapter.update_if_active(|| whole);
        }
        Some(a11y::to_json(&snapshot))
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

    /// Runs the work of every surface whose waker fired.
    pub fn dispatch(&mut self) -> Dispatched {
        let mut out = Dispatched::default();
        for (id, request) in self.a11y.take() {
            let Some(instance) = self.instances.iter_mut().find(|i| i.id == id) else {
                continue;
            };
            let (Some(embedded), Some(adapter)) =
                (instance.embedded.as_mut(), instance.adapter.as_mut())
            else {
                continue;
            };
            let ran = boundary::guard(instance.def.name(), || match request {
                a11y::Request::Activate => {
                    if let Some(tree) = embedded.set_screen_reader(true) {
                        adapter.update_if_active(|| tree);
                    }
                }
                a11y::Request::Deactivate => {
                    embedded.set_screen_reader(false);
                }
                a11y::Request::Action(request) => embedded.accessibility_action(request),
            });
            match ran {
                Ok(()) => std::task::Wake::wake_by_ref(&instance.wakeup),
                Err(message) => instance.crash(&message),
            }
        }
        for instance in &mut self.instances {
            if !instance.wakeup.pending.swap(false, Ordering::AcqRel) {
                continue;
            }
            let Some(embedded) = instance.embedded.as_mut() else {
                continue;
            };
            instance.wakes += 1;
            let fonts = &mut self.fonts;
            let opening = std::mem::take(&mut instance.opening);
            let held = std::mem::take(&mut instance.held_keys);
            let ran = boundary::guard(instance.def.name(), || {
                let mut update = embedded.update(fonts);
                if opening {
                    // The content is mounted now; the keys typed while it opened reach it in
                    // order.
                    for event in held {
                        embedded.event(event);
                    }
                    let more = embedded.update(fonts);
                    update.redraw |= more.redraw;
                    update.cursor = more.cursor.or(update.cursor);
                }
                update
            });
            let mut update = match ran {
                Ok(update) => update,
                Err(message) => {
                    instance.crash(&message);
                    continue;
                }
            };
            if let (Some(tree), Some(adapter)) = (update.accessibility.take(), instance.adapter.as_mut())
            {
                adapter.update_if_active(|| tree);
            }
            if update.redraw && instance.open && !out.redraw.contains(&instance.output) {
                out.redraw.push(instance.output.clone());
            }
            let focus = ShellFocus {
                id: instance.id,
                alive: instance.alive.clone(),
            };
            if let Some(cursor) = update.cursor {
                instance.cursor = cursor;
                out.cursors.push((focus.clone(), cursor));
            }
            for action in instance.actions.borrow_mut().drain(..) {
                out.actions
                    .push((instance.def.name().to_string(), focus.clone(), action));
            }
        }
        out.disabled = self.recover();
        out
    }

    /// Rebuilds every instance that panicked from its definition, or turns it off after its
    /// sixth panic in a minute. Returns the names turned off.
    fn recover(&mut self) -> Vec<String> {
        let mut disabled = Vec::new();
        // A rebuild can panic too; each pass counts one more panic, so this ends.
        while let Some(index) = self.instances.iter().position(|i| i.crashed && !i.disabled) {
            let mut instance = self.instances.remove(index);
            instance.crashed = false;
            let now = std::time::Instant::now();
            instance.panics.push_back(now);
            while instance
                .panics
                .front()
                .is_some_and(|t| now.duration_since(*t) > PANIC_WINDOW)
            {
                instance.panics.pop_front();
            }
            if let Some(target) = instance.gl.take() {
                self.released.push(target);
            }
            if instance.panics.len() > PANIC_LIMIT {
                warn!(
                    surface = instance.def.name(),
                    "panicked {} times in a minute; off until restarted",
                    instance.panics.len()
                );
                instance.disabled = true;
                disabled.push(instance.def.name().to_string());
            } else {
                self.build(&mut instance);
                std::task::Wake::wake_by_ref(&instance.wakeup);
            }
            self.instances.insert(index, instance);
        }
        disabled
    }

    /// Turns a surface turned off by its panics back on, closed, with its count cleared.
    pub fn restart(&mut self, name: &str) -> bool {
        let mut found = false;
        for index in 0..self.instances.len() {
            if self.instances[index].def.name() != name {
                continue;
            }
            found = true;
            let mut instance = self.instances.remove(index);
            instance.disabled = false;
            instance.panics.clear();
            instance.open = false;
            if let Some(target) = instance.gl.take() {
                self.released.push(target);
            }
            self.build(&mut instance);
            self.instances.insert(index, instance);
        }
        self.recover();
        found
    }

    /// Runs `f` in the root scope of `name`'s first instance, behind its boundary.
    pub fn with_root(&mut self, name: &str, f: impl FnOnce()) -> bool {
        let Some(instance) = self.instances.iter_mut().find(|i| i.def.name() == name) else {
            return false;
        };
        let Some(embedded) = instance.embedded.as_mut() else {
            return false;
        };
        if let Err(message) = boundary::guard(instance.def.name(), || embedded.with_root(f)) {
            instance.crash(&message);
        }
        std::task::Wake::wake_by_ref(&instance.wakeup);
        self.recover();
        true
    }

    /// The open surface of `layer` under `at` on `output`, and its origin; both relative to the
    /// output.
    pub fn focus_at(
        &self,
        output: &Output,
        layer: Layer,
        at: Point<f64, Logical>,
    ) -> Option<(ShellFocus, Point<f64, Logical>)> {
        self.instances
            .iter()
            .rev()
            .filter(|i| i.open && &i.output == output && i.def.layer() == layer)
            .find(|i| i.geometry.to_f64().contains(at))
            .map(|i| (Self::focus_of(i), i.geometry.loc.to_f64()))
    }

    /// The focus of `name`'s open instance that takes the keyboard.
    pub fn keyboard_taker(&self, name: &str) -> Option<ShellFocus> {
        self.instances
            .iter()
            .find(|i| i.def.name() == name && i.open && i.def.takes_keyboard())
            .map(Self::focus_of)
    }

    pub fn name_of(&self, focus: &ShellFocus) -> Option<&'static str> {
        self.instances
            .iter()
            .find(|i| i.id == focus.id)
            .map(|i| i.def.name())
    }

    pub fn cursor(&self, focus: &ShellFocus) -> freya::prelude::CursorIcon {
        self.instance(focus)
            .map(|i| i.cursor)
            .unwrap_or_default()
    }

    /// The pointer moved to `at`, relative to the surface.
    pub fn pointer_motion(&mut self, focus: &ShellFocus, at: Point<f64, Logical>) {
        self.feed(focus, |instance| {
            let at = at.to_physical(instance.scale);
            instance.pointer = (at.x, at.y);
            vec![PlatformEvent::Mouse {
                name: MouseEventName::MouseMove,
                cursor: instance.pointer.into(),
                button: None,
            }]
        });
    }

    /// A button by its evdev code.
    pub fn pointer_button(&mut self, focus: &ShellFocus, button: u32, pressed: bool) {
        let button = match button {
            0x110 => MouseButton::Left,
            0x111 => MouseButton::Right,
            0x112 => MouseButton::Middle,
            0x113 => MouseButton::Back,
            0x114 => MouseButton::Forward,
            other => MouseButton::Other(other as u16),
        };
        self.feed(focus, |instance| {
            vec![PlatformEvent::Mouse {
                name: if pressed {
                    MouseEventName::MouseDown
                } else {
                    MouseEventName::MouseUp
                },
                cursor: instance.pointer.into(),
                button: Some(button),
            }]
        });
    }

    /// Scrolling as Wayland measures it: positive is down and right. `discrete` is whether it
    /// came in wheel steps, each `v120` of 120.
    pub fn pointer_axis(&mut self, focus: &ShellFocus, x: f64, y: f64, discrete: Option<(f64, f64)>) {
        let (scroll, source) = match discrete {
            Some((sx, sy)) => (
                (-sx / 120.0 * WheelSource::LINE_SIZE, -sy / 120.0 * WheelSource::LINE_SIZE),
                WheelSource::Line,
            ),
            None => ((-x * 2.0, -y * 2.0), WheelSource::Pixel),
        };
        self.feed(focus, |instance| {
            vec![PlatformEvent::Wheel {
                name: WheelEventName::Wheel,
                scroll: scroll.into(),
                cursor: instance.pointer.into(),
                source,
                timestamp: std::time::Instant::now(),
            }]
        });
    }

    pub fn pointer_leave(&mut self, focus: &ShellFocus) {
        self.feed(focus, |instance| {
            vec![PlatformEvent::PointerExit {
                cursor: instance.pointer.into(),
            }]
        });
    }

    pub fn keyboard_focus(&mut self, focus: &ShellFocus, focused: bool) {
        if let Some(instance) = self.instances.iter_mut().find(|i| i.id == focus.id)
            && let Some(embedded) = instance.embedded.as_mut()
        {
            if let Err(message) =
                boundary::guard(instance.def.name(), || embedded.set_focused(focused))
            {
                instance.crash(&message);
            }
            if let Some(adapter) = instance.adapter.as_mut() {
                adapter.update_window_focus_state(focused);
            }
            std::task::Wake::wake_by_ref(&instance.wakeup);
        }
    }

    pub fn set_modifiers(&mut self, state: &ModifiersState) {
        self.modifiers = keys::modifiers(state);
    }

    /// A key, with the text it types and its evdev code.
    pub fn key(
        &mut self,
        focus: &ShellFocus,
        keysym: Keysym,
        text: Option<String>,
        evdev: u32,
        pressed: bool,
    ) {
        let event = PlatformEvent::Keyboard {
            name: if pressed {
                KeyboardEventName::KeyDown
            } else {
                KeyboardEventName::KeyUp
            },
            key: keys::key(keysym, text),
            code: keys::code(evdev),
            modifiers: self.modifiers,
        };
        let Some(instance) = self.instances.iter_mut().find(|i| i.id == focus.id) else {
            return;
        };
        if instance.opening {
            instance.held_keys.push(event);
            return;
        }
        if let Some(embedded) = instance.embedded.as_mut() {
            if let Err(message) = boundary::guard(instance.def.name(), || embedded.event(event)) {
                instance.crash(&message);
            }
            std::task::Wake::wake_by_ref(&instance.wakeup);
        }
        self.recover();
    }

    fn feed(&mut self, focus: &ShellFocus, events: impl FnOnce(&mut Instance) -> Vec<PlatformEvent>) {
        let Some(instance) = self.instances.iter_mut().find(|i| i.id == focus.id) else {
            return;
        };
        let events = events(instance);
        if let Some(embedded) = instance.embedded.as_mut() {
            let ran = boundary::guard(instance.def.name(), || {
                for event in events {
                    embedded.event(event);
                }
            });
            if let Err(message) = ran {
                instance.crash(&message);
            }
            std::task::Wake::wake_by_ref(&instance.wakeup);
        }
        self.recover();
    }

    fn instance(&self, focus: &ShellFocus) -> Option<&Instance> {
        self.instances.iter().find(|i| i.id == focus.id)
    }

    fn focus_of(instance: &Instance) -> ShellFocus {
        ShellFocus {
            id: instance.id,
            alive: instance.alive.clone(),
        }
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
            let name = instance.def.name();
            let drawn = renderer.with_context(|gl| {
                // Smithay has moved GL state since Skia last drew.
                gpu.context().reset(None);
                let ran = boundary::guard(name, || {
                    embedded.render(fonts, target.target.surface().canvas());
                });
                // Whatever was recorded before a panic still reaches the target it was for.
                gpu.context().flush_and_submit();
                unsafe { gl::restore(gl) };
                ran
            });
            match drawn {
                Err(err) => {
                    warn!(surface = name, "drawing failed: {err}");
                    continue;
                }
                Ok(Err(message)) => {
                    instance.crash(&message);
                    continue;
                }
                Ok(Ok(())) => {}
            }
            let full = smithay::utils::Rectangle::from_size((size.w, size.h).into());
            let _ = target
                .buffer
                .render()
                .draw(|_| Ok::<_, std::convert::Infallible>(vec![full]));
            instance.drew = true;
        }
        self.recover();
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
            if instance.def.name() != name || (open && instance.disabled) {
                continue;
            }
            found = true;
            if instance.open == open {
                continue;
            }
            instance.open = open;
            instance.opening = open;
            instance.held_keys.clear();
            if let (Some(embedded), Some(shown)) = (instance.embedded.as_mut(), instance.shown) {
                let takes_keyboard = instance.def.takes_keyboard();
                let ran = boundary::guard(instance.def.name(), || {
                    embedded.with_root(|| {
                        let mut state = shown.0;
                        state.set(open);
                    });
                    embedded.set_focused(open && takes_keyboard);
                });
                if let Err(message) = ran {
                    instance.crash(&message);
                }
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
        self.recover();
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
                    instance.origin = place.geometry.loc;
                    Self::set_bounds(&mut instance);
                    std::task::Wake::wake_by_ref(&instance.wakeup);
                    kept.push(instance);
                }
                None => {
                    debug!(surface = instance.def.name(), "instance removed");
                    instance.alive.store(false, Ordering::Release);
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
        let id = self.next_id;
        self.next_id += 1;
        let mut instance = Instance {
            id,
            alive: Arc::new(AtomicBool::new(true)),
            actions: Default::default(),
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
            crashed: false,
            wakes: 0,
            pointer: (0.0, 0.0),
            cursor: Default::default(),
            opening: false,
            held_keys: Vec::new(),
            adapter: None,
            theme: None,
            origin: place.geometry.loc,
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
        let actions = Actions(instance.actions.clone());
        let theme = self.theme.clone();
        let mut theme_state = None;
        let fonts = &mut self.fonts;
        let built = boundary::guard(def.name(), || Embedded::new(
            EmbedConfig {
                app: AppComponent::new(Root(def.app())),
                size: (size.w as f32, size.h as f32).into(),
                scale_factor: instance.scale,
                name: def.name().to_string(),
                waker: Waker::from(instance.wakeup.clone()),
                preferred_theme: PreferredTheme::Dark,
                clipboard: None,
            },
            fonts,
            |runner| {
                shown = Some(runner.provide_root_context(|| Shown(State::create(open))));
                runner.provide_root_context(|| actions);
                theme_state = Some(
                    runner
                        .provide_root_context(|| isle_ui::ThemeContext(State::create(theme)))
                        .0,
                );
                def.provide(runner);
            },
        ));
        let mut embedded = match built {
            Ok(embedded) => embedded,
            Err(message) => {
                instance.crash(&message);
                return;
            }
        };
        if open
            && let Err(message) = boundary::guard(def.name(), || {
                embedded.set_focused(def.takes_keyboard());
            })
        {
            instance.crash(&message);
            return;
        }
        instance.embedded = Some(embedded);
        instance.shown = shown;
        instance.theme = theme_state;
        instance.adapter = Some(self.a11y.adapter(instance.id));
        Self::set_bounds(instance);
    }

    fn set_bounds(instance: &mut Instance) {
        let Some(adapter) = instance.adapter.as_mut() else {
            return;
        };
        let at = instance.geometry.loc + instance.origin;
        let rect = accesskit::Rect {
            x0: at.x as f64,
            y0: at.y as f64,
            x1: (at.x + instance.geometry.size.w) as f64,
            y1: (at.y + instance.geometry.size.h) as f64,
        };
        adapter.set_root_window_bounds(rect, rect);
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

/// More panics than this within [`PANIC_WINDOW`] turn a surface off.
const PANIC_LIMIT: usize = 5;
const PANIC_WINDOW: std::time::Duration = std::time::Duration::from_secs(60);

impl Instance {
    /// Drops a tree that panicked; [`Shell::recover`] rebuilds it.
    fn crash(&mut self, message: &str) {
        warn!(surface = self.def.name(), "panicked: {message}");
        self.crashed = true;
        self.shown = None;
        self.adapter = None;
        self.opening = false;
        self.held_keys.clear();
        if let Some(embedded) = self.embedded.take() {
            // A tree that panicked can panic again as it is dropped; whatever is left leaks.
            let _ = boundary::guard(self.def.name(), move || drop(embedded));
        }
    }
}

/// Skia's resource cache budget for every surface together, until 3.6 measures it.
const CACHE_LIMIT: usize = 64 * 1024 * 1024;

