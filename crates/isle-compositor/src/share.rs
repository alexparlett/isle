//! Screen sharing through the portal: the picker, the sessions and their streams, what a
//! share keeps to come back without asking, and the island's Sharing row with Stop
//! (`p11-capture.md` choices 23–26).

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use isle_portal::screencast::{Selection, Started, StreamOut};
use isle_shell::capture::picker::{Model, PickerAction, Preview, Tab, Tile};
use smithay::{
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::Rectangle,
};
use tokio::sync::oneshot;
use tracing::info;
use zvariant::{OwnedValue, Value};

use crate::{
    cast::{CursorMode, Ended, Source},
    state::{Isle, State},
};

/// Restore data's vendor and version.
const VENDOR: &str = "Isle";
const VERSION: u32 = 1;
/// How soon a second ask from the same app is the same share: Chromium asks once to
/// preview and again to capture.
const SECOND_ASK: Duration = Duration::from_secs(10);
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
const PREVIEW_EVERY: Duration = Duration::from_millis(500);

/// What the user chose.
pub struct Picked {
    pub sources: Vec<Source>,
    pub remember: bool,
}

/// One app's share: its streams, by id.
pub struct Session {
    pub app: String,
    pub app_id: String,
    pub streams: Vec<u64>,
}

struct Picking {
    model: Model,
    reply: Option<oneshot::Sender<Option<Picked>>>,
    last_click: Option<(String, Instant)>,
    previews: HashMap<String, Preview>,
    timer: Option<RegistrationToken>,
}

struct Grant {
    app_id: String,
    types: u32,
    at: Instant,
    sources: Vec<Source>,
}

#[derive(Default)]
pub struct ShareState {
    pub sessions: HashMap<String, Session>,
    picking: Option<Picking>,
    last: Option<Grant>,
}

/// A source as restore data keeps it: a screen by its identity, a window by its app and
/// title.
fn spec(isle: &Isle, source: &Source) -> Option<String> {
    Some(match source {
        Source::Screen(o) => format!("screen:{}", Isle::output_identity(o)),
        Source::Portion(o, r) => format!(
            "portion:{}\t{},{},{},{}",
            Isle::output_identity(o),
            r.loc.x,
            r.loc.y,
            r.size.w,
            r.size.h
        ),
        Source::Window(id) => {
            let record = isle.wm.get(*id)?;
            let title = record
                .window
                .toplevel()
                .and_then(|t| {
                    smithay::wayland::compositor::with_states(t.wl_surface(), |s| {
                        s.data_map
                            .get::<smithay::wayland::shell::xdg::XdgToplevelSurfaceData>()
                            .and_then(|d| d.lock().unwrap().title.clone())
                    })
                })
                .unwrap_or_default();
            format!("window:{}\t{}", record.app, title)
        }
    })
}

/// The source `spec` names now, if it is there.
fn resolve(isle: &Isle, spec: &str) -> Option<Source> {
    let (kind, rest) = spec.split_once(':')?;
    let output = |identity: &str| {
        isle.space
            .outputs()
            .find(|o| Isle::output_identity(o) == identity)
            .cloned()
    };
    match kind {
        "screen" => output(rest).map(Source::Screen),
        "portion" => {
            let (identity, rect) = rest.split_once('\t')?;
            let n: Vec<i32> = rect.split(',').filter_map(|v| v.parse().ok()).collect();
            let [x, y, w, h] = n[..] else { return None };
            Some(Source::Portion(
                output(identity)?,
                Rectangle::new((x, y).into(), (w, h).into()),
            ))
        }
        "window" => {
            let (app, title) = rest.split_once('\t')?;
            let shown: Vec<u64> = isle
                .wm
                .records
                .iter()
                .filter(|r| r.app == app && isle.wm.is_shown(r.id))
                .map(|r| r.id)
                .collect();
            let titled = shown.iter().copied().find(|id| {
                isle.window_infos()
                    .iter()
                    .any(|w| w.id == *id && w.title == title)
            });
            titled
                .or_else(|| shown.first().copied())
                .map(Source::Window)
        }
        _ => None,
    }
}

fn source_key(source: &Source) -> String {
    match source {
        Source::Screen(o) => format!("screen:{}", o.name()),
        Source::Portion(o, _) => format!("portion:{}", o.name()),
        Source::Window(id) => format!("window:{id}"),
    }
}

