//! Mission Control and App Exposé: opening and closing, the screens fed to the model, its
//! effects carried out, the keys and pointer read while it is open, the miniatures and tray
//! thumbnails, and the tiles drawn in the compositor's own pass (`p16-mission-control.md`
//! choices 7 to 13).

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use isle_shell::{
    mission::{
        DeskFace, ScreenFrame, WindowFace,
        model::{
            self, Desk, DeskKind, Effect, Key, Metrics, Mission, Mode, Place, Rect, Screen,
            Settings,
        },
    },
    switcher::surface::PreviewImage,
    transition::Motion,
};
use smithay::{
    backend::renderer::{
        element::{
            Kind,
            solid::{SolidColorBuffer, SolidColorRenderElement},
        },
        gles::GlesRenderer,
    },
    input::keyboard::{Keysym, keysyms},
    output::Output,
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::{Logical, Point, Rectangle, Scale},
};
use tracing::{debug, info};

use crate::{
    render::IsleElement,
    state::{Isle, State},
    wm::desktops::Kind as DesktopKind,
};

/// The most miniatures and thumbnails of things that drew since redrawn in one pass, and how
/// often.
const IMAGES_PER_PASS: usize = 4;
const IMAGE_INTERVAL: Duration = Duration::from_millis(100);

/// What happens once Mission Control has closed.
#[derive(Clone, Debug, PartialEq)]
enum After {
    Nothing,
    Activate(u64),
}

pub struct MissionState {
    pub model: Mission,
    /// The outputs in the model's screen order.
    pub outputs: Vec<Output>,
    /// 0 closed, 1 open: windows travel between their places and their tiles by it.
    pub progress: Motion,
    last_step: Instant,
    /// Closing: no input is read, and the tiles travel home.
    pub closing: bool,
    /// The window chosen, drawn over the rest as it lands.
    chosen: Option<u64>,
    /// Windows that committed since their images were made.
    pub stale: HashSet<u64>,
    miniatures: HashMap<(String, u64), PreviewImage>,
    thumbs: HashMap<u64, PreviewImage>,
    refreshed: Instant,
    refresh: Option<RegistrationToken>,
    veil: RefCell<HashMap<String, SolidColorBuffer>>,
}

impl MissionState {
    /// Open and reading input.
    pub fn open(&self) -> bool {
        !self.closing
    }
}

fn rect_of(r: Rectangle<i32, Logical>, origin: Point<i32, Logical>) -> Rect {
    Rect::new(
        (r.loc.x - origin.x) as f32,
        (r.loc.y - origin.y) as f32,
        r.size.w as f32,
        r.size.h as f32,
    )
}

fn logical(r: &Rect, origin: Point<i32, Logical>) -> Rectangle<f64, Logical> {
    Rectangle::new(
        (r.x as f64 + origin.x as f64, r.y as f64 + origin.y as f64).into(),
        (r.w as f64, r.h as f64).into(),
    )
}

impl Isle {
    /// The model's metrics from the tokens.
    fn mission_metrics(&self) -> Metrics {
        let t = &self.shell.theme().tokens;
        let line = |style| isle_ui::text::metrics(self.shell.theme(), style).1;
        Metrics {
            strip: t.size_strip,
            miniature: t.size_desktop_miniature,
            miniature_min: t.size_desktop_miniature_min,
            label: line(isle_ui::text::Style::Callout),
            tray_tile: (t.size_tray_tile_width, t.size_tray_tile_height),
            tray_heading: line(isle_ui::text::Style::Subheadline),
            header: t.size_row_large,
            space_xs: t.space_xs,
            space_s: t.space_s,
            space_m: t.space_m,
            space_l: t.space_l,
            space_xxl: t.space_xxl,
            space_xxxl: t.space_xxxl,
            tile_reserve: t.size_app_icon_medium / 2.0 + t.space_xs + t.size_control,
            cluster_reserve: t.space_s + t.size_control_large,
            max_desks: crate::wm::desktops::MAX_DESKTOPS,
        }
    }

    pub fn mission_settings(&self) -> Settings {
        Settings {
            group_by_app: self.settings.mission_control.group_by_app,
        }
    }

