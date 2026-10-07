//! Cmd+Tab: the hold, the keys read while it lasts, and the commit (`p7-switcher.md`
//! choices 1 to 6). The switcher never takes the seat's focus.

use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use isle_shell::switcher::{
    Frame, SwitcherAction, SwitcherSurface,
    model::{self, Effect, Entry, Key, Metrics, Switcher, Window},
    surface::PreviewImage,
};
use smithay::{
    input::keyboard::{Keysym, keysyms},
    output::Output,
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::{Logical, Point, Rectangle},
};
use tracing::{debug, info};

use crate::state::State;

/// The switcher while its chord is held.
pub struct SwitcherState {
    pub model: Switcher,
    /// The output it opened on: the focused window's, else the pointer's.
    pub output: Option<Output>,
    timer: Option<RegistrationToken>,
    /// The panel's top edge on its output once drawn: the window row grows it downward.
    top: Option<i32>,
    /// Each previewed window's image, and the windows that committed since it was made.
    previews: HashMap<u64, PreviewImage>,
    pub stale: HashSet<u64>,
    /// Where the pointer was when the panel appeared; hover waits for it to move.
    pointer_at: Option<Point<f64, Logical>>,
}

impl SwitcherState {
    pub fn drawn(&self) -> bool {
        self.model.drawn()
    }
}

impl crate::state::Isle {
    /// The switcher's entries as the apps are now.
    pub fn switcher_entries(&self) -> Vec<Entry> {
        self.app_infos()
            .into_iter()
            .map(|info| {
                let windows: Vec<Window> = self
                    .app_windows(&info.app)
                    .into_iter()
                    .filter_map(|id| self.wm.get(id))
                    .map(|r| Window {
                        id: r.id,
                        title: r.names().1,
                        minimised: r.minimised,
                    })
                    .collect();
                let recent = self
                    .wm
                    .recent
                    .iter()
                    .copied()
                    .find(|id| windows.iter().any(|w| w.id == *id))
                    .or(windows.last().map(|w| w.id));
                Entry {
                    cover: self
                        .games
                        .for_app(&info.app)
                        .is_some_and(|g| g.cover.is_some()),
                    name: info.name,
                    icon: (!info.icon.is_empty()).then(|| info.icon.into()),
                    windows,
                    recent,
                    hidden: info.hidden,
                    minimised: info.minimised,
                    background: info.background,
                    not_responding: info.not_responding,
                    attention: info.attention,
                    urgent: info.urgent,
                    badge: (info.badge >= 0).then_some(info.badge),
                    progress: (info.progress >= 0.0).then_some(info.progress),
                    app: info.app,
                }
            })
            .collect()
    }
}

/// What a key is to the switcher by its symbol in the seat's keymap: `None` for a modifier,
/// which goes on to the engine so the release is seen.
fn switcher_key(
    role: isle_keys::keyinfo::Role,
    sym: Option<Keysym>,
    shift: bool,
) -> Option<Option<Key>> {
    use isle_keys::keyinfo::Role;
    if role != Role::Key {
        return None;
    }
    let key = match sym.map(|s| s.raw()) {
        Some(keysyms::KEY_Tab | keysyms::KEY_ISO_Left_Tab) if shift => Key::Previous,
        Some(keysyms::KEY_Tab | keysyms::KEY_ISO_Left_Tab) => Key::Next,
        Some(keysyms::KEY_Left) => Key::Left,
        Some(keysyms::KEY_Right) => Key::Right,
        Some(keysyms::KEY_Up) => Key::Up,
        Some(keysyms::KEY_Down) => Key::Down,
        Some(keysyms::KEY_Return | keysyms::KEY_KP_Enter) => Key::Return,
        Some(keysyms::KEY_Escape) => Key::Escape,
        Some(keysyms::KEY_q) => Key::Quit,
        Some(keysyms::KEY_h) => Key::Hide,
        Some(keysyms::KEY_w) => Key::Close,
        Some(keysyms::KEY_grave) => Key::Windows { forward: !shift },
        _ => return Some(None),
    };
    Some(Some(key))
}