impl State {
    /// A ScreenCast Start: its sources from the restore data, a second ask, or the picker.
    pub fn share_begin(
        &mut self,
        app_id: String,
        selection: Selection,
        reply: oneshot::Sender<Option<Picked>>,
    ) {
        if self.isle.lock.locked {
            let _ = reply.send(None);
            return;
        }
        if let Some((vendor, version, payload)) = &selection.restore
            && vendor == VENDOR
            && *version == VERSION
            && let Ok(specs) = Vec::<String>::try_from(
                payload
                    .try_clone()
                    .unwrap_or_else(|_| OwnedValue::from(0u32)),
            )
        {
            let sources: Option<Vec<Source>> =
                specs.iter().map(|s| resolve(&self.isle, s)).collect();
            if let Some(sources) = sources.filter(|s| !s.is_empty()) {
                info!(app_id, "a share restored");
                let _ = reply.send(Some(Picked {
                    sources,
                    remember: true,
                }));
                return;
            }
        }
        if let Some(last) = &self.isle.share.last
            && !app_id.is_empty()
            && last.app_id == app_id
            && last.types == selection.types
            && last.at.elapsed() < SECOND_ASK
        {
            info!(app_id, "the second ask of a share");
            let _ = reply.send(Some(Picked {
                sources: last.sources.clone(),
                remember: false,
            }));
            return;
        }
        self.share_cancel();
        let app = self.isle.app_name(app_id.trim_end_matches(".desktop"));
        let app = if app.is_empty() {
            "the app".to_owned()
        } else {
            app
        };
        let mut tabs = Vec::new();
        if selection.types & isle_portal::screencast::MONITOR != 0 {
            tabs.push(Tab::Screens);
        }
        if selection.types & isle_portal::screencast::WINDOW != 0 {
            tabs.push(Tab::Windows);
        }
        let mut model = Model {
            icon: self.isle.app_icon(app_id.trim_end_matches(".desktop"), 32),
            app,
            tabs,
            multiple: selection.multiple,
            remember: (selection.persist_mode > 0).then_some(true),
            ..Default::default()
        };
        model.tiles = self.share_tiles(model.tabs.first().copied());
        // With one screen and screens only, that screen is chosen already.
        if model.tabs == [Tab::Screens] && model.tiles.len() == 1 {
            model.chosen = vec![model.tiles[0].key.clone()];
        }
        self.isle.share.picking = Some(Picking {
            model,
            reply: Some(reply),
            last_click: None,
            previews: HashMap::new(),
            timer: None,
        });
        self.share_previews();
        self.open_picker();
    }

    fn share_tiles(&self, tab: Option<Tab>) -> Vec<Tile> {
        match tab {
            Some(Tab::Screens) | None => self
                .isle
                .space
                .outputs()
                .map(|o| {
                    let p = o.physical_properties();
                    let size = o.current_mode().map(|m| m.size).unwrap_or_default();
                    Tile {
                        key: format!("screen:{}", o.name()),
                        title: if p.model.is_empty() {
                            o.name()
                        } else {
                            format!("{} {}", p.make, p.model)
                        },
                        subtitle: format!("{} × {}", size.w, size.h),
                        icon: None,
                        preview: None,
                    }
                })
                .collect(),
            Some(Tab::Windows) => {
                let mut windows: Vec<_> = self
                    .isle
                    .window_infos()
                    .into_iter()
                    .filter(|w| self.isle.wm.is_shown(w.id))
                    .collect();
                // Grouped by app, front to back within each.
                windows.sort_by(|a, b| a.app.cmp(&b.app));
                windows
                    .into_iter()
                    .map(|w| Tile {
                        key: format!("window:{}", w.id),
                        title: if w.title.is_empty() {
                            w.app.clone()
                        } else {
                            w.title.clone()
                        },
                        subtitle: w.app.clone(),
                        icon: self.isle.app_icon(&w.app_id, 16),
                        preview: None,
                    })
                    .collect()
            }
        }
    }

    fn open_picker(&mut self) {
        let Some(picking) = &self.isle.share.picking else {
            return;
        };
        let Some(output) = self.isle.main_output() else {
            return;
        };
        let Some(screen) = self.isle.space.output_geometry(&output) else {
            return;
        };
        let mut model = picking.model.clone();
        for tile in &mut model.tiles {
            tile.preview = picking.previews.get(&tile.key).cloned();
        }
        let theme = self.isle.shell.theme().clone();
        let size = isle_shell::capture::picker::PickerSurface::size_for(&theme);
        let geometry = Rectangle::new(
            ((screen.size.w - size.w) / 2, (screen.size.h - size.h) / 3).into(),
            size,
        );
        let surface = self.isle.share_picker.clone();
        surface.set_model(model.clone());
        self.isle
            .shell
            .with_root("share-picker", || surface.set(model.clone()));
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        let was_open = self.isle.shell.is_open("share-picker");
        self.isle
            .shell
            .open_placed("share-picker", &place, geometry);
        if !was_open {
            self.open_surface("share-picker");
        }
        self.dispatch_shell();
    }

