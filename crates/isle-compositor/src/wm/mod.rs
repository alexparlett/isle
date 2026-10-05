//! Every toplevel the compositor manages, from its creation to its destruction. Smithay's
//! `Space` holds only the ones shown now; the rest keep their record here.

mod apps;
mod bars;
pub mod desktops;
pub mod draw;
pub mod motion;
pub mod geometry;
mod places;
mod tiling;
mod zoom;

pub use apps::watch as watch_apps;
pub use tiling::ZonePreview;
pub use zoom::ZoomState;

use isle_ipc::WindowInfo;
use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{Resource, protocol::wl_surface::WlSurface},
    },
    utils::{IsAlive, Logical, Point, Rectangle},
    wayland::{compositor::with_states, shell::xdg::XdgToplevelSurfaceData},
};

use crate::{
    state::{Isle, State},
    worker::Event,
};

/// What a window does with its box. A tile and a fill keep the frame they came from.
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Floating,
    Tiled {
        zone: geometry::Zone,
        restore: Rectangle<i32, Logical>,
    },
    /// Maximised: the work area.
    Filled { restore: Rectangle<i32, Logical> },
    /// On a desktop of its own; it returns to `origin` with `previous` and `restore`.
    FullScreen {
        origin: u64,
        restore: Rectangle<i32, Logical>,
        previous: Box<Mode>,
        /// It covers its output by itself and was never told it is full screen.
        borderless: bool,
    },
}

impl Mode {
    pub fn name(&self) -> String {
        match self {
            Mode::Floating => "floating".into(),
            Mode::Tiled { zone, .. } => format!("tiled:{}", zone.name()),
            Mode::Filled { .. } => "filled".into(),
            Mode::FullScreen { .. } => "fullscreen".into(),
        }
    }
}

pub struct Record {
    pub id: u64,
    pub window: Window,
    pub app: String,
    pub pid: Option<i32>,
    pub parent: Option<u64>,
    /// A modal dialog of its parent (xdg-dialog), which keeps the parent from taking focus.
    pub modal: bool,
    /// The remembered box it was asked to open at, applied at map.
    pub planned: Option<Rectangle<i32, Logical>>,
    /// Its place is recorded once it commits at this size.
    pub record_at_size: Option<smithay::utils::Size<i32, Logical>>,
    pub mode: Mode,
    pub minimised: bool,
    /// Its app is hidden.
    pub hidden: bool,
    /// The desktop it is on; 0 until it maps.
    pub desktop: u64,
    /// It asked to be full screen before it mapped.
    pub wants_full_screen: bool,
    /// The output identity and desktop it left when its output went.
    pub home: Option<(String, u64, Point<i32, Logical>)>,
    /// Placed after its first buffer; until then it is not shown.
    pub mapped: bool,
    /// It asked for focus without a fresh activation token.
    pub attention: bool,
    /// Its client missed a ping and has not answered since.
    pub not_responding: bool,
    /// Drawn away from its box while it opens, comes back or slides in.
    pub motion: Option<motion::WindowMotion>,
    /// Its frame moving to a new box.
    pub morph: Option<motion::Morph>,
    /// It asked the compositor to draw its decorations.
    pub server_side: bool,
    /// How it negotiates decorations, for when title bars are turned on or off.
    pub decoration: Option<Decoration>,
    /// Isle's bar over it.
    pub bar: Option<bars::Bar>,
    /// Its box in the global space: where it is while shown, where it returns to while not.
    pub rect: Rectangle<i32, Logical>,
    suspended: bool,
    /// It is a game: translation, the swap and all but the escape set are off over it.
    pub game: bool,
    /// Its process carries Steam's game label.
    pub steam: bool,
}

impl Record {
    pub fn shown(&self) -> bool {
        self.mapped && !self.minimised && !self.hidden
    }

    /// Its Wayland surface; an X11 window has one once Xwayland has associated it.
    pub fn surface(&self) -> Option<WlSurface> {
        use smithay::wayland::seat::WaylandFocus;
        self.window.wl_surface().map(|s| s.into_owned())
    }

    pub fn x11(&self) -> Option<&smithay::xwayland::X11Surface> {
        self.window.x11_surface()
    }