impl State {
    /// Cmd+Tab or Shift+Cmd+Tab pressed: the switcher opens on the apps as they are, and
    /// nothing moves until the chord is let go.
    pub fn open_switcher(&mut self, backwards: bool) {
        if self.isle.switcher.is_some() {
            return;
        }
        self.close_menu_bar_group();
        let output = self
            .isle
            .output_with_focus()
            .or_else(|| self.isle.active_output());
        let front = self.isle.front_app();
        let delay =
            Duration::from_millis(self.isle.shell.theme().tokens.timing_switcher_delay_ms as u64);
        let now = Instant::now();
        let model = Switcher::open(
            self.isle.switcher_entries(),
            front.as_deref(),
            backwards,
            delay,
            now,
        );
        debug!(
            selected = model.selected().map(|e| e.app.as_str()),
            "switcher held"
        );
        self.isle.switcher = Some(SwitcherState {
            model,
            output,
            timer: None,
            top: None,
            previews: HashMap::new(),
            stale: HashSet::new(),
            pointer_at: None,
        });
        self.arm_switcher();
    }

    fn arm_switcher(&mut self) {
        let Some(switcher) = self.isle.switcher.as_mut() else {
            return;
        };
        if let Some(token) = switcher.timer.take() {
            self.isle.loop_handle.remove(token);
        }
        let Some(at) = switcher.model.next_change() else {
            return;
        };
        let wait = at.saturating_duration_since(Instant::now());
        let token =
            self.isle
                .loop_handle
                .insert_source(Timer::from_duration(wait), |_, _, state| {
                    if let Some(switcher) = state.isle.switcher.as_mut() {
                        switcher.timer = None;
                        if switcher.model.tick(Instant::now()) {
                            state.switcher_changed();
                        }
                    }
                    TimeoutAction::Drop
                });
        if let Ok(token) = token {
            self.isle.switcher.as_mut().unwrap().timer = Some(token);
        }
    }

    /// A key pressed while the switcher is held; true when the switcher took it.
    pub fn switcher_key(&mut self, role: isle_keys::keyinfo::Role, sym: Option<Keysym>) -> bool {
        if self.isle.switcher.is_none() {
            return false;
        }
        let shift = self.held_mods().shift;
        let Some(key) = switcher_key(role, sym, shift) else {
            return false;
        };
        if let Some(key) = key {
            let effects = self
                .isle
                .switcher
                .as_mut()
                .unwrap()
                .model
                .key(key, Instant::now());
            self.switcher_effects(effects);
        }
        true
    }

    /// The chord was let go: the selection comes forward.
    pub fn switcher_released(&mut self) {
        let Some(switcher) = self.isle.switcher.as_mut() else {
            return;
        };
        let effects = switcher.model.commit();
        self.switcher_effects(effects);
    }

    /// Ends the hold without a commit: a switch of console, the lock, the keyboard gone.
    pub fn cancel_switcher(&mut self) {
        if let Some(switcher) = self.isle.switcher.as_mut() {
            switcher.model.cancel();
            self.close_switcher();
        }
    }