    /// Each tile's live picture, again every half second while the picker is open.
    fn share_previews(&mut self) {
        let Some(picking) = &self.isle.share.picking else {
            return;
        };
        let keys: Vec<String> = picking.model.tiles.iter().map(|t| t.key.clone()).collect();
        let w = isle_shell::capture::picker::TILE_WIDTH as f64;
        let h = isle_shell::capture::picker::TILE_PREVIEW as f64;
        let mut previews = HashMap::new();
        for key in keys {
            let image = if let Some(id) = key.strip_prefix("window:").and_then(|i| i.parse().ok()) {
                let scale = self
                    .isle
                    .output_of(id)
                    .map_or(1.0, |o| o.current_scale().fractional_scale());
                self.render_preview(id, w, h, scale)
            } else if let Some(name) = key.strip_prefix("screen:") {
                let output = self
                    .isle
                    .space
                    .outputs()
                    .find(|o| o.name() == name)
                    .cloned();
                output.and_then(|o| {
                    let texture = self.backend.capture_screen(&self.isle, &o, false).ok()?;
                    let renderer = self.backend.renderer_opt()?;
                    self.isle.shell.image_of(renderer, &texture)
                })
            } else {
                None
            };
            if let Some(image) = image {
                previews.insert(key, Preview(image));
            }
        }
        let Some(picking) = self.isle.share.picking.as_mut() else {
            return;
        };
        picking.previews = previews;
        if picking.timer.is_none() {
            picking.timer = self
                .isle
                .loop_handle
                .insert_source(Timer::from_duration(PREVIEW_EVERY), |_, _, state| {
                    if state.isle.share.picking.is_none() {
                        return TimeoutAction::Drop;
                    }
                    state.share_previews();
                    state.open_picker();
                    TimeoutAction::ToDuration(PREVIEW_EVERY)
                })
                .ok();
        }
    }

    pub fn picker_action(&mut self, action: PickerAction) {
        let Some(picking) = self.isle.share.picking.as_mut() else {
            return;
        };
        match action {
            PickerAction::Tab(i) if i < picking.model.tabs.len() => {
                picking.model.tab = i;
                picking.model.focus = None;
                let tab = picking.model.tabs.get(i).copied();
                let tiles = self.share_tiles(tab);
                if let Some(p) = self.isle.share.picking.as_mut() {
                    p.model.tiles = tiles;
                }
                self.share_previews();
            }
            PickerAction::Tab(_) => {}
            PickerAction::Choose(key) => {
                let double = picking
                    .last_click
                    .as_ref()
                    .is_some_and(|(k, at)| *k == key && at.elapsed() < DOUBLE_CLICK);
                picking.last_click = Some((key.clone(), Instant::now()));
                if picking.model.multiple {
                    if let Some(i) = picking.model.chosen.iter().position(|k| *k == key) {
                        picking.model.chosen.remove(i);
                    } else {
                        picking.model.chosen.push(key.clone());
                    }
                } else {
                    picking.model.chosen = vec![key.clone()];
                }
                if double {
                    picking.model.chosen = vec![key];
                    return self.share_picked();
                }
            }
            PickerAction::Step(step) => {
                let n = picking.model.tiles.len() as i32;
                if n == 0 {
                    return;
                }
                if step == 0 {
                    if let Some(key) = picking
                        .model
                        .focus
                        .and_then(|i| picking.model.tiles.get(i))
                        .map(|t| t.key.clone())
                    {
                        return self.picker_action(PickerAction::Choose(key));
                    }
                } else {
                    let at = picking.model.focus.map_or(0, |f| f as i32 + step);
                    picking.model.focus = Some(at.clamp(0, n - 1) as usize);
                }
            }
            PickerAction::Remember(on) => picking.model.remember = Some(on),
            PickerAction::Share => return self.share_picked(),
            PickerAction::Cancel => return self.share_cancel(),
        }
        self.open_picker();
    }