    /// The toplevel's app id and title; an X11 window's class and title.
    pub fn names(&self) -> (String, String) {
        if let Some(x11) = self.x11() {
            return (x11.class(), x11.title());
        }
        let Some(surface) = self.surface() else {
            return Default::default();
        };
        with_states(&surface, |states| {
            states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .and_then(|d| d.lock().ok())
                .map(|d| {
                    (
                        d.app_id.clone().unwrap_or_default(),
                        d.title.clone().unwrap_or_default(),
                    )
                })
                .unwrap_or_default()
        })
    }

    pub fn send_close(&self) {
        if let Some(toplevel) = self.window.toplevel() {
            toplevel.send_close();
        }
        if let Some(x11) = self.x11() {
            let _ = x11.close();
        }
    }

    /// Asks the client to draw at `size`. An X11 window is told its place too, from its
    /// record, which the move that follows corrects.
    pub fn configure_size(&self, size: smithay::utils::Size<i32, Logical>) {
        if let Some(x11) = self.x11() {
            let _ = x11.configure(Rectangle::new(self.rect.loc, size));
        }
        if let Some(toplevel) = self.window.toplevel() {
            toplevel.with_pending_state(|state| state.size = Some(size));
            if toplevel.is_initial_configure_sent() {
                toplevel.send_pending_configure();
            }
        }
    }

    fn set_suspended(&mut self, suspended: bool) {
        if self.suspended == suspended {
            return;
        }
        self.suspended = suspended;
        if let Some(toplevel) = self.window.toplevel() {
            toplevel.with_pending_state(|state| {
                if suspended {
                    state.states.set(xdg_toplevel::State::Suspended);
                } else {
                    state.states.unset(xdg_toplevel::State::Suspended);
                }
            });
            if toplevel.is_initial_configure_sent() {
                toplevel.send_pending_configure();
            }
        }
    }
}

/// A Wayland window's decoration protocol and what the client asked for.
#[derive(Clone, Debug)]
pub enum Decoration {
    Xdg(Option<smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode>),
    Kde {
        object: smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration::OrgKdeKwinServerDecoration,
        requested: smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration::Mode,
    },
}

/// A window's id, stable for its life and never reused.
struct WindowId(u64);

#[derive(Default)]
pub struct Wm {
    /// Bottom to top, windows not shown among them, so each comes back at its place in the
    /// stack.
    pub records: Vec<Record>,
    /// Ids, most recently focused first.
    pub recent: Vec<u64>,
    next_id: u64,
    /// The desktops shown now, one per output.
    pub current: std::collections::HashSet<u64>,
}