    /// Every output's row and windows, the outputs left to right.
    pub fn mission_screens(&self) -> (Vec<Screen>, Vec<Output>) {
        let mut outputs: Vec<Output> = self.space.outputs().cloned().collect();
        outputs.sort_by_key(|o| {
            self.space
                .output_geometry(o)
                .map_or((0, 0), |g| (g.loc.x, g.loc.y))
        });
        let focused = self.output_with_focus();
        let mut screens = Vec::new();
        let mut kept = Vec::new();
        for output in outputs {
            let Some(geometry) = self.space.output_geometry(&output) else {
                continue;
            };
            let identity = Self::output_identity(&output);
            let Some(row) = self.desktops.rows.get(&identity) else {
                continue;
            };
            let ids: Vec<u64> = row.desktops.iter().map(|d| d.id).collect();
            let current = row.desktops.get(row.current).map(|d| d.id);
            let on = |r: &crate::wm::Record| {
                r.mapped
                    && r.parent.is_none()
                    && (ids.contains(&r.desktop)
                        || (r.desktop == 0 && self.output_of(r.id).as_ref() == Some(&output)))
            };
            let windows: Vec<model::Window> = self
                .wm
                .records
                .iter()
                .filter(|r| on(r))
                .filter_map(|r| {
                    let frame = self.frame(r.id)?;
                    let app = self.owner_app(r).to_owned();
                    Some(model::Window {
                        id: r.id,
                        title: r.names().1,
                        icon: self
                            .app_icon(&app, 128)
                            .map(|p| p.to_string_lossy().into_owned()),
                        app,
                        rect: rect_of(frame, geometry.loc),
                        minimised: r.minimised,
                        hidden: r.hidden,
                        game: r.game,
                        min_width: self.size_bounds(r.id).0.w as f32,
                        set: None,
                    })
                })
                .collect();
            let desks = row
                .desktops
                .iter()
                .map(|d| Desk {
                    id: d.id,
                    kind: match d.kind {
                        DesktopKind::Normal => DeskKind::Normal,
                        DesktopKind::FullScreen { window } => DeskKind::FullScreen(window),
                        DesktopKind::Split { left, right, .. } => DeskKind::Split(left, right),
                    },
                    // Back to front, as the registry stacks them; one on every desktop is on
                    // the one in view.
                    windows: self
                        .wm
                        .records
                        .iter()
                        .filter(|r| on(r))
                        .filter(|r| r.desktop == d.id || (r.desktop == 0 && Some(d.id) == current))
                        .map(|r| r.id)
                        .collect(),
                    stage: false,
                })
                .collect();
            screens.push(Screen {
                output: output.name(),
                width: geometry.size.w as f32,
                height: geometry.size.h as f32,
                desks,
                current: row.current,
                windows,
                focused: focused.as_ref() == Some(&output),
            });
            kept.push(output);
        }
        if !screens.iter().any(|s| s.focused)
            && let Some(first) = screens.first_mut()
        {
            first.focused = true;
        }
        (screens, kept)
    }

    pub fn mission_on(&self, output: &Output) -> bool {
        self.mission
            .as_ref()
            .is_some_and(|m| m.outputs.contains(output))
    }

    /// Something is moving: the tiles on their way, or a drag.
    pub fn mission_moving(&self, output: &Output) -> bool {
        self.mission
            .as_ref()
            .is_some_and(|m| m.outputs.contains(output) && !m.progress.settled())
    }

    /// The veil over the wallpaper, faded with the opening.
    pub fn mission_veil(&self, output: &Output, push: &mut dyn FnMut(IsleElement)) {
        let Some(mission) = &self.mission else {
            return;
        };
        let Some(geometry) = self.space.output_geometry(output) else {
            return;
        };
        // Split View's chooser veils only the free half.
        let (at, size) = match mission.model.mode() {
            Mode::Chooser { left, first, .. } if self.output_of(*first).as_ref() == Some(output) => {
                let half = (geometry.size.w - model::DIVIDER as i32) / 2;
                let x = if *left { half + model::DIVIDER as i32 } else { 0 };
                (Point::from((x, 0)), smithay::utils::Size::from((geometry.size.w - half - model::DIVIDER as i32, geometry.size.h)))
            }
            Mode::Chooser { .. } => return,
            _ => (Point::from((0, 0)), geometry.size),
        };
        let mut veils = mission.veil.borrow_mut();
        let buffer = veils.entry(output.name()).or_default();
        buffer.update(
            size,
            crate::wallpaper::premultiplied(self.shell.theme().tokens.colour_veil),
        );
        let scale = Scale::from(output.current_scale().fractional_scale());
        push(IsleElement::Solid(SolidColorRenderElement::from_buffer(
            buffer,
            at.to_physical_precise_round(scale),
            scale,
            mission.progress.value().clamp(0.0, 1.0) as f32,
            Kind::Unspecified,
        )));
    }