    fn share_picked(&mut self) {
        let Some(mut picking) = self.isle.share.picking.take() else {
            return;
        };
        if picking.model.chosen.is_empty() {
            self.isle.share.picking = Some(picking);
            return;
        }
        let sources: Vec<Source> = picking
            .model
            .chosen
            .iter()
            .filter_map(|key| {
                if let Some(name) = key.strip_prefix("screen:") {
                    self.isle
                        .space
                        .outputs()
                        .find(|o| o.name() == name)
                        .cloned()
                        .map(Source::Screen)
                } else {
                    key.strip_prefix("window:")
                        .and_then(|i| i.parse().ok())
                        .map(Source::Window)
                }
            })
            .collect();
        self.close_picker(&mut picking);
        if let Some(reply) = picking.reply.take() {
            let _ = reply.send(Some(Picked {
                sources,
                remember: picking.model.remember.unwrap_or(false),
            }));
        }
    }

    /// The picker goes and its request is answered cancel.
    pub fn share_cancel(&mut self) {
        if let Some(mut picking) = self.isle.share.picking.take() {
            self.close_picker(&mut picking);
            if let Some(reply) = picking.reply.take() {
                let _ = reply.send(None);
            }
        }
    }

    fn close_picker(&mut self, picking: &mut Picking) {
        if let Some(token) = picking.timer.take() {
            self.isle.loop_handle.remove(token);
        }
        self.close_surface("share-picker");
    }

    /// Starts a stream per source for the session; their ids.
    pub fn share_start(
        &mut self,
        session: &str,
        app_id: &str,
        selection: &Selection,
        picked: &Picked,
    ) -> Option<Vec<u64>> {
        let cursor = if selection.cursor_mode == isle_portal::screencast::CURSOR_EMBEDDED {
            CursorMode::Embedded
        } else {
            CursorMode::Hidden
        };
        let mut ids = Vec::new();
        for source in &picked.sources {
            match self.start_cast(source.clone(), cursor, "portal") {
                Ok(id) => ids.push(id),
                Err(err) => {
                    tracing::warn!("a share's stream: {err}");
                    for id in ids {
                        self.end_cast(id, Ended::Stopped);
                    }
                    return None;
                }
            }
        }
        let app = self.isle.app_name(app_id.trim_end_matches(".desktop"));
        self.isle.share.sessions.insert(
            session.to_owned(),
            Session {
                app: if app.is_empty() { "An app".into() } else { app },
                app_id: app_id.to_owned(),
                streams: ids.clone(),
            },
        );
        self.isle.share.last = Some(Grant {
            app_id: app_id.to_owned(),
            types: selection.types,
            at: Instant::now(),
            sources: picked.sources.clone(),
        });
        self.sharing_activity();
        Some(ids)
    }

    /// The streams' nodes, once every one has its node and serial.
    pub fn share_ready(
        &self,
        ids: &[u64],
        selection: &Selection,
        remember: bool,
    ) -> Option<Started> {
        let mut streams = Vec::new();
        let mut specs = Vec::new();
        for id in ids {
            let stream = self.isle.casting.get(*id)?;
            let node = stream.node()?;
            let serial = self.isle.casting.serial(node);
            let (position, size, source_type) = match &stream.source {
                Source::Screen(o) => {
                    let g = self.isle.space.output_geometry(o)?;
                    (
                        Some((g.loc.x, g.loc.y)),
                        (g.size.w, g.size.h),
                        isle_portal::screencast::MONITOR,
                    )
                }
                Source::Portion(o, r) => {
                    let g = self.isle.space.output_geometry(o)?;
                    (
                        Some((g.loc.x + r.loc.x, g.loc.y + r.loc.y)),
                        (r.size.w, r.size.h),
                        isle_portal::screencast::MONITOR,
                    )
                }
                Source::Window(w) => {
                    let f = self.isle.frame(*w)?;
                    (None, (f.size.w, f.size.h), isle_portal::screencast::WINDOW)
                }
            };
            specs.extend(spec(&self.isle, &stream.source));
            streams.push(StreamOut {
                node,
                serial,
                position,
                size,
                source_type,
                mapping_id: Some(source_key(&stream.source)),
            });
        }
        let persist = if remember { selection.persist_mode } else { 0 };
        let restore = (persist > 0)
            .then(|| OwnedValue::try_from(Value::from(specs)).ok())
            .flatten()
            .map(|payload| (VENDOR.to_owned(), VERSION, payload));
        Some(Started {
            streams,
            persist_mode: if restore.is_some() { persist } else { 0 },
            restore,
        })
    }