impl Wm {
    pub fn get(&self, id: u64) -> Option<&Record> {
        self.records.iter().find(|r| r.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Record> {
        self.records.iter_mut().find(|r| r.id == id)
    }

    pub fn of(&self, window: &Window) -> Option<&Record> {
        self.get(window_id(window))
    }

    pub fn for_surface(&self, surface: &WlSurface) -> Option<&Record> {
        self.records.iter().find(|r| r.surface().as_ref() == Some(surface))
    }

    /// Moves `id` and the windows it is the parent of to the top of the stack.
    pub fn raise(&mut self, id: u64) {
        let mut moved = Vec::new();
        let mut take = vec![id];
        while let Some(next) = take.pop() {
            if let Some(index) = self.records.iter().position(|r| r.id == next) {
                let record = self.records.remove(index);
                take.extend(
                    self.records
                        .iter()
                        .filter(|r| r.parent == Some(next))
                        .map(|r| r.id),
                );
                moved.push(record);
            }
        }
        self.records.extend(moved);
    }

    pub fn touch(&mut self, id: u64) {
        self.recent.retain(|r| *r != id);
        self.recent.insert(0, id);
    }

    /// The shown modal dialog of `id` that takes its focus, deepest first.
    pub fn modal_of(&self, id: u64) -> Option<u64> {
        let child = self
            .records
            .iter()
            .rev()
            .find(|r| r.parent == Some(id) && r.modal && self.is_shown(r.id))?;
        Some(self.modal_of(child.id).unwrap_or(child.id))
    }

    /// On screen: mapped, neither minimised nor hidden, on a desktop shown now, and its
    /// parent too, since a dialog goes with its parent.
    pub fn is_shown(&self, id: u64) -> bool {
        let Some(record) = self.get(id) else {
            return false;
        };
        record.shown()
            && (record.desktop == 0 || self.current.contains(&record.desktop))
            && record.parent.is_none_or(|p| self.is_shown(p))
    }

    /// The most recently focused window still shown.
    pub fn most_recent_shown(&self) -> Option<&Record> {
        self.recent
            .iter()
            .filter_map(|id| self.get(*id))
            .find(|r| self.is_shown(r.id))
            .or_else(|| self.records.iter().rev().find(|r| self.is_shown(r.id)))
    }
}

pub fn window_id(window: &Window) -> u64 {
    window.user_data().get::<WindowId>().map_or(0, |id| id.0)
}

impl Isle {
    pub fn notify(&self, event: Event) {
        let _ = self.events.send(event);
    }

    /// Takes a new toplevel into the registry; it is shown once it has a buffer and a place.
    pub fn add_window(&mut self, window: Window) -> u64 {
        self.wm.next_id += 1;
        let id = self.wm.next_id;
        window.user_data().insert_if_missing(|| WindowId(id));
        let pid = window
            .toplevel()
            .and_then(|t| self.display_handle.get_client(t.wl_surface().id()).ok())
            .and_then(|c| c.get_credentials(&self.display_handle).ok())
            .map(|c| c.pid);
        self.wm.records.push(Record {
            id,
            window,
            app: String::new(),
            pid,
            parent: None,
            modal: false,
            planned: None,
            record_at_size: None,
            mode: Mode::Floating,
            minimised: false,
            hidden: false,
            desktop: 0,
            wants_full_screen: false,
            home: None,
            mapped: false,
            attention: false,
            not_responding: false,
            motion: None,
            morph: None,
            server_side: false,
            decoration: None,
            bar: None,
            rect: Rectangle::default(),
            suspended: false,
            game: false,
            steam: false,
        });
        id
    }

    pub fn window_by_id(&self, id: u64) -> Option<Window> {
        self.wm.get(id).map(|r| r.window.clone())
    }

    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.wm.for_surface(surface).map(|r| r.window.clone())
    }

    /// Makes the `Space` show what the registry says is shown, in its stacking order.
    pub fn sync_space(&mut self) {
        let present: Vec<String> = self.space.outputs().map(Self::output_identity).collect();
        self.wm.current = self
            .desktops
            .rows
            .iter()
            .filter(|(identity, _)| present.contains(identity))
            .filter_map(|(_, row)| row.desktops.get(row.current).map(|d| d.id))
            .collect();
        for record in &mut self.wm.records {
            if let Some(loc) = self.space.element_location(&record.window) {
                record.rect = Rectangle::new(loc, record.window.geometry().size);
            }
        }
        let mut changed = false;
        let parked: Vec<u64> = self
            .wm
            .records
            .iter()
            .filter(|r| !self.wm.is_shown(r.id))
            .map(|r| r.id)
            .collect();
        for record in &mut self.wm.records {
            let shown = !parked.contains(&record.id) && record.window.alive();
            let mapped = self.space.element_location(&record.window).is_some();
            if shown && !mapped {
                self.space
                    .map_element(record.window.clone(), record.rect.loc, false);
                changed = true;
            } else if !shown && mapped {
                self.space.unmap_elem(&record.window);
                changed = true;
            }
            record.set_suspended(record.mapped && !shown);
        }
        let in_space: Vec<Window> = self.space.elements().cloned().collect();
        let order: Vec<Window> = self
            .wm
            .records
            .iter()
            .filter(|r| in_space.contains(&r.window))
            .map(|r| r.window.clone())
            .collect();
        if order != in_space {
            for window in &order {
                self.space.raise_element(window, false);
            }
            self.restack_x11();
            changed = true;
        }
        self.sync_bars();
        if changed {
            self.queue_redraw_all();
            self.notify(Event::WindowsChanged);
        }
    }

    /// Front first.
    pub fn window_infos(&self) -> Vec<WindowInfo> {
        let focus = self.seat.get_keyboard().and_then(|k| k.current_focus());
        let focused = focus.as_ref().and_then(|f| f.surface());
        self.wm
            .records
            .iter()
            .rev()
            .filter(|r| r.mapped)
            .map(|record| {
                let (app_id, title) = record.names();
                let rect = self
                    .space
                    .element_geometry(&record.window)
                    .unwrap_or(record.rect);
                let output = self
                    .space
                    .outputs()
                    .find(|o| {
                        self.space
                            .output_geometry(o)
                            .is_some_and(|g| g.overlaps(rect))
                    })
                    .map(|o| o.name())
                    .unwrap_or_default();
                WindowInfo {
                    id: record.id,
                    app_id,
                    app: record.app.clone(),
                    title,
                    output,
                    focused: focused.is_some() && focused == record.surface().as_ref(),
                    x: rect.loc.x,
                    y: rect.loc.y,
                    width: rect.size.w,
                    height: rect.size.h,
                    mode: record.mode.name(),
                    minimised: record.minimised,
                    hidden: record.hidden,
                    parent: record.parent.unwrap_or(0),
                    attention: record.attention,
                    not_responding: record.not_responding,
                    desktop: self.desktop_index(record.id),
                    game: record.game,
                }
            })
            .collect()
    }

    pub fn display_infos(&self) -> Vec<isle_ipc::DisplayInfo> {
        self.space
            .outputs()
            .map(|output| {
                let properties = output.physical_properties();
                let mode = output.current_mode();
                let location = self
                    .space
                    .output_geometry(output)
                    .map(|g| g.loc)
                    .unwrap_or_default();
                isle_ipc::DisplayInfo {
                    name: output.name(),
                    make: properties.make,
                    model: properties.model,
                    serial: properties.serial_number,
                    width: mode.map_or(0, |m| m.size.w),
                    height: mode.map_or(0, |m| m.size.h),
                    refresh: mode.map_or(0, |m| m.refresh),
                    scale: output.current_scale().fractional_scale(),
                    x: location.x,
                    y: location.y,
                }
            })
            .collect()
    }
}

impl State {
    /// Shows `id`, un-minimising it, showing its app and moving its output's view to its
    /// desktop, then focuses it.
    pub fn activate(&mut self, id: u64) -> bool {
        let Some((app, desktop)) = self.isle.wm.get(id).map(|r| (r.app.clone(), r.desktop)) else {
            return false;
        };
        if let Some((identity, index)) = self.isle.desktops.locate(desktop)
            && self.isle.desktops.rows[identity].current != index
        {
            let identity = identity.to_owned();
            self.isle.desktops.rows.get_mut(&identity).unwrap().current = index;
            self.isle.notify(Event::DesktopsChanged);
        }
        if self.isle.wm.get(id).is_some_and(|r| r.hidden) {
            self.set_app_hidden(&app, false);
        }
        if let Some(record) = self.isle.wm.get_mut(id) {
            record.minimised = false;
        }
        self.isle.sync_space();
        let window = self.isle.window_by_id(id);
        self.set_focus(window.as_ref());
        true
    }