    /// The windows as tiles, front to back: a drag over all, then the window looked at,
    /// then the one chosen, then the rest as they stack.
    pub fn mission_tiles(
        &self,
        renderer: &mut GlesRenderer,
        output: &Output,
        push: &mut dyn FnMut(IsleElement),
    ) {
        let Some(mission) = &self.mission else {
            return;
        };
        let Some(geometry) = self.space.output_geometry(output) else {
            return;
        };
        let Some((index, layout)) = mission.model.layout_of(&output.name()) else {
            return;
        };
        let scale = output.current_scale().fractional_scale();
        let p = mission.progress.value().clamp(0.0, 1.0) as f32;
        let origin = geometry.loc;
        let mut draw =
            |id: u64, to: Rect, alpha: f32, shadow: bool, push: &mut dyn FnMut(IsleElement)| {
                let Some(frame) = self.frame(id) else {
                    return;
                };
                let from = Rectangle::new(frame.loc - origin, frame.size);
                let mut to = logical(&to, origin);
                to.loc -= origin.to_f64();
                self.window_pieces(renderer, id, origin, scale, shadow, alpha, &mut |piece| {
                    push(IsleElement::Window(crate::wm::draw::place(
                        piece, from, to, scale,
                    )))
                });
            };
        let real = |id: u64| self.frame(id).map(|f| rect_of(f, origin));
        if let Some(drag) = mission.model.drag()
            && drag.screen == index
            && let model::Dragged::Window(id) = drag.what
            && let Some(r) = real(id)
        {
            let tile = layout
                .tile(id)
                .map(|t| t.rect)
                .or_else(|| layout.tray_slot(id))
                .unwrap_or(r);
            // Over the bar a tile shrinks to twice a miniature's scale.
            let over_bar = layout
                .bar
                .as_ref()
                .is_some_and(|b| b.panel.contains(drag.at.0, drag.at.1));
            let k = match &layout.bar {
                Some(b) if over_bar => 2.0 * b.miniature.1 / geometry.size.h as f32,
                _ => tile.w / r.w.max(1.0),
            };
            let (w, h) = (r.w * k, r.h * k);
            let grab = (
                drag.grab.0 * w / tile.w.max(1.0),
                drag.grab.1 * h / tile.h.max(1.0),
            );
            draw(
                id,
                Rect::new(drag.at.0 - grab.0, drag.at.1 - grab.1, w, h),
                1.0,
                true,
                push,
            );
        }
        if let Some(id) = mission.model.looking()
            && let Some(r) = real(id)
        {
            draw(id, r, 1.0, true, push);
        }
        if let Some(id) = mission.chosen
            && let (Some(r), Some(t)) = (real(id), layout.tile(id))
        {
            draw(id, r.lerp(&t.rect, p), 1.0, true, push);
        }
        let expose_app = match mission.model.mode() {
            Mode::Expose(app) => Some(app.clone()),
            Mode::MissionControl | Mode::Chooser { .. } => None,
        };
        let first = match mission.model.mode() {
            Mode::Chooser { first, .. } => Some(*first),
            _ => None,
        };
        let skip = |id: u64| {
            mission
                .model
                .drag()
                .is_some_and(|d| d.what == model::Dragged::Window(id))
                || mission.model.looking() == Some(id)
                || mission.chosen == Some(id)
        };
        for record in self.wm.records.iter().rev() {
            let id = record.id;
            if skip(id) {
                continue;
            }
            match layout.tile(id) {
                Some(t) => {
                    if let Some(r) = real(id) {
                        // A misfit is offered dimmed.
                        let alpha = if mission.model.misfit(id) { 0.4 } else { 1.0 };
                        draw(id, r.lerp(&t.rect, p), alpha, true, push);
                    }
                }
                // The chooser's first window keeps its half.
                None if first == Some(id) => {
                    if let Some(r) = real(id) {
                        draw(id, r, 1.0, false, push);
                    }
                }
                // App Exposé fades the other apps' windows in place as it opens.
                None if expose_app.is_some()
                    && p < 1.0
                    && self.wm.is_shown(id)
                    && self.output_of(id).as_ref() == Some(output) =>
                {
                    if let Some(r) = real(id) {
                        draw(id, r, 1.0 - p, true, push);
                    }
                }
                None => {}
            }
        }
    }
}

/// What a key is to Mission Control, by its symbol and the held modifiers; `None` for a key
/// it leaves to the shortcuts.
fn mission_key(
    role: isle_keys::keyinfo::Role,
    sym: Option<Keysym>,
    mods: isle_keys::chord::Mods,
) -> Option<Key> {
    use isle_keys::keyinfo::Role;
    if role != Role::Key {
        return None;
    }
    let raw = sym?.raw();
    let digit = (keysyms::KEY_1..=keysyms::KEY_9)
        .contains(&raw)
        .then(|| (raw - keysyms::KEY_1) as usize);
    if raw == keysyms::KEY_XF86LaunchA {
        return Some(if mods.ctrl {
            Key::Expose
        } else {
            Key::MissionControl
        });
    }
    if mods.ctrl && mods.option && raw == keysyms::KEY_space {
        return Some(Key::WindowMenu);
    }
    if mods.ctrl && mods.shift {
        return match raw {
            keysyms::KEY_Left => Some(Key::Carry(-1)),
            keysyms::KEY_Right => Some(Key::Carry(1)),
            _ => digit.map(Key::CarryTo),
        };
    }
    if mods.ctrl {
        return match raw {
            keysyms::KEY_Up => Some(Key::MissionControl),
            keysyms::KEY_Down => Some(Key::Expose),
            keysyms::KEY_Left => Some(Key::Desktop(-1)),
            keysyms::KEY_Right => Some(Key::Desktop(1)),
            _ => digit.map(Key::Jump),
        };
    }
    if mods.cmd {
        return (raw == keysyms::KEY_w).then_some(Key::Close);
    }
    if mods.option {
        return None;
    }
    Some(match raw {
        keysyms::KEY_Left => Key::Left,
        keysyms::KEY_Right => Key::Right,
        keysyms::KEY_Up => Key::Up,
        keysyms::KEY_Down => Key::Down,
        keysyms::KEY_ISO_Left_Tab => Key::BackTab,
        keysyms::KEY_Tab if mods.shift => Key::BackTab,
        keysyms::KEY_Tab => Key::Tab,
        keysyms::KEY_Return | keysyms::KEY_KP_Enter => Key::Return,
        keysyms::KEY_space => Key::Space,
        keysyms::KEY_w => Key::Close,
        keysyms::KEY_Delete | keysyms::KEY_BackSpace => Key::Delete,
        keysyms::KEY_Escape => Key::Escape,
        _ => return None,
    })
}