    fn switcher_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            info!(?effect, "switcher");
            match effect {
                Effect::Activate(app) => {
                    self.close_switcher();
                    self.activate_app(&app);
                }
                Effect::ActivateWindow(id) => {
                    self.close_switcher();
                    self.activate(id);
                }
                Effect::Relaunch(app) => {
                    self.close_switcher();
                    self.launch(crate::launch::Launch::App {
                        app,
                        action: None,
                        uris: Vec::new(),
                    });
                }
                Effect::Quit(app) => {
                    self.quit_app(&app);
                }
                Effect::Stop(app) => {
                    self.stop_app(&app);
                }
                Effect::Hide(app, hidden) => {
                    self.set_app_hidden(&app, hidden);
                }
                Effect::Close(id) => {
                    self.close_window(id);
                }
            }
        }
        if self.isle.switcher.as_ref().is_some_and(|s| s.model.done()) {
            self.close_switcher();
        } else {
            self.refresh_switcher();
        }
    }

    fn close_switcher(&mut self) {
        let Some(mut switcher) = self.isle.switcher.take() else {
            return;
        };
        if let Some(token) = switcher.timer.take() {
            self.isle.loop_handle.remove(token);
        }
        debug!("switcher closed");
        self.switcher_closed(switcher);
    }

    /// The apps changed while the chord is held.
    pub fn refresh_switcher(&mut self) {
        if self.isle.switcher.is_none() {
            return;
        }
        let entries = self.isle.switcher_entries();
        self.isle.switcher.as_mut().unwrap().model.refresh(entries);
        self.switcher_changed();
    }

    /// Run once per loop iteration: a change of windows while held reaches the switcher.
    pub fn switcher_after_dispatch(&mut self) {
        if self.isle.switcher_dirty.take() {
            self.refresh_switcher();
        } else if self
            .isle
            .switcher
            .as_ref()
            .is_some_and(|s| s.drawn() && !s.stale.is_empty())
        {
            self.switcher_changed();
        }
    }

    /// Something the switcher shows changed: the panel is drawn again from the model.
    fn switcher_changed(&mut self) {
        let Some(switcher) = &self.isle.switcher else {
            return;
        };
        if !switcher.drawn() {
            return;
        }
        #[cfg(feature = "test-hooks")]
        {
            self.isle.switcher_drawn_count += u32::from(!self.isle.switcher_was_drawn);
            self.isle.switcher_was_drawn = true;
        }
        let Some(output) = switcher.output.clone() else {
            return;
        };
        let Some(screen) = self.isle.space.output_geometry(&output) else {
            return;
        };
        let theme = self.isle.shell.theme().clone();
        let t = &theme.tokens;
        let metrics = Metrics {
            card_width: t.size_switcher_card_width,
            card_height: t.size_switcher_card_height,
            card_min_width: t.size_switcher_card_min_width,
            background_width: t.size_switcher_background_width,
            space_xs: t.space_xs,
            space_s: t.space_s,
            space_l: t.space_l,
            space_xxl: t.space_xxl,
            band: t
                .size_row
                .max(isle_ui::text::metrics(&theme, isle_ui::text::Style::Headline).1 + t.space_m),
            icon_overhang: t.size_app_icon_medium / 2.0,
            progress: 6.0,
        };
        let model = &switcher.model;
        let row_of = model.row().map(|_| model.selected_index());
        let windows = model.selected().map_or(0, |e| e.windows.len());
        let layout = model::layout(
            model.entries(),
            screen.size.w as f32,
            &metrics,
            row_of,
            if row_of.is_some() { windows } else { 0 },
        );
        self.update_previews(&layout);
        let game = self.isle.island_game_mode();
        let pointer = self.isle.seat.get_pointer().map(|p| p.current_location());
        let Some(switcher) = &mut self.isle.switcher else {
            return;
        };
        let model = &switcher.model;
        let frame = Frame {
            entries: model.entries().to_vec(),
            selected: model.selected_index(),
            row: model.row(),
            quitting: model
                .entries()
                .iter()
                .filter(|e| model.quitting(&e.app))
                .map(|e| e.app.clone())
                .collect(),
            previews: switcher
                .previews
                .iter()
                .map(|(id, p)| (*id, p.clone()))
                .collect(),
            solid: theme.appearance.reduce_transparency || game,
            layout: layout.clone(),
        };
        let size = SwitcherSurface::size_for(&layout, &theme);
        let margin = SwitcherSurface::margin(&theme) as i32;
        if switcher.top.is_none() {
            switcher.pointer_at = pointer;
        }
        // Centred on the screen as it appears; then its top stays and it grows downward.
        let top = *switcher
            .top
            .get_or_insert_with(|| (screen.size.h - layout.rows_height.ceil() as i32) / 2 - margin);
        let geometry = Rectangle::new(((screen.size.w - size.w) / 2, top).into(), size);

        let surface = self.isle.switcher_surface.clone();
        surface.set_size(size);
        surface.set_still(game);
        surface.set_model(frame.clone());
        self.isle
            .shell
            .with_root("switcher", || surface.set(frame.clone()));
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        if !self.isle.shell.is_open("switcher") {
            self.close_menu_bar_group();
        }
        self.isle.shell.open_placed("switcher", &place, geometry);
        self.dispatch_shell();
        self.isle.queue_redraw(&output);
    }

    /// Renders a preview for each plate's window that has none or has committed since, at
    /// its card's size: the card's window for each app, and the window row's.
    fn update_previews(&mut self, layout: &model::Layout) {
        let Some(switcher) = &self.isle.switcher else {
            return;
        };
        let Some(output) = switcher.output.clone() else {
            return;
        };
        let scale = output.current_scale().fractional_scale();
        let mut wanted: Vec<u64> = switcher
            .model
            .entries()
            .iter()
            .filter(|e| !e.background)
            .filter_map(|e| e.recent)
            .collect();
        if switcher.model.row().is_some()
            && let Some(entry) = switcher.model.selected()
        {
            wanted.extend(entry.windows.iter().map(|w| w.id));
        }
        let todo: Vec<u64> = wanted
            .iter()
            .copied()
            .filter(|id| !switcher.previews.contains_key(id) || switcher.stale.contains(id))
            .collect();
        let (w, h) = (layout.card_width as f64, layout.card_height as f64);
        for id in todo {
            if let Some(image) = self.render_preview(id, w, h, scale) {
                let switcher = self.isle.switcher.as_mut().unwrap();
                switcher.previews.insert(id, PreviewImage(image));
            }
        }
        let switcher = self.isle.switcher.as_mut().unwrap();
        switcher.stale.clear();
        switcher.previews.retain(|id, _| wanted.contains(id));
    }

    /// Window `id` drawn into a texture fitted inside `w` by `h` points, as an image the
    /// switcher's tree draws. A window not on screen gives its last buffer.
    fn render_preview(
        &mut self,
        id: u64,
        w: f64,
        h: f64,
        scale: f64,
    ) -> Option<isle_shell::switcher::surface::ImageHandle> {
        let frame = self.isle.frame(id)?;
        let fit = (w / frame.size.w.max(1) as f64).min(h / frame.size.h.max(1) as f64);
        let (pw, ph) = (
            (frame.size.w as f64 * fit).round().max(1.0),
            (frame.size.h as f64 * fit).round().max(1.0),
        );
        let renderer = self.backend.renderer_opt()?;
        let mut pieces = Vec::new();
        self.isle
            .window_pieces(renderer, id, frame.loc, scale, false, 1.0, &mut |e| {
                pieces.push(e)
            });
        if pieces.is_empty() {
            return None;
        }
        let from = Rectangle::from_size(frame.size);
        let to = Rectangle::<f64, Logical>::from_size((pw, ph).into());
        let placed: Vec<_> = pieces
            .into_iter()
            .map(|p| crate::wm::draw::place(p, from, to, scale))
            .collect();
        let size = ((pw * scale).round() as i32, (ph * scale).round() as i32).into();
        let texture = match crate::wm::motion::render_to_texture(renderer, size, scale, &placed) {
            Ok(texture) => texture,
            Err(err) => {
                debug!(id, "no preview: {err}");
                return None;
            }
        };
        self.isle.shell.image_of(renderer, &texture)
    }

    /// The pointer in the panel: hover selects once it has moved, a click commits.
    pub fn switcher_pointer(&mut self, action: SwitcherAction) {
        let Some(switcher) = self.isle.switcher.as_mut() else {
            return;
        };
        match action {
            SwitcherAction::Hover { index, window } => {
                let now = self.isle.seat.get_pointer().map(|p| p.current_location());
                if switcher.pointer_at.is_some() && switcher.pointer_at == now {
                    return;
                }
                switcher.pointer_at = None;
                switcher.model.hover(index, window);
                self.switcher_changed();
            }
            SwitcherAction::Click { index, window } => {
                let effects = switcher.model.click(index, window);
                self.switcher_effects(effects);
            }
        }
    }

    fn switcher_closed(&mut self, _switcher: SwitcherState) {
        #[cfg(feature = "test-hooks")]
        {
            self.isle.switcher_was_drawn = false;
        }
        if self.isle.shell.is_open("switcher") {
            self.isle.shell.close("switcher");
            self.dispatch_shell();
            self.isle.queue_redraw_all();
        }
    }

    /// Opening a centre surface, the switcher included, closes every menu-bar surface: the
    /// bar and its menus, Control Centre, the window menu, About and the zoom menu.
    pub fn close_menu_bar_group(&mut self) {
        if self.isle.bar.output.is_some() {
            self.hide_bar();
        }
        if self.isle.island.panel.is_some() {
            self.close_control_centre();
        }
        for name in ["window-menu", "about"] {
            if self.isle.shell.is_open(name) {
                self.close_surface(name);
            }
        }
        self.close_zoom_menu();
    }

    /// The switcher's state for tests and `isle`: the entries, the selection, the window row.
    pub fn switcher_json(&self) -> String {
        let Some(switcher) = &self.isle.switcher else {
            return "null".into();
        };
        let entries: Vec<serde_json::Value> = switcher
            .model
            .entries()
            .iter()
            .map(|e| {
                serde_json::json!({
                    "app": e.app,
                    "name": e.name,
                    "windows": e.windows.iter().map(|w| w.id).collect::<Vec<_>>(),
                    "background": e.background,
                    "hidden": e.hidden,
                    "quitting": switcher.model.quitting(&e.app),
                })
            })
            .collect();
        serde_json::json!({
            "entries": entries,
            "selected": switcher.model.selected().map(|e| e.app.clone()),
            "row": switcher.model.row(),
            "drawn": switcher.drawn(),
            "output": switcher.output.as_ref().map(|o| o.name()),
        })
        .to_string()
    }
}
