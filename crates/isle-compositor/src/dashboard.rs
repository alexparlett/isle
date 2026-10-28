//! The widgets dashboard: its model, its keys, its routes and its backdrop
//! (`p1/structure/widgets.md`; `p17-widgets.md` choices 5–8).

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};

use isle_shell::{
    dashboard::{
        catalogue,
        grid::{Extent, History, Layout, Metrics, default_layout, kind_of},
        surface::{Body, ClockView, DashboardAction, DashboardSurface, Frame, WidgetView},
    },
    prompts::session::Choice,
    transition::Motion,
};
use smithay::{
    backend::renderer::element::{Kind, solid::SolidColorBuffer, solid::SolidColorRenderElement},
    output::Output,
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::{Rectangle, Scale},
};
use tracing::info;

use crate::{
    focus::Focus,
    overlays::Overlay,
    render::IsleElement,
    state::{Isle, State},
};

pub struct DashboardState {
    pub surface: Rc<DashboardSurface>,
    pub open: Option<Open>,
    /// The output its backdrop is drawn on, until it has faded out.
    pub shown_on: Option<String>,
    pub backdrop: Motion,
    last_step: Instant,
    tick: Option<RegistrationToken>,
    floors: RefCell<HashMap<String, (SolidColorBuffer, SolidColorBuffer)>>,
}

pub struct Open {
    pub output: String,
    pub identity: String,
    pub extent: Extent,
    pub layout: Layout,
    pub edit: bool,
    pub selected: Option<String>,
    /// The selection came from a key, so the ring shows.
    pub ring: bool,
    pub history: History,
    /// The hot corner that opened it, which closes it again.
    pub corner: bool,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            surface: DashboardSurface::new(),
            open: None,
            shown_on: None,
            backdrop: Motion::at(0.0),
            last_step: Instant::now(),
            tick: None,
            floors: RefCell::default(),
        }
    }
}

impl Isle {
    /// The grid on `output`, from the tokens.
    pub fn dashboard_extent(&self, output: &Output) -> Extent {
        let size = self.space.output_geometry(output).unwrap_or_default().size;
        let t = &self.shell.theme().tokens;
        Extent::of(
            size.w as f32,
            size.h as f32,
            Metrics {
                cell: t.size_widget_cell,
                gap: t.space_l,
                side: t.space_xxxl,
                top: t.size_strip + t.space_l,
                bottom: t.space_xxxl + t.size_control_large + t.space_xxl,
            },
        )
    }

    /// The layout `settings.kdl` keeps for the screen of `identity`, or the default.
    pub fn dashboard_layout(&self, identity: &str, extent: &Extent) -> Layout {
        match self
            .settings
            .dashboard_output
            .get(identity)
            .and_then(|d| d.widgets.as_ref())
        {
            Some(lines) => {
                let (layout, skipped) = Layout::from_lines(lines, extent);
                if !skipped.is_empty() {
                    tracing::warn!(identity, ?skipped, "dashboard lines that do not read");
                }
                layout
            }
            None => default_layout(extent),
        }
    }

    /// The backdrop's opacity on `output`, while the dashboard is there or fading from it.
    pub fn dashboard_backdrop(&self, output: &Output) -> Option<f32> {
        (self.dashboard.shown_on.as_deref() == Some(&output.name()))
            .then(|| self.dashboard.backdrop.value().clamp(0.0, 1.0) as f32)
    }

    pub fn dashboard_open_on(&self, output: &Output) -> bool {
        self.dashboard
            .open
            .as_ref()
            .is_some_and(|o| o.output == output.name())
    }

    /// The wash, the softened wallpaper and, once it covers everything, a floor, front to
    /// back (as the lock's).
    pub fn dashboard_backdrop_elements(
        &self,
        output: &Output,
        alpha: f32,
        push: &mut dyn FnMut(IsleElement),
    ) {
        let Some(geometry) = self.space.output_geometry(output) else {
            return;
        };
        let scale = Scale::from(output.current_scale().fractional_scale());
        let tokens = &self.shell.theme().tokens;
        let wash = if self.shell.theme().appearance.increase_contrast {
            tokens.material_lock_wash_contrast
        } else {
            tokens.material_lock_wash
        };
        let mut floors = self.dashboard.floors.borrow_mut();
        let (floor, wash_buffer) = floors.entry(output.name()).or_default();
        wash_buffer.update(geometry.size, crate::wallpaper::premultiplied(wash));
        floor.update(
            geometry.size,
            crate::wallpaper::premultiplied(tokens.colour_window_sunken),
        );
        push(IsleElement::Solid(SolidColorRenderElement::from_buffer(
            wash_buffer,
            (0, 0),
            scale,
            alpha,
            Kind::Unspecified,
        )));
        self.softened_elements(output, alpha, &mut |e| push(IsleElement::Wallpaper(e)));
        if alpha >= 1.0 {
            push(IsleElement::Solid(SolidColorRenderElement::from_buffer(
                floor,
                (0, 0),
                scale,
                1.0,
                Kind::Unspecified,
            )));
        }
    }