impl State {
    /// Opens Mission Control, or App Exposé of the front app; the same again closes it.
    pub fn toggle_mission(&mut self, expose: bool) {
        if let Some(mission) = &self.isle.mission
            && mission.open()
        {
            let same = matches!(
                (mission.model.mode(), expose),
                (Mode::MissionControl, false) | (Mode::Expose(_), true)
            );
            if same {
                self.close_mission(After::Nothing);
            } else {
                let mode = self.expose_mode(expose);
                self.mission_mode(mode);
            }
            return;
        }
        let mode = self.expose_mode(expose);
        self.open_mission(mode);
    }

    fn expose_mode(&self, expose: bool) -> Mode {
        match (expose, self.isle.front_app()) {
            (true, Some(app)) => Mode::Expose(app),
            _ => Mode::MissionControl,
        }
    }

    pub fn open_mission(&mut self, mode: Mode) {
        if self.isle.lock.locked || self.picking() {
            return;
        }
        if self.isle.mission.is_some() {
            self.finish_mission();
        }
        self.cancel_switcher();
        self.close_spotlight();
        self.close_menu_bar_group();
        let shown: Vec<Output> = self
            .isle
            .space
            .outputs()
            .filter(|o| self.isle.desktop_shown(o))
            .cloned()
            .collect();
        for output in shown {
            self.put_desktop_back(&output);
        }
        if self.isle.island.panel.is_some() {
            self.close_control_centre();
        }
        let (screens, outputs) = self.isle.mission_screens();
        if screens.is_empty() {
            return;
        }
        let apps = self.isle.apps_by_recency();
        let focused = self.focused_window();
        let model = Mission::open(
            mode.clone(),
            screens,
            apps,
            focused,
            self.isle.mission_settings(),
            self.isle.mission_metrics(),
        );
        let style = crate::wm::motion::Style::of(&self.isle);
        let mut progress = Motion::at(0.0);
        if style.animated() {
            progress.spring_to(1.0, self.isle.shell.theme().spring_move());
        } else {
            progress = Motion::at(1.0);
        }
        info!(?mode, "mission control");
        self.isle.mission = Some(MissionState {
            model,
            outputs,
            progress,
            last_step: Instant::now(),
            closing: false,
            chosen: None,
            stale: HashSet::new(),
            miniatures: HashMap::new(),
            thumbs: HashMap::new(),
            refreshed: Instant::now(),
            refresh: None,
            veil: RefCell::default(),
        });
        // Pointer constraints are let go while it is open (P1 States).
        self.release_constraints_for_mission();
        self.update_mission_images(true);
        self.push_mission_frames();
        let still = !style.animated();
        self.isle.mission_back.set_still(still);
        self.isle.mission_front.set_still(still);
        self.isle.shell.open("mission-back");
        self.open_surface("mission-front");
        self.isle.queue_redraw_all();
    }

    fn mission_mode(&mut self, mode: Mode) {
        let Some(mission) = self.isle.mission.as_mut() else {
            return;
        };
        mission.model.set_mode(mode);
        self.update_mission_images(true);
        self.push_mission_frames();
        self.isle.queue_redraw_all();
    }

    /// Starts closing: the surfaces go, the tiles travel home.
    fn close_mission(&mut self, after: After) {
        let Some(mission) = self.isle.mission.as_mut() else {
            return;
        };
        if mission.closing {
            return;
        }
        mission.closing = true;
        if let Some(token) = mission.refresh.take() {
            self.isle.loop_handle.remove(token);
        }
        let style = crate::wm::motion::Style::of(&self.isle);
        let mission = self.isle.mission.as_mut().unwrap();
        if style.animated() {
            mission
                .progress
                .spring_to(0.0, self.isle.shell.theme().spring_exit());
        } else {
            mission.progress = Motion::at(0.0);
        }
        if let After::Activate(id) = after {
            mission.chosen = Some(id);
        }
        self.close_surface("mission-front");
        self.isle.shell.close("mission-back");
        self.dispatch_shell();
        if let After::Activate(id) = after {
            self.activate(id);
        }
        if self
            .isle
            .mission
            .as_ref()
            .is_some_and(|m| m.progress.settled())
        {
            self.finish_mission();
        }
        self.isle.queue_redraw_all();
    }

    /// Gone at once: the lock, another centre surface, or the end of its closing.
    pub fn finish_mission(&mut self) {
        let Some(mut mission) = self.isle.mission.take() else {
            return;
        };
        if let Some(token) = mission.refresh.take() {
            self.isle.loop_handle.remove(token);
        }
        if self.isle.shell.is_open("mission-front") {
            self.close_surface("mission-front");
        }
        if self.isle.shell.is_open("mission-back") {
            self.isle.shell.close("mission-back");
        }
        self.isle.mission_back.clear();
        self.isle.mission_front.clear();
        self.dispatch_shell();
        self.refresh_constraint();
        self.isle.queue_redraw_all();
        debug!("mission control closed");
    }

    /// Moves the opening or closing on; the end of a closing removes it.
    pub fn step_mission(&mut self, now: Instant) {
        let Some(mission) = self.isle.mission.as_mut() else {
            return;
        };
        let dt = now.duration_since(mission.last_step).as_secs_f64().min(0.1);
        mission.last_step = now;
        mission.progress.step(now, dt);
        if mission.closing && mission.progress.settled() {
            self.finish_mission();
        }
    }

    fn release_constraints_for_mission(&mut self) {
        self.refresh_constraint();
    }