    pub fn minimise(&mut self, id: u64) -> bool {
        let Some(record) = self.isle.wm.get(id) else {
            return false;
        };
        if matches!(record.mode, Mode::FullScreen { .. }) {
            return false;
        }
        let target = self.isle.minimise_target(id);
        self.animate_leaving(id, target);
        let record = self.isle.wm.get_mut(id).unwrap();
        record.minimised = true;
        self.after_parking(id);
        true
    }

    pub fn unminimise(&mut self, id: u64) -> bool {
        if !self.isle.wm.get(id).is_some_and(|r| r.minimised) {
            return self.isle.wm.get(id).is_some();
        }
        let target = self.isle.minimise_target(id);
        let done = self.activate(id);
        self.animate_arriving(id, target);
        done
    }

    /// Hides or shows every window of `app`; false when it has none.
    pub fn set_app_hidden(&mut self, app: &str, hidden: bool) -> bool {
        let ids: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| r.app == app)
            .map(|r| r.id)
            .collect();
        if hidden {
            for id in &ids {
                self.animate_leaving(*id, None);
            }
        }
        let was_shown: Vec<u64> = ids
            .iter()
            .copied()
            .filter(|id| self.isle.wm.is_shown(*id))
            .collect();
        for id in &ids {
            self.isle.wm.get_mut(*id).unwrap().hidden = hidden;
        }
        if !hidden {
            self.isle.sync_space();
            let appearing: Vec<u64> = ids
                .iter()
                .copied()
                .filter(|id| !was_shown.contains(id) && self.isle.wm.is_shown(*id))
                .collect();
            for id in appearing {
                self.animate_showing(id);
            }
        }
        match (hidden, ids.first()) {
            (_, None) => return false,
            (true, Some(id)) => self.after_parking(*id),
            (false, Some(_)) => self.isle.sync_space(),
        }
        true
    }