    /// The frontend closed the session: its streams end.
    pub fn share_closed(&mut self, session: &str) {
        let Some(s) = self.isle.share.sessions.remove(session) else {
            return;
        };
        for id in s.streams {
            self.end_cast(id, Ended::Stopped);
        }
        self.sharing_activity();
    }

    /// Stop from the island: the session's streams end and its Closed tells the app.
    pub fn share_stop(&mut self, session: &str) {
        self.share_closed(session);
        self.portal_end_session(session.to_owned());
    }

    /// A share's stream ended on its own; the session goes with its last stream.
    pub fn share_stream_ended(&mut self, id: u64, why: Ended) {
        let Some((path, session)) = self
            .isle
            .share
            .sessions
            .iter_mut()
            .find(|(_, s)| s.streams.contains(&id))
        else {
            return;
        };
        session.streams.retain(|s| *s != id);
        let (path, app, empty) = (
            path.clone(),
            session.app.clone(),
            session.streams.is_empty(),
        );
        if empty {
            self.isle.share.sessions.remove(&path);
            self.portal_end_session(path);
        }
        if why != Ended::Stopped {
            use isle_shell::island::model::{Event, EventKind, Form};
            self.isle.island.model.post(
                Event {
                    kind: EventKind::Ended,
                    form: Form::Line {
                        symbol: "screencast".into(),
                        fill: false,
                        text: format!("Sharing with {app} stopped: {}", why.reason()),
                    },
                    output: None,
                    app: None,
                    urgent: false,
                },
                Instant::now(),
            );
        }
        self.sharing_activity();
    }

    fn portal_end_session(&mut self, path: String) {
        if let (Some(conn), Some(worker)) = (self.isle.portal.clone(), self.isle.worker.clone()) {
            worker.spawn(async move { isle_portal::request::end_session(&conn, &path).await });
        }
    }

    /// The island's Sharing activity: each session's app and sources, each with Stop.
    fn sharing_activity(&mut self) {
        use isle_shell::island::model::{Activity, ActivityKind};
        if self.isle.share.sessions.is_empty() {
            self.isle.island.model.end_activity(ActivityKind::Sharing);
            self.isle.refresh_island();
            return;
        }
        let mut sessions: Vec<(&String, &Session)> = self.isle.share.sessions.iter().collect();
        sessions.sort_by_key(|(p, _)| p.as_str());
        let what = |s: &Session| -> String {
            s.streams
                .iter()
                .filter_map(|id| self.isle.casting.get(*id))
                .map(|st| match &st.source {
                    Source::Screen(_) => "Entire screen".to_owned(),
                    Source::Portion(..) => "Portion of screen".to_owned(),
                    Source::Window(w) => self
                        .isle
                        .window_infos()
                        .into_iter()
                        .find(|i| i.id == *w)
                        .map(|i| i.title)
                        .unwrap_or_else(|| "A window".into()),
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        let activity = Activity {
            kind: ActivityKind::Sharing,
            face: sessions[0].1.app.clone(),
            title: "Sharing".into(),
            detail: sessions
                .iter()
                .map(|(_, s)| format!("{}: {}", s.app, what(s)))
                .collect::<Vec<_>>()
                .join("\n"),
            actions: sessions
                .iter()
                .map(|(p, s)| (format!("stop:{p}"), format!("Stop {}", s.app)))
                .collect(),
            app: Some(sessions[0].1.app_id.clone()),
            art: None,
            started: Instant::now(),
        };
        self.isle
            .island
            .model
            .set_activity(activity, Instant::now());
        self.isle.refresh_island();
    }

    /// The share as JSON, for the Test interface.
    #[cfg(feature = "test-hooks")]
    pub fn share_status(&self) -> serde_json::Value {
        let picker = self.isle.share.picking.as_ref().map(|p| {
            serde_json::json!({
                "app": p.model.app,
                "tabs": p.model.tabs.iter().map(|t| format!("{t:?}")).collect::<Vec<_>>(),
                "tiles": p.model.tiles.iter().map(|t| t.key.clone()).collect::<Vec<_>>(),
                "chosen": p.model.chosen,
                "previews": p.previews.len(),
                "remember": p.model.remember,
                "drawn": self.isle.shell.is_open("share-picker"),
            })
        });
        let sessions: serde_json::Map<String, serde_json::Value> = self
            .isle
            .share
            .sessions
            .iter()
            .map(|(p, s)| {
                (
                    p.clone(),
                    serde_json::json!({"app": s.app, "streams": s.streams}),
                )
            })
            .collect();
        serde_json::json!({"picker": picker, "sessions": sessions})
    }
}