    /// A key while it is open; true when Mission Control took it.
    /// Mission Control reads the pointer and keys: it is open, and no window menu opened from
    /// it is.
    pub fn mission_reads_input(&self) -> bool {
        self.isle.mission.as_ref().is_some_and(|m| m.open())
            && !self.isle.lock.locked
            && !self.isle.shell.is_open("window-menu")
    }

    pub fn mission_key_pressed(&mut self, code: u32) -> bool {
        if !self.mission_reads_input() {
            return false;
        }
        let (role, sym) = self.isle.keyboards.with_engine(false, &[], |_, ctx| {
            (ctx.keys.role(code), ctx.keys.sym(code))
        });
        let Some(key) = mission_key(role, sym, self.held_mods()) else {
            return false;
        };
        let effects = self.isle.mission.as_mut().unwrap().model.key(key);
        self.mission_effects(effects);
        true
    }

    /// The pointer moved while it is open.
    pub fn mission_pointer_to(&mut self, at: Point<f64, Logical>) {
        let pointer = self.isle.seat.get_pointer().unwrap();
        pointer.motion(
            self,
            None,
            &smithay::input::pointer::MotionEvent {
                location: at,
                serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                time: smithay::backend::input::InputTime::now(),
            },
        );
        pointer.frame(self);
        let Some((screen, x, y)) = self.mission_point(at) else {
            return;
        };
        let effects = self
            .isle
            .mission
            .as_mut()
            .unwrap()
            .model
            .pointer_move(screen, x, y);
        self.mission_effects(effects);
        self.push_mission_frames();
        self.isle.queue_redraw_all();
    }

    pub fn mission_button(&mut self, button: u32, pressed: bool) {
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        let Some((screen, x, y)) = self.mission_point(at) else {
            return;
        };
        let model = &mut self.isle.mission.as_mut().unwrap().model;
        let effects = if pressed {
            model.press(screen, x, y, button)
        } else {
            model.release(screen, x, y, button)
        };
        self.mission_effects(effects);
        self.push_mission_frames();
        self.isle.queue_redraw_all();
    }

    /// The wheel over the bar scrolls it when it overflows.
    pub fn mission_axis(&mut self, amount: f64) {
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        let Some((screen, _, _)) = self.mission_point(at) else {
            return;
        };
        self.isle
            .mission
            .as_mut()
            .unwrap()
            .model
            .scroll_bar(screen, amount as f32);
        self.push_mission_frames();
        self.isle.queue_redraw_all();
    }

    /// A point in the global space as a screen of the model and a place on it.
    fn mission_point(&self, at: Point<f64, Logical>) -> Option<(usize, f32, f32)> {
        let mission = self.isle.mission.as_ref().filter(|m| m.open())?;
        mission.outputs.iter().enumerate().find_map(|(i, o)| {
            let g = self.isle.space.output_geometry(o)?.to_f64();
            g.contains(at)
                .then(|| (i, (at.x - g.loc.x) as f32, (at.y - g.loc.y) as f32))
        })
    }

    fn mission_output(&self, screen: usize) -> Option<Output> {
        self.isle.mission.as_ref()?.outputs.get(screen).cloned()
    }

    fn mission_effects(&mut self, effects: Vec<Effect>) {
        if effects.is_empty() {
            self.push_mission_frames();
            self.isle.queue_redraw_all();
            return;
        }
        for effect in effects {
            info!(?effect, "mission control");
            match effect {
                Effect::Dismiss | Effect::Choose(_)
                    if matches!(self.isle.mission.as_ref().map(|m| m.model.mode()), Some(Mode::Chooser { .. })) =>
                {
                    let Some(Mode::Chooser { first, .. }) = self.isle.mission.as_ref().map(|m| m.model.mode().clone()) else {
                        continue;
                    };
                    self.finish_mission();
                    match effect {
                        Effect::Choose(other) => {
                            self.choose_split(first, other);
                        }
                        _ => self.cancel_split(first),
                    }
                    return;
                }
                Effect::Dismiss => self.close_mission(After::Nothing),
                Effect::Choose(id) => self.close_mission(After::Activate(id)),
                Effect::Restore(id) => {
                    if let Some(app) = self
                        .isle
                        .wm
                        .get(id)
                        .filter(|r| r.hidden)
                        .map(|r| r.app.clone())
                    {
                        self.set_app_hidden(&app, false);
                    }
                    self.unminimise(id);
                    self.close_mission(After::Activate(id));
                }
                Effect::Show { screen, index } => {
                    let output = self.mission_output(screen);
                    self.finish_mission();
                    if let Some(output) = output {
                        self.switch_desktop(&output, index);
                    }
                }
                Effect::Switch { screen, index } => {
                    if let Some(output) = self.mission_output(screen) {
                        self.switch_desktop(&output, index);
                    }
                }
                Effect::CloseWindow(id) => {
                    self.close_window(id);
                }
                Effect::Move { window, to } => self.mission_move(window, to),
                Effect::AddDesktop { screen } => {
                    if let Some(output) = self.mission_output(screen) {
                        self.add_desktop(&output);
                    }
                }
                Effect::RemoveDesktop { screen, index } => {
                    if let Some(output) = self.mission_output(screen) {
                        self.remove_desktop(&output, index);
                    }
                }
                Effect::ExitFullScreen(id) => {
                    self.leave_full_screen(id);
                }
                Effect::Reorder { screen, from, to } => {
                    if let Some(output) = self.mission_output(screen) {
                        self.reorder_desktop(&output, from, to);
                    }
                }
                Effect::Look(_) => {}
                Effect::WindowMenu(id) => {
                    // By its tile, where the window is seen.
                    let at = self.isle.mission.as_ref().and_then(|m| {
                        m.model.layouts().iter().zip(&m.outputs).find_map(|(l, o)| {
                            let t = l.tile(id)?;
                            let g = self.isle.space.output_geometry(o)?;
                            Some(Point::from((
                                g.loc.x + t.rect.x as i32,
                                g.loc.y + t.rect.y as i32,
                            )))
                        })
                    });
                    self.open_window_menu(id, at, at.is_none());
                }
                Effect::DeskMenu { .. } => {}
                Effect::Expose(app) => self.mission_mode(Mode::Expose(app)),
                Effect::Overview => self.mission_mode(Mode::MissionControl),
            }
        }
        self.refresh_mission();
    }