    fn clock_view(&self, id: &str) -> ClockView {
        let zone = self
            .settings
            .widget
            .get(id)
            .and_then(|w| match w.get("zone") {
                Some(isle_settings::Value::Str(z)) => jiff::tz::TimeZone::get(z).ok(),
                _ => None,
            })
            .unwrap_or_else(jiff::tz::TimeZone::system);
        let now = jiff::Zoned::now().with_time_zone(zone.clone());
        let hours = crate::island::clock_24_hour(&self.settings.island);
        let time = if hours {
            now.strftime("%H:%M").to_string()
        } else {
            now.strftime("%-I:%M").to_string()
        };
        ClockView {
            city: city_of(zone.iana_name().unwrap_or("UTC")),
            time,
            weekday: now.strftime("%A").to_string(),
            date: now.strftime("%-d %B").to_string(),
        }
    }

    pub fn dashboard_frame(&self) -> Option<Frame> {
        let open = self.dashboard.open.as_ref()?;
        let widgets = open
            .layout
            .widgets
            .iter()
            .map(|p| {
                let kind = kind_of(&p.id);
                let name = catalogue::builtin(kind).map_or(kind, |b| b.name).to_owned();
                let body = match kind {
                    "clock" => Body::Clock(self.clock_view(&p.id)),
                    "session" => Body::Session,
                    _ => Body::Title,
                };
                WidgetView {
                    id: p.id.clone(),
                    name,
                    at: p.at,
                    body,
                }
            })
            .collect();
        Some(Frame {
            extent: open.extent,
            widgets,
            edit: open.edit,
            selected: open.selected.clone(),
            ring: open.ring,
            solid: self.game_mode_active() || self.shell.theme().appearance.reduce_transparency,
        })
    }
}

impl Isle {
    /// The dashboard's model for the Test interface.
    pub fn dashboard_status(&self) -> String {
        let open = self.dashboard.open.as_ref();
        serde_json::json!({
            "open": open.is_some(),
            "output": open.map(|o| o.output.clone()),
            "identity": open.map(|o| o.identity.clone()),
            "edit": open.is_some_and(|o| o.edit),
            "selected": open.and_then(|o| o.selected.clone()),
            "ring": open.is_some_and(|o| o.ring),
            "grid": open.map(|o| [o.extent.cols, o.extent.rows]),
            "layout": open.map(|o| o.layout.to_lines()),
            "backdrop": self.dashboard.backdrop.value(),
            "shown_on": self.dashboard.shown_on.clone(),
        })
        .to_string()
    }
}

/// A zone's city as people write it: `America/New_York` is New York.
fn city_of(zone: &str) -> String {
    zone.rsplit('/').next().unwrap_or(zone).replace('_', " ")
}

impl State {
    /// The chord, the CLI and Spotlight: the dashboard on the screen with the focused
    /// window, else the pointer's, or closed where it is open. A hot corner names its screen.
    pub fn toggle_dashboard(&mut self, on: Option<Output>, edit: bool, corner: bool) {
        if let Some(open) = &self.isle.dashboard.open {
            let same = on.as_ref().is_none_or(|o| o.name() == open.output);
            if same && edit && !open.edit {
                return self.dashboard_edit(true);
            }
            if same && (!corner || open.corner) {
                return self.close_dashboard();
            }
        }
        let output = on
            .or_else(|| self.isle.output_with_focus())
            .or_else(|| self.isle.active_output())
            .or_else(|| self.isle.space.outputs().next().cloned());
        if let Some(output) = output {
            self.open_dashboard(&output, edit, corner);
        }
    }