    /// Moves and sizes `id` to `rect` in the global space.
    pub fn set_box(&mut self, id: u64, rect: Rectangle<i32, Logical>) -> bool {
        let Some(record) = self.isle.wm.get_mut(id) else {
            return false;
        };
        record.configure_size(rect.size);
        record.record_at_size = Some(rect.size);
        self.move_window(id, rect.loc);
        true
    }

    /// After `id` left the screen: the `Space` follows, and focus moves on if it was there.
    fn after_parking(&mut self, id: u64) {
        let had_focus = self.focused_window() == Some(id)
            || self
                .focused_window()
                .and_then(|f| self.isle.wm.get(f))
                .is_some_and(|r| !self.isle.wm.is_shown(r.id));
        self.isle.sync_space();
        if had_focus {
            self.focus_next();
        }
    }

    /// The window that has the keyboard, by id.
    pub fn focused_window(&self) -> Option<u64> {
        let focus = self.isle.seat.get_keyboard()?.current_focus()?;
        self.isle.wm.for_surface(focus.surface()?).map(|r| r.id)
    }

    /// A toplevel went: it leaves the registry, and focus moves on if it had it.
    pub fn remove_window(&mut self, surface: &WlSurface) {
        let Some(id) = self.isle.wm.for_surface(surface).map(|r| r.id) else {
            return;
        };
        if self.isle.wm.get(id).is_some_and(|r| r.mapped) {
            self.animate_leaving(id, None);
        }
        self.isle.shadow_cache.borrow_mut().retain(|(w, _), _| *w != id);
        self.isle.shell.detach(id);
        self.isle.scrims.borrow_mut().remove(&id);
        let record = self.isle.wm.get(id).unwrap();
        let id = record.id;
        let window = record.window.clone();
        let full_screen = match &record.mode {
            Mode::FullScreen { origin, .. } => Some((record.desktop, *origin)),
            _ => None,
        };
        self.isle.queue_redraw_for_window(&window);
        self.isle.space.unmap_elem(&window);
        self.isle.wm.records.retain(|r| r.id != id);
        self.isle.wm.recent.retain(|r| *r != id);
        for record in &mut self.isle.wm.records {
            if record.parent == Some(id) {
                record.parent = None;
            }
        }
        self.isle.notify(Event::WindowsChanged);
        self.isle.pings.retain(|(_, i)| *i != id);
        self.finish_quits();
        if let Some((desktop, origin)) = full_screen {
            self.full_screen_window_gone(desktop, origin);
        }
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        if keyboard
            .current_focus()
            .is_none_or(|f| f.surface() == Some(surface) || !f.alive())
        {
            self.focus_next();
        }
    }

    /// A window whose client attached a null buffer: it leaves the screen and waits to be
    /// configured again, keeping its record.
    pub fn unmap_window(&mut self, id: u64) {
        let Some(record) = self.isle.wm.get_mut(id) else {
            return;
        };
        let full_screen = match &record.mode {
            Mode::FullScreen { origin, .. } => Some((record.desktop, *origin)),
            _ => None,
        };
        record.mapped = false;
        record.mode = Mode::Floating;
        record.bar = None;
        record.motion = None;
        record.morph = None;
        let window = record.window.clone();
        self.isle.shell.detach(id);
        self.isle.queue_redraw_for_window(&window);
        self.isle.space.unmap_elem(&window);
        if let Some((desktop, origin)) = full_screen {
            self.full_screen_window_gone(desktop, origin);
        }
        self.isle.notify(Event::WindowsChanged);
        if self.focused_window().is_none_or(|f| f == id) {
            self.focus_next();
        }
    }

    /// Focuses the most recently focused window still shown, after the focused one went.
    pub fn focus_next(&mut self) {
        let next = self.isle.wm.most_recent_shown().map(|r| r.window.clone());
        self.set_focus(next.as_ref());
    }
}