    /// A window dropped or carried to a desktop, a new one, or another screen.
    fn mission_move(&mut self, window: u64, to: Place) {
        match to {
            Place::Desk { screen, index } => {
                if let Some(output) = self.mission_output(screen) {
                    self.place_on_desktop(window, &output, index);
                }
            }
            Place::New { screen } => {
                if let Some(output) = self.mission_output(screen)
                    && let Some(index) = self.add_desktop(&output)
                {
                    self.place_on_desktop(window, &output, index);
                }
            }
            Place::Split {
                screen,
                index,
                left,
            } => {
                if let Some(output) = self.mission_output(screen) {
                    self.split_with(window, &output, index, left);
                }
            }
        }
    }

    /// The windows or desktops changed while it is open.
    pub fn refresh_mission(&mut self) {
        if !self.isle.mission.as_ref().is_some_and(|m| m.open()) {
            return;
        }
        let (screens, outputs) = self.isle.mission_screens();
        if screens.is_empty() {
            self.finish_mission();
            return;
        }
        let apps = self.isle.apps_by_recency();
        let settings = self.isle.mission_settings();
        let mission = self.isle.mission.as_mut().unwrap();
        mission.model.refresh(screens, apps);
        mission.model.set_settings(settings);
        mission.outputs = outputs;
        self.update_mission_images(false);
        self.push_mission_frames();
        self.isle.queue_redraw_all();
    }

    /// Run once per loop iteration: a change of windows reaches the model, and images of
    /// windows that drew are redrawn a few at a time.
    pub fn mission_after_dispatch(&mut self) {
        if self.isle.mission_dirty.take() {
            self.refresh_mission();
        } else if let Some(mission) = &self.isle.mission
            && mission.open()
            && !mission.stale.is_empty()
            && mission.refresh.is_none()
        {
            let wait = IMAGE_INTERVAL.saturating_sub(mission.refreshed.elapsed());
            let token =
                self.isle
                    .loop_handle
                    .insert_source(Timer::from_duration(wait), |_, _, state| {
                        if let Some(mission) = state.isle.mission.as_mut() {
                            mission.refresh = None;
                            state.update_mission_images(false);
                            state.push_mission_frames();
                        }
                        TimeoutAction::Drop
                    });
            if let Ok(token) = token {
                self.isle.mission.as_mut().unwrap().refresh = Some(token);
            }
        }
    }

    /// Draws the miniatures and thumbnails missing, and a few of those whose windows drew
    /// since; with `all`, every one.
    fn update_mission_images(&mut self, all: bool) {
        let Some(mission) = &self.isle.mission else {
            return;
        };
        let stale = mission.stale.clone();
        let mut minis: Vec<(Output, String, u64, Vec<u64>, f32, f32)> = Vec::new();
        let mut thumbs: Vec<(u64, f64)> = Vec::new();
        for (i, layout) in mission.model.layouts().iter().enumerate() {
            let (Some(screen), Some(output)) =
                (mission.model.screens().get(i), mission.outputs.get(i))
            else {
                continue;
            };
            let scale = output.current_scale().fractional_scale();
            if let Some(bar) = &layout.bar {
                for desk in &screen.desks {
                    let key = (screen.output.clone(), desk.id);
                    let have = mission.miniatures.contains_key(&key);
                    if all || !have || desk.windows.iter().any(|w| stale.contains(w)) {
                        minis.push((
                            output.clone(),
                            screen.output.clone(),
                            desk.id,
                            desk.windows.clone(),
                            bar.miniature.0,
                            bar.miniature.1,
                        ));
                    }
                }
            }
            if let Some(tray) = &layout.tray {
                for (id, _) in tray.groups.iter().flat_map(|g| &g.items) {
                    if all || !mission.thumbs.contains_key(id) || stale.contains(id) {
                        thumbs.push((*id, scale));
                    }
                }
            }
        }
        // Missing ones first; those that only drew wait their turn a few at a time.
        if !all {
            let missing = |m: &Option<&MissionState>, key: &(String, u64)| {
                m.is_some_and(|m| !m.miniatures.contains_key(key))
            };
            let mission = self.isle.mission.as_ref();
            let (mut new, old): (Vec<_>, Vec<_>) = minis
                .into_iter()
                .partition(|m| missing(&mission, &(m.1.clone(), m.2)));
            new.extend(old.into_iter().take(IMAGES_PER_PASS));
            minis = new;
        }
        let mut done: Vec<u64> = Vec::new();
        for (output, name, desk, windows, w, h) in minis {
            if let Some(image) = self.render_miniature(&output, &windows, w as f64, h as f64) {
                self.isle
                    .mission
                    .as_mut()
                    .unwrap()
                    .miniatures
                    .insert((name, desk), PreviewImage(image));
            }
            done.extend(windows);
        }
        for (id, scale) in thumbs {
            let (tw, th) = self.isle.mission_metrics().tray_tile;
            if let Some(image) = self.render_preview(id, tw as f64, th as f64, scale) {
                self.isle
                    .mission
                    .as_mut()
                    .unwrap()
                    .thumbs
                    .insert(id, PreviewImage(image));
            }
            done.push(id);
        }
        let mission = self.isle.mission.as_mut().unwrap();
        if all {
            mission.stale.clear();
        } else {
            for id in done {
                mission.stale.remove(&id);
            }
        }
        mission.refreshed = Instant::now();
    }