    pub fn open_dashboard(&mut self, output: &Output, edit: bool, corner: bool) {
        if self.isle.lock.locked || self.isle.prompts_hold_keyboard() {
            return;
        }
        if self.isle.dashboard.open.is_some() {
            self.close_dashboard();
        }
        self.close_overlays(Overlay::Dashboard);
        if self.isle.island.panel.is_some() {
            self.close_control_centre();
        }
        let identity = Isle::output_identity(output);
        let extent = self.isle.dashboard_extent(output);
        let layout = self.isle.dashboard_layout(&identity, &extent);
        info!(output = output.name(), edit, "widgets");
        self.isle.dashboard.open = Some(Open {
            output: output.name(),
            identity,
            extent,
            layout,
            edit,
            selected: None,
            ring: false,
            history: History::default(),
            corner,
        });
        let screen = self.isle.space.output_geometry(output).unwrap_or_default();
        self.isle.dashboard.surface.set_size(screen.size);
        self.put_dashboard_frame();
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        self.isle
            .shell
            .open_placed("dashboard", &place, Rectangle::from_size(screen.size));
        if let Some(focus) = self.isle.shell.keyboard_taker("dashboard") {
            let keyboard = self.isle.seat.get_keyboard().unwrap();
            let previous = keyboard.current_focus();
            self.isle.shell_keyboard_return.push((focus.clone(), previous));
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            self.focus_keyboard(Some(Focus::Shell(focus)), serial);
        }
        let theme = self.isle.shell.theme().clone();
        let still = !self.isle.shell.animated()
            || theme.appearance.reduce_motion
            || self.isle.game_mode_active();
        if self.isle.dashboard.shown_on.as_deref() != Some(&output.name()) {
            self.isle.dashboard.backdrop = Motion::at(0.0);
        }
        self.isle.dashboard.shown_on = Some(output.name());
        if still {
            self.isle.dashboard.backdrop = Motion::at(1.0);
        } else {
            self.isle
                .dashboard
                .backdrop
                .fade_to(1.0, Duration::from_millis(theme.tokens.motion_fade_ms as u64));
        }
        self.isle.dashboard.last_step = Instant::now();
        self.arm_dashboard_tick();
        self.isle.refresh_island();
        self.dispatch_shell();
        self.isle.queue_redraw_all();
    }

    pub fn close_dashboard(&mut self) {
        let Some(_) = self.isle.dashboard.open.take() else {
            return;
        };
        if let Some(token) = self.isle.dashboard.tick.take() {
            self.isle.loop_handle.remove(token);
        }
        let theme = self.isle.shell.theme().clone();
        let still = !self.isle.shell.animated()
            || theme.appearance.reduce_motion
            || self.isle.game_mode_active();
        if still {
            self.isle.dashboard.backdrop = Motion::at(0.0);
            self.isle.dashboard.shown_on = None;
        } else {
            self.isle.dashboard.backdrop.fade_to(
                0.0,
                Duration::from_millis(theme.tokens.motion_exit_fade_ms as u64),
            );
        }
        self.isle.dashboard.last_step = Instant::now();
        self.close_surface("dashboard");
        self.isle.refresh_island();
        self.isle.queue_redraw_all();
    }

    /// Moves the backdrop's fade on; its end on closing stops drawing it.
    pub fn step_dashboard(&mut self, now: Instant) {
        let d = &mut self.isle.dashboard;
        if d.shown_on.is_none() {
            return;
        }
        let dt = now.duration_since(d.last_step).as_secs_f64().min(0.1);
        d.last_step = now;
        d.backdrop.step(now, dt);
        if d.open.is_none() && d.backdrop.settled() {
            d.shown_on = None;
        }
        if !d.backdrop.settled() {
            self.isle.queue_redraw_all();
        }
    }

    pub fn put_dashboard_frame(&mut self) {
        if let Some(frame) = self.isle.dashboard_frame() {
            self.isle.dashboard.surface.set(frame);
            self.isle.shell.with_root("dashboard", || {});
            self.isle.queue_redraw_all();
        }
    }

