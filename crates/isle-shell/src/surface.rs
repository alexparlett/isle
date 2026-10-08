//! What a shell surface is: its definition, and one instance of it on an output.

use std::{
    cell::RefCell,
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
use freya_embed::{AppComponent, Embedded};
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
    /// A system prompt: over everything a client can show and the shell's overlays, with the
    /// compositor's scrim under it; under the lock.
    Prompt,
    /// The lock's own, drawn only while the session is locked and only by the lock's path.
    Lock,
}

/// How many instances a surface has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    EveryOutput,
    MainOutput,
    /// Made, placed and removed by the compositor for something it owns, such as a window's
    /// title bar, and drawn with it rather than in a layer.
    Attached,
    /// One instance, in its layer, opened by the compositor on the output and at the box it
    /// gives: a menu by a window.
    Placed,
}

/// A kind of shell surface. The definition outlives every tree drawn from it, so state that
/// must survive a rebuild (shell.md rule 3) lives here and the tree it makes holds it.
pub trait SurfaceDef: 'static {
    fn name(&self) -> &'static str;
    fn layer(&self) -> Layer;
    fn scope(&self) -> Scope;
    /// Whether the surface takes the keyboard while it is open.
    fn takes_keyboard(&self) -> bool;
    /// The surface's box on an output of `output` logical size, relative to the output.
    fn geometry(&self, output: Size<i32, Logical>) -> Rectangle<i32, Logical>;
    /// The tree. State that must survive a rebuild is captured in it from the definition.
    fn app(&self) -> AppComponent;
    /// The point it opens from and closes toward, as fractions of its box.
    fn anchor(&self) -> (f64, f64) {
        (0.5, 0.5)
    }
    /// Its corner radius in points; its texture is clipped to it.
    fn corner_radius(&self, _theme: &isle_tokens::Theme) -> f32 {
        0.0
    }
    /// Each corner's radius in points: top left, top right, bottom right, bottom left.
    fn corners(&self, theme: &isle_tokens::Theme) -> [f32; 4] {
        [self.corner_radius(theme); 4]
    }
    /// The pointer reaches it only within the shapes its tree marks with `isle_ui::Hit`.
    fn hit_shapes(&self) -> bool {
        false
    }
    /// It opens and closes with the compositor's transitions; one that moves by its own
    /// springs appears and goes at once.
    fn transitions(&self) -> bool {
        true
    }
    /// Its transitions are fades in place, with no scale.
    fn fades(&self) -> bool {
        false
    }
}

/// The output a surface's instance is on, by name, for a tree drawn on every output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Here(pub String);

/// What a surface asks of the compositor, which acts on it after the surface's update.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Close this surface; keyboard focus goes back first.
    Close,
    /// Press and release a key, by evdev code, in whatever then has the keyboard.
    Key(u32),
    /// For the window an attached surface belongs to.
    Window(WindowAction),
    /// From the zoom menu, for the window it was opened on.
    Zone(crate::zones::ZoneAction),
    /// From the island on an output.
    Island(crate::island::IslandAction),
    /// From the menu bar on an output.
    MenuBar(crate::menubar::BarAction),
    /// From a menu in a placed surface: the window menu.
    Menu(isle_ui::MenuEvent),
    /// From the switcher's panel.
    Switcher(crate::switcher::SwitcherAction),
    /// From the notification banners.
    Banner(crate::banners::BannerAction),
    /// From the lock's face.
    Lock(crate::lock::LockAction),
    /// From a system prompt.
    Prompt(crate::prompts::PromptAction),
    /// From the session dialog.
    Session(crate::prompts::session::SessionAction),
}

/// What a title bar asks of its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowAction {
    Close,
    /// Option-click on close: every window of the app.
    CloseApp,
    Minimise,
    /// Option-click on minimise: every window of the app.
    MinimiseApp,
    /// Full screen, or out of it.
    Zoom,
    /// Option-click on zoom: fill, or back.
    Fill,
    /// The double-click action from settings.
    TitlebarAction,
    /// A press on the bar that may become a drag; `raise` is false with Cmd held.
    Move { raise: bool },
    /// Secondary click: the window menu.
    Menu,
}

/// The tree's way to ask for an [`Action`].
#[derive(Clone)]
pub struct Actions(pub(crate) Rc<RefCell<Vec<Action>>>);

impl Actions {
    pub fn get() -> Self {
        use_consume::<Actions>()
    }

    pub fn send(&self, action: Action) {
        self.0.borrow_mut().push(action);
    }
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
    pub id: u64,
    /// Cleared when the instance goes, so a focus that names it is no longer alive.
    pub alive: Arc<AtomicBool>,
    pub actions: Rc<RefCell<Vec<Action>>>,
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
    /// Panicked since the shell last recovered its instances.
    pub crashed: bool,
    /// Updates run because its waker fired.
    pub wakes: u64,
    /// Times its tree was drawn into its texture.
    pub draws: u64,
    /// Draw times in microseconds while the shell measures.
    pub draw_times: Vec<u64>,
    /// Glass passes that blurred, in microseconds, while the shell measures.
    pub glass_times: Vec<u64>,
    pub transition: crate::transition::Transition,
    /// The pointer's last place over the surface, in physical pixels from its origin.
    pub pointer: (f64, f64),
    pub cursor: freya::prelude::CursorIcon,
    /// Opened but not yet updated: keys wait until the content is mounted.
    pub opening: bool,
    pub held_keys: Vec<freya_embed::PlatformEvent>,
    pub adapter: Option<accesskit_unix::Adapter>,
    pub theme: Option<State<Rc<isle_tokens::Theme>>>,
    pub glass: isle_ui::GlassShapes,
    pub hits: isle_ui::HitShapes,
    /// Each glass shape's blur, by the shape's id.
    pub blurs: std::collections::BTreeMap<u64, crate::blur::Blur>,
    /// The output's origin in the global space, for the adapter's window bounds.
    pub origin: smithay::utils::Point<i32, Logical>,
    pub disabled: bool,
    /// The compositor's key for an attached instance.
    pub attached: Option<u64>,
}