    /// A desktop's miniature: its wallpaper and its windows at their places, scaled, as an
    /// image the bar's tree draws. Windows off the desktop in view show their last buffers.
    fn render_miniature(
        &mut self,
        output: &Output,
        windows: &[u64],
        w: f64,
        h: f64,
    ) -> Option<isle_shell::switcher::surface::ImageHandle> {
        let geometry = self.isle.space.output_geometry(output)?;
        let scale = output.current_scale().fractional_scale();
        let k = w / geometry.size.w.max(1) as f64;
        let renderer = self.backend.renderer_opt()?;
        let mut placed = Vec::new();
        for &id in windows.iter().rev() {
            let Some(frame) = self.isle.frame(id) else {
                continue;
            };
            if self
                .isle
                .wm
                .get(id)
                .is_some_and(|r| r.minimised || r.hidden)
            {
                continue;
            }
            let from = Rectangle::new(frame.loc - geometry.loc, frame.size);
            let to = Rectangle::<f64, Logical>::new(
                (from.loc.x as f64 * k, from.loc.y as f64 * k).into(),
                (from.size.w as f64 * k, from.size.h as f64 * k).into(),
            );
            self.isle.window_pieces(
                renderer,
                id,
                geometry.loc,
                scale,
                false,
                1.0,
                &mut |piece| placed.push(crate::wm::draw::place(piece, from, to, scale)),
            );
        }
        let screen = Rectangle::from_size(geometry.size);
        let mini = Rectangle::<f64, Logical>::from_size((w, h).into());
        self.isle.wallpaper_elements(output, &mut |e| {
            placed.push(crate::wm::draw::place(
                crate::wm::draw::WindowPiece::Wallpaper(e),
                screen,
                mini,
                scale,
            ))
        });
        let size = ((w * scale).round() as i32, (h * scale).round() as i32).into();
        let texture = crate::wm::motion::render_to_texture(renderer, size, scale, &placed).ok()?;
        self.isle.shell.image_of(renderer, &texture)
    }