    /// Wakes on the next minute while open, for the clocks.
    fn arm_dashboard_tick(&mut self) {
        if let Some(token) = self.isle.dashboard.tick.take() {
            self.isle.loop_handle.remove(token);
        }
        let wall = jiff::Timestamp::now();
        let into = wall.as_second().rem_euclid(60) as f64 + wall.subsec_nanosecond() as f64 / 1e9;
        let next = Duration::from_secs_f64(60.0 - into + 0.001);
        self.isle.dashboard.tick = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(next), |_, _, state| {
                state.isle.dashboard.tick = None;
                if state.isle.dashboard.open.is_some() {
                    state.put_dashboard_frame();
                    state.arm_dashboard_tick();
                }
                TimeoutAction::Drop
            })
            .ok();
    }

    pub fn dashboard_edit(&mut self, on: bool) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        if open.edit == on {
            return;
        }
        open.edit = on;
        open.history.clear();
        self.put_dashboard_frame();
    }

    pub fn dashboard_action(&mut self, action: DashboardAction) {
        match action {
            DashboardAction::Close => self.close_dashboard(),
            DashboardAction::Edit => self.dashboard_edit(true),
            DashboardAction::Done => self.dashboard_edit(false),
            DashboardAction::Gallery => {}
            DashboardAction::Open(id) => self.dashboard_open_widget(&id),
            DashboardAction::Session(choice) => {
                self.close_dashboard();
                match choice {
                    Choice::Lock => self.lock("widgets"),
                    Choice::Sleep => self.isle.suspend(),
                    other => self.open_session_confirm(other),
                }
            }
        }
    }

    /// What a widget's body opens, where it has one; until each widget has its place, none.
    fn dashboard_open_widget(&mut self, id: &str) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        if open.edit {
            open.selected = Some(id.to_owned());
            open.ring = false;
            self.put_dashboard_frame();
        }
    }

    /// A key while the dashboard has the keyboard; true when it took it.
    pub fn dashboard_key(&mut self, code: u32) -> bool {
        use smithay::input::keyboard::keysyms;
        let Some(open) = self.isle.dashboard.open.as_ref() else {
            return false;
        };
        let focus = self.isle.shell.keyboard_taker("dashboard");
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        if focus.is_none() || keyboard.current_focus() != focus.map(Focus::Shell) {
            return false;
        }
        let mods = self.held_mods();
        if mods.ctrl || mods.cmd || mods.option {
            return false;
        }
        let sym = self
            .isle
            .keyboards
            .with_engine(false, &[], |_, ctx| ctx.keys.sym(code))
            .map_or(0, |s| s.raw());
        let edit = open.edit;
        match sym {
            keysyms::KEY_Escape => {
                if edit {
                    self.dashboard_edit(false);
                } else {
                    self.close_dashboard();
                }
            }
            keysyms::KEY_e | keysyms::KEY_E if !edit => self.dashboard_edit(true),
            keysyms::KEY_Left => self.dashboard_select(-1, 0),
            keysyms::KEY_Right => self.dashboard_select(1, 0),
            keysyms::KEY_Up => self.dashboard_select(0, -1),
            keysyms::KEY_Down => self.dashboard_select(0, 1),
            keysyms::KEY_Home => self.dashboard_select_end(false),
            keysyms::KEY_End => self.dashboard_select_end(true),
            keysyms::KEY_Return | keysyms::KEY_KP_Enter => {
                if let Some(id) = open.selected.clone() {
                    self.dashboard_open_widget(&id);
                }
            }
            _ => return false,
        }
        true
    }

    /// The selection moved to the nearest widget in a direction, by position; the first in
    /// reading order when there is none.
    fn dashboard_select(&mut self, dx: i32, dy: i32) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        open.ring = true;
        let reading = reading_order(&open.layout);
        let Some(from) = open.selected.as_ref().and_then(|id| open.layout.get(id)) else {
            open.selected = reading.first().cloned();
            return self.put_dashboard_frame();
        };
        let centre = |c: isle_shell::dashboard::grid::Cells| (2 * c.x + c.w, 2 * c.y + c.h);
        let (fx, fy) = centre(from.at);
        let best = open
            .layout
            .widgets
            .iter()
            .filter(|p| p.id != from.id)
            .filter_map(|p| {
                let (x, y) = centre(p.at);
                let (along, across) = if dx != 0 { ((x - fx) * dx, (y - fy).abs()) } else { ((y - fy) * dy, (x - fx).abs()) };
                (along > 0).then_some((along + 2 * across, p.id.clone()))
            })
            .min();
        if let Some((_, id)) = best {
            open.selected = Some(id);
        }
        self.put_dashboard_frame();
    }

    fn dashboard_select_end(&mut self, last: bool) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        open.ring = true;
        let reading = reading_order(&open.layout);
        open.selected = if last { reading.last() } else { reading.first() }.cloned();
        self.put_dashboard_frame();
    }
}

/// Widgets by row, then column, of their top left.
fn reading_order(layout: &Layout) -> Vec<String> {
    let mut all: Vec<_> = layout.widgets.iter().map(|p| (p.at.y, p.at.x, p.id.clone())).collect();
    all.sort();
    all.into_iter().map(|(_, _, id)| id).collect()
}
