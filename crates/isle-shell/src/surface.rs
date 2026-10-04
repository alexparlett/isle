//! What a shell surface is: its definition, and one instance of it on an output.

use std::{
    collections::VecDeque,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::Wake,
    time::Instant,
};

use freya::prelude::*;
use freya_embed::{AppComponent, Embedded, Runner};
use smithay::{
    output::Output,
    reexports::calloop::ping::Ping,
    utils::{Logical, Rectangle, Size},
};

use crate::gl::GlSurface;

/// Where a surface stacks among windows and layer-shell clients.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// Under every window and layer-shell surface: the wallpaper.
    Background,
    /// Over windows and layer-shell's top layer, under its overlays.
    Top,
    /// Over everything a client can put on screen, overlays included (shell.md rule 13).
    Overlay,
}

/// How many instances a surface has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    EveryOutput,
    MainOutput,
}

/// A kind of shell surface. The definition outlives every tree drawn from it, so state that
/// must survive a rebuild (shell.md rule 3) lives here and reaches the tree through `provide`.
pub trait SurfaceDef: 'static {
    fn name(&self) -> &'static str;
    fn layer(&self) -> Layer;
    fn scope(&self) -> Scope;
    /// Whether the surface takes the keyboard while it is open.
    fn takes_keyboard(&self) -> bool;
    /// The surface's box on an output of `output` logical size, relative to the output.
    fn geometry(&self, output: Size<i32, Logical>) -> Rectangle<i32, Logical>;
    fn app(&self) -> AppComponent;
    /// Root contexts the tree reads, provided before its first render.
    fn provide(&self, runner: &mut Runner);
}

/// Whether the surface's content is mounted; a closed surface mounts none (shell.md rule 7).
#[derive(Clone, Copy)]
pub(crate) struct Shown(pub State<bool>);

/// The root every surface's tree hangs from.
#[derive(Clone)]
pub(crate) struct Root(pub AppComponent);

impl App for Root {
    fn render(&self) -> impl IntoElement {
        let shown = use_consume::<Shown>();
        let content = self.0.clone();
        rect()
            .expanded()
            .maybe_child(shown.0.read().then(move || content.into_element()))
    }
}

/// Wakes the loop for one instance: the flag says which, the ping says when.
pub(crate) struct Wakeup {
    pub pending: AtomicBool,
    pub ping: Ping,
}

impl Wake for Wakeup {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if !self.pending.swap(true, Ordering::AcqRel) {
            self.ping.ping();
        }
    }
}

pub(crate) struct Instance {
    pub def: Rc<dyn SurfaceDef>,
    pub output: Output,
    pub embedded: Option<Embedded>,
    pub shown: Option<Shown>,
    pub open: bool,
    pub wakeup: Arc<Wakeup>,
    /// Relative to the output.
    pub geometry: Rectangle<i32, Logical>,
    pub scale: f64,
    pub gl: Option<GlSurface>,
    /// Drawn since the output last presented, so its animations wait on that present.
    pub drew: bool,
    pub panics: VecDeque<Instant>,
    /// Updates run because its waker fired.
    pub wakes: u64,
    pub disabled: bool,
}