    /// Each screen's frame for the two surfaces.
    pub fn push_mission_frames(&mut self) {
        let Some(mission) = &self.isle.mission else {
            return;
        };
        let solid =
            self.isle.shell.theme().appearance.reduce_transparency || self.isle.island_game_mode();
        let mut frames: Vec<(String, ScreenFrame)> = Vec::new();
        for (i, (screen, layout)) in mission
            .model
            .screens()
            .iter()
            .zip(mission.model.layouts())
            .enumerate()
        {
            let here = |t: Option<&model::Target>| t.filter(|t| t.screen() == i).cloned();
            let window_face = |w: &model::Window| WindowFace {
                id: w.id,
                title: w.title.clone(),
                app: self.isle.app_name(&w.app),
                icon: w.icon.clone(),
                game: w.game,
            };
            let desks = screen
                .desks
                .iter()
                .enumerate()
                .map(|(n, d)| {
                    let icon = |id: u64| screen.window(id).and_then(|w| w.icon.clone());
                    let name = |id: u64| {
                        screen
                            .window(id)
                            .map(|w| self.isle.app_name(&w.app))
                            .unwrap_or_default()
                    };
                    let (label, icons, game) = match d.kind {
                        DeskKind::Normal => (
                            format!(
                                "Desktop {}",
                                n + 1
                                    - screen.desks[..n].iter().filter(|d| d.full_screen()).count()
                            ),
                            vec![],
                            false,
                        ),
                        DeskKind::FullScreen(w) => {
                            let game = screen.window(w).is_some_and(|w| w.game);
                            (name(w), if game { vec![] } else { vec![icon(w)] }, game)
                        }
                        DeskKind::Split(a, b) => (
                            format!("{} and {}", name(a), name(b)),
                            vec![icon(a), icon(b)],
                            false,
                        ),
                    };
                    DeskFace {
                        label,
                        current: n == screen.current,
                        icons,
                        game,
                    }
                })
                .collect();
            let drag = mission.model.drag().filter(|d| d.screen == i);
            let header = match mission.model.mode() {
                Mode::Expose(app) => Some((
                    self.isle.app_name(app),
                    self.isle
                        .app_icon(app, 128)
                        .map(|p| p.to_string_lossy().into_owned()),
                )),
                Mode::MissionControl | Mode::Chooser { .. } => None,
            };
            let clusters = layout
                .clusters
                .iter()
                .map(|c| {
                    let app = c.key.strip_prefix("set:").map_or(c.key.clone(), |_| {
                        screen
                            .windows
                            .iter()
                            .find(|w| {
                                w.set.map(|s| format!("set:{s}")).as_deref() == Some(c.key.as_str())
                            })
                            .map(|w| w.app.clone())
                            .unwrap_or_default()
                    });
                    (
                        c.key.clone(),
                        self.isle.app_name(&app),
                        self.isle
                            .app_icon(&app, 64)
                            .map(|p| p.to_string_lossy().into_owned()),
                    )
                })
                .collect();
            frames.push((
                screen.output.clone(),
                ScreenFrame {
                    layout: layout.clone(),
                    desks,
                    windows: screen.windows.iter().map(window_face).collect(),
                    miniatures: screen
                        .desks
                        .iter()
                        .enumerate()
                        .filter_map(|(n, d)| {
                            mission
                                .miniatures
                                .get(&(screen.output.clone(), d.id))
                                .map(|p| (n, p.clone()))
                        })
                        .collect(),
                    thumbs: layout
                        .tray
                        .iter()
                        .flat_map(|t| t.groups.iter().flat_map(|g| &g.items))
                        .filter_map(|(id, _)| mission.thumbs.get(id).map(|p| (*id, p.clone())))
                        .collect(),
                    selection: here(mission.model.selection()),
                    hover: here(mission.model.hover()),
                    badge: mission.model.badge().cloned(),
                    drop: drag.and_then(|d| d.over.clone()),
                    dragging: drag.and_then(|d| match d.what {
                        model::Dragged::Window(id) => Some(id),
                        model::Dragged::Desk(_) => None,
                    }),
                    header,
                    clusters,
                    solid,
                    looking: mission.model.looking(),
                    chooser: matches!(mission.model.mode(), Mode::Chooser { .. }),
                    misfits: layout
                        .tiles
                        .iter()
                        .map(|t| t.window)
                        .filter(|id| mission.model.misfit(*id))
                        .collect(),
                },
            ));
        }
        for (name, frame) in &frames {
            self.isle.mission_back.set_model(name, frame.clone());
            self.isle.mission_front.set_model(name, frame.clone());
        }
        let (back, front) = (
            self.isle.mission_back.clone(),
            self.isle.mission_front.clone(),
        );
        for (surface, name) in [(&back, "mission-back"), (&front, "mission-front")] {
            self.isle.shell.with_roots(name, |output| {
                if let Some((_, frame)) = frames.iter().find(|(n, _)| *n == output.name()) {
                    surface.set(&output.name(), frame.clone());
                }
            });
        }
        self.dispatch_shell();
    }

    /// The model as JSON, for tests and `isle`.
    pub fn mission_json(&self) -> String {
        let Some(mission) = &self.isle.mission else {
            return "null".into();
        };
        let r = |r: &Rect| serde_json::json!([r.x, r.y, r.w, r.h]);
        let screens: Vec<serde_json::Value> = mission
            .model
            .screens()
            .iter()
            .zip(mission.model.layouts())
            .map(|(s, l)| {
                serde_json::json!({
                    "output": s.output,
                    "current": s.current,
                    "desks": s.desks.iter().map(|d| serde_json::json!({
                        "id": d.id,
                        "windows": d.windows,
                        "full_screen": d.full_screen(),
                    })).collect::<Vec<_>>(),
                    "bar": l.bar.as_ref().map(|b| serde_json::json!({
                        "panel": r(&b.panel),
                        "items": b.items.iter().map(|i| serde_json::json!({
                            "desk": i.desk,
                            "miniature": r(&i.miniature),
                        })).collect::<Vec<_>>(),
                        "overflow": b.overflow,
                    })),
                    "tiles": l.tiles.iter().map(|t| serde_json::json!({
                        "window": t.window,
                        "rect": r(&t.rect),
                    })).collect::<Vec<_>>(),
                    "tray": l.tray.as_ref().map(|t| t.groups.iter().map(|g| serde_json::json!({
                        "head": format!("{:?}", g.head),
                        "windows": g.items.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                        "slots": g.items.iter().map(|(_, s)| r(s)).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>()),
                    "header": l.header.as_ref().map(r),
                    "empty": l.empty.is_some(),
                    "clusters": l.clusters.iter().map(|c| c.key.clone()).collect::<Vec<_>>(),
                    "miniatures": s.desks.iter().filter(|d| mission.miniatures.contains_key(&(s.output.clone(), d.id))).count(),
                })
            })
            .collect();
        serde_json::json!({
            "mode": match mission.model.mode() {
                Mode::MissionControl => serde_json::Value::Null,
                Mode::Expose(app) => serde_json::Value::String(app.clone()),
                Mode::Chooser { first, .. } => serde_json::json!({ "chooser": first }),
            },
            "screens": screens,
            "selection": mission.model.selection().map(|t| format!("{t:?}")),
            "hover": mission.model.hover().map(|t| format!("{t:?}")),
            "drag": mission.model.drag().map(|d| format!("{:?}", d.over)),
            "looking": mission.model.looking(),
            "progress": mission.progress.value(),
            "closing": mission.closing,
            "thumbs": mission.thumbs.len(),
        })
        .to_string()
    }
}
