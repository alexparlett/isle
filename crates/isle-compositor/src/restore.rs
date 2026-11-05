//! Session restore: the record of what is open, kept from the window list, and its return at
//! each compositor start, each window on the desktop it left (`p20b-session.md` choices 3–12).

use std::{
    cell::Cell,
    collections::HashMap,
    time::{Duration, Instant},
};

use isle_state::{
    Loaded, StateDir,
    domains::{Restore, RestoreApp, RestoreWindow},
};
use smithay::output::Output;
use tracing::{info, warn};

use crate::{
    launch::Launch,
    state::{Isle, State},
    state_writer::Saver,
    wm::desktops::Kind,
};

/// How long a restored app's noted windows wait for it to map them.
pub const PENDING_FOR: Duration = Duration::from_secs(120);

/// A noted window waiting for its app to map it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    pub window: RestoreWindow,
    /// It is the front app's: it takes focus and its desktop is shown.
    pub focus: bool,
    /// Its place in the restored stack, back to front.
    pub rank: (usize, usize),
}

/// A launched app's noted windows, in the order they take the next windows it maps.
struct Pending {
    app: String,
    slots: Vec<Slot>,
    since: Instant,
}

/// How a start's restore is decided: by the supervised start, by a test, or not at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// A run with no supervisor (nested, or a test that does not ask): nothing is recorded
    /// or restored.
    Off,
    /// Held until the start's restore is decided, so the empty desktop of a fresh start is
    /// never written over the record it restores from.
    Deciding,
    Decided,
}

pub struct RestoreState {
    /// The record as last written or loaded.
    pub record: Restore,
    dir: Option<StateDir>,
    saver: Option<Saver<Restore>>,
    pub start: Start,
    /// The session's end has started: nothing is recorded until it is called off.
    pub frozen: bool,
    pub dirty: Cell<bool>,
    pending: Vec<Pending>,
    /// Windows that took a slot, until they map.
    pub slots: HashMap<u64, Slot>,
    /// Restored windows' ranks, so later ones map behind earlier ones only.
    ranks: HashMap<u64, (usize, usize)>,
    /// Activation tokens of restored launches that must not take focus.
    pub quiet_tokens: Vec<(String, Instant)>,
    /// Apps this start's restore launched.
    pub reopened: Vec<String>,
    /// Isle's crash notice waits for the restore, so it can say the apps came back.
    pub crash_note_owed: bool,
    /// Login Items started beside a restore that brought back a front app, whose windows
    /// open behind until the restore's windows stop waiting.
    behind: Vec<String>,
    behind_since: Option<Instant>,
    /// Each app's windows as last seen, kept after they go, for a crashed app's Reopen.
    pub last_windows: HashMap<String, Vec<RestoreWindow>>,
}

impl Default for RestoreState {
    fn default() -> Self {
        Self {
            record: Restore::default(),
            dir: None,
            saver: None,
            start: Start::Off,
            frozen: false,
            dirty: Cell::new(true),
            pending: Vec::new(),
            slots: HashMap::new(),
            ranks: HashMap::new(),
            quiet_tokens: Vec::new(),
            reopened: Vec::new(),
            crash_note_owed: false,
            behind: Vec::new(),
            behind_since: None,
            last_windows: HashMap::new(),
        }
    }
}

/// A window as the record sees it, back to front.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seen {
    pub app: String,
    pub window: RestoreWindow,
}

/// The record's apps from the windows kept, back to front, and the apps running with none: an
/// app's place is its front window's, and windowless apps go behind. The front app is the
/// most recent in `recency` that is kept.
pub fn work_out(windows: &[Seen], background: &[String], recency: &[String]) -> Restore {
    let mut apps: Vec<RestoreApp> = background
        .iter()
        .filter(|a| !windows.iter().any(|w| &w.app == *a))
        .map(|a| RestoreApp {
            id: a.clone(),
            windows: Vec::new(),
        })
        .collect();
    let mut windowed: Vec<(usize, RestoreApp)> = Vec::new();
    for (at, seen) in windows.iter().enumerate() {
        match windowed.iter_mut().find(|(_, a)| a.id == seen.app) {
            Some((last, app)) => {
                *last = at;
                app.windows.push(seen.window.clone());
            }
            None => windowed.push((
                at,
                RestoreApp {
                    id: seen.app.clone(),
                    windows: vec![seen.window.clone()],
                },
            )),
        }
    }
    windowed.sort_by_key(|(last, _)| *last);
    apps.extend(windowed.into_iter().map(|(_, a)| a));
    let front = recency
        .iter()
        .find(|f| apps.iter().any(|a| &a.id == *f))
        .cloned();
    Restore {
        apps,
        front,
        ..Restore::default()
    }
}

/// The index among `kinds`' plain desktops of the one at `at`, or the nearest plain one
/// before it: a full-screen or Split View desktop sits beside the one it came from.
pub fn plain_index(kinds: &[bool], at: usize) -> u32 {
    let upto = at.min(kinds.len().saturating_sub(1));
    kinds[..=upto]
        .iter()
        .filter(|plain| **plain)
        .count()
        .saturating_sub(1) as u32
}

impl Isle {
    pub fn load_restore(&mut self, runtime: &tokio::runtime::Handle, start: Start) {
        let dir = StateDir::new(StateDir::default_dir());
        let (record, loaded) = dir.load::<Restore>();
        if !matches!(loaded, Loaded::Read | Loaded::Missing) {
            warn!(?loaded, "restore");
        }
        self.restore.record = record;
        self.restore.saver = Some(Saver::spawn(runtime, dir.clone()));
        self.restore.dir = Some(dir);
        self.restore.start = start;
    }

    /// Where a desktop is: its output's identity and its plain index there.
    fn desktop_place(&self, desktop: u64) -> Option<(String, u32)> {
        let (identity, at) = self.desktops.locate(desktop)?;
        let kinds: Vec<bool> = self.desktops.rows[identity]
            .desktops
            .iter()
            .map(|d| d.kind == Kind::Normal)
            .collect();
        Some((identity.to_owned(), plain_index(&kinds, at)))
    }

    /// The windows the record keeps, back to front: mapped, titled, without a parent, of an
    /// app with a desktop entry, and not a game.
    fn seen_windows(&self) -> Vec<Seen> {
        self.wm
            .records
            .iter()
            .filter(|r| r.mapped && r.parent.is_none() && !r.game && !r.parked)
            .filter(|r| !r.names().1.is_empty() && self.apps.get(&r.app).is_some())
            .filter_map(|r| {
                let desktop = match &r.mode {
                    crate::wm::Mode::FullScreen { origin, .. } => *origin,
                    _ => r.desktop,
                };
                let (output, desktop) = match &r.home {
                    Some((identity, home, _)) => (
                        identity.clone(),
                        self.desktop_place(*home).map_or(0, |(_, i)| i),
                    ),
                    None => match self.desktop_place(desktop) {
                        Some(place) => place,
                        None => (Self::output_identity(&self.output_of(r.id)?), 0),
                    },
                };
                Some(Seen {
                    app: r.app.clone(),
                    window: RestoreWindow {
                        output,
                        desktop,
                        hidden: r.hidden,
                        minimised: r.minimised,
                    },
                })
            })
            .collect()
    }

    /// The record as the window list stands, with the restore's windows yet to map.
    pub fn work_out_restore(&self) -> Restore {
        let mut record = if self.settings.windows.reopen_apps {
            let background: Vec<String> = self
                .processes
                .seen()
                .filter(|a| self.apps.get(a).is_some())
                .filter(|a| !self.wm.records.iter().any(|r| r.mapped && &r.app == *a))
                .cloned()
                .collect();
            work_out(&self.seen_windows(), &background, &self.apps_by_recency())
        } else {
            Restore::default()
        };
        for pending in &self.restore.pending {
            let slots = pending.slots.iter().map(|s| s.window.clone());
            match record.apps.iter_mut().find(|a| a.id == pending.app) {
                Some(app) => app.windows.extend(slots),
                None => record.apps.insert(
                    0,
                    RestoreApp {
                        id: pending.app.clone(),
                        windows: slots.collect(),
                    },
                ),
            }
        }
        record.skip = self.restore.record.skip;
        record.reopen_windows = self.restore.record.reopen_windows;
        record
    }

    /// The output a window opens on: its slot's when connected, else the main screen; any
    /// other window opens on the output with focus.
    pub fn opening_output(&self, id: u64) -> Option<Output> {
        let Some(slot) = self.restore.slots.get(&id) else {
            return self.output_with_focus();
        };
        self.space
            .outputs()
            .find(|o| Self::output_identity(o) == slot.window.output)
            .cloned()
            .or_else(|| self.main_output())
    }

    /// Whether a new window of `app` opens behind: a Login Item's, beside a restore that
    /// brought the front app back.
    pub fn opens_behind(&self, app: &str) -> bool {
        self.restore.behind.iter().any(|a| a == app)
            && self
                .restore
                .behind_since
                .is_some_and(|at| at.elapsed() < PENDING_FOR)
    }

    /// Whether `token` is a restored launch's that opens behind.
    pub fn quiet_token(&mut self, token: &str) -> bool {
        self.restore
            .quiet_tokens
            .retain(|(_, at)| at.elapsed() < PENDING_FOR);
        self.restore.quiet_tokens.iter().any(|(t, _)| t == token)
    }

    pub fn restore_status(&self) -> String {
        serde_json::json!({
            "record": self.restore.record,
            "start": format!("{:?}", self.restore.start),
            "frozen": self.restore.frozen,
            "pending": self.restore.pending.iter().map(|p| {
                serde_json::json!({ "app": p.app, "slots": p.slots.len() })
            }).collect::<Vec<_>>(),
            "reopened": self.restore.reopened,
        })
        .to_string()
    }
}

impl State {
    /// After each dispatch: the record worked out again if the windows changed, and written
    /// only when it differs.
    pub fn restore_after_dispatch(&mut self) {
        if !self.isle.restore.dirty.take() {
            return;
        }
        let mut seen: HashMap<String, Vec<RestoreWindow>> = HashMap::new();
        for window in self.isle.seen_windows() {
            seen.entry(window.app).or_default().push(window.window);
        }
        self.isle.restore.last_windows.extend(seen);
        if self.isle.restore.frozen || self.isle.restore.start != Start::Decided {
            return;
        }
        self.isle
            .restore
            .pending
            .retain(|p| p.since.elapsed() < PENDING_FOR);
        let record = self.isle.work_out_restore();
        if record != self.isle.restore.record {
            self.isle.restore.record = record.clone();
            if let Some(saver) = &self.isle.restore.saver {
                saver.save(record);
            }
        }
    }

    /// The session's end started: the record as it is now is kept, written at once, with the
    /// dialog's checkbox and the skip it sets.
    pub fn freeze_restore(&mut self, reopen_windows: Option<bool>) {
        if self.isle.restore.frozen || self.isle.restore.start == Start::Off {
            return;
        }
        if self.isle.restore.start == Start::Decided {
            self.isle.restore.record = self.isle.work_out_restore();
        }
        if let Some(reopen) = reopen_windows {
            self.isle.restore.record.reopen_windows = reopen;
            self.isle.restore.record.skip = !reopen;
        }
        self.isle.restore.frozen = true;
        info!(
            skip = self.isle.restore.record.skip,
            "the restore record frozen"
        );
        if let Some(dir) = &self.isle.restore.dir
            && let Err(err) = dir.save(&self.isle.restore.record)
        {
            warn!("could not write the restore record: {err}");
        }
    }

    /// The session's end called off or stopped: recording carries on.
    pub fn thaw_restore(&mut self) {
        if !std::mem::take(&mut self.isle.restore.frozen) {
            return;
        }
        info!("the restore record thawed");
        self.isle.restore.record.skip = false;
        self.isle.restore.dirty.set(true);
        if let Some(saver) = &self.isle.restore.saver {
            saver.save(self.isle.restore.record.clone());
        }
    }

    /// Whether this start restores, and why not.
    pub fn restore_refused(&self, sign_in: bool) -> Option<&'static str> {
        if crate::greeter::on() {
            Some("the greeter")
        } else if self.isle.safe_start {
            Some("a safe start")
        } else if std::env::var("ISLE_RESTORE").as_deref() == Ok("0") {
            Some("ISLE_RESTORE=0")
        } else if !self.isle.settings.windows.reopen_apps {
            Some("Reopen apps is off")
        } else if sign_in && self.isle.restore.record.skip {
            Some("the last log out asked not to")
        } else {
            None
        }
    }

    /// The start's restore, once Login Items are decided: `skip` names the apps something
    /// else starts or that still run, `items` the Login Items just started. The skip is
    /// cleared whatever happens.
    pub fn restore_at_start(&mut self, skip: &[String], items: Vec<String>) -> Vec<String> {
        let sign_in = std::env::var("ISLE_RESTART").as_deref() != Ok("1");
        let refused = self.restore_refused(sign_in);
        let record = std::mem::take(&mut self.isle.restore.record);
        self.isle.restore.record.reopen_windows = record.reopen_windows;
        self.isle.restore.start = Start::Decided;
        self.isle.restore.dirty.set(true);
        let mut launched = Vec::new();
        match refused {
            Some(why) => info!(why, "nothing restored"),
            None => {
                for (rank, app) in record.apps.iter().enumerate() {
                    if skip.contains(&app.id) {
                        info!(app = app.id, "not restored: started or running already");
                        continue;
                    }
                    let focus = record.front.as_deref() == Some(app.id.as_str());
                    let slots = app
                        .windows
                        .iter()
                        .enumerate()
                        .map(|(n, w)| Slot {
                            window: w.clone(),
                            focus,
                            rank: (rank, n),
                        })
                        .collect();
                    if self.reopen_with(&app.id, slots, focus) {
                        launched.push(app.id.clone());
                    }
                }
            }
        }
        info!(?launched, "restored");
        self.isle.restore.reopened = launched.clone();
        let front = record.front.as_ref().is_some_and(|f| launched.contains(f));
        self.isle.restore.behind = if front { items } else { Vec::new() };
        self.isle.restore.behind_since = front.then(Instant::now);
        self.restore_after_dispatch();
        // Written whether or not it changed, so a skip used here is cleared on disk too.
        if let Some(saver) = &self.isle.restore.saver {
            saver.save(self.isle.restore.record.clone());
        }
        if std::mem::take(&mut self.isle.restore.crash_note_owed) {
            self.crash_note();
        }
        launched
    }

    /// Launches `app` with `slots` for the windows it maps next.
    pub fn reopen_with(&mut self, app: &str, slots: Vec<Slot>, focus: bool) -> bool {
        self.isle.restore.pending.retain(|p| p.app != app);
        if !self.launch(Launch::Restore {
            app: app.to_owned(),
            focus,
        }) {
            return false;
        }
        if !slots.is_empty() {
            self.isle.restore.pending.push(Pending {
                app: app.to_owned(),
                slots,
                since: Instant::now(),
            });
        }
        true
    }

    /// Before a window's first configure, or at its map: a parentless window of an app with
    /// noted windows takes the next.
    pub fn take_restore_slot(&mut self, id: u64) {
        if self.isle.restore.slots.contains_key(&id) {
            return;
        }
        let Some(record) = self.isle.wm.get(id) else {
            return;
        };
        if record.parent.is_some() || record.mapped || record.app.is_empty() {
            return;
        }
        let app = record.app.clone();
        self.isle
            .restore
            .pending
            .retain(|p| p.since.elapsed() < PENDING_FOR);
        let Some(at) = self.isle.restore.pending.iter().position(|p| p.app == app) else {
            return;
        };
        let pending = &mut self.isle.restore.pending[at];
        let slot = pending.slots.remove(0);
        if pending.slots.is_empty() {
            self.isle.restore.pending.remove(at);
        }
        info!(id, app, ?slot, "a restored window");
        self.isle.restore.slots.insert(id, slot);
    }

    /// At map, in place of the current desktop: the slot's desktop on its output, clamped to
    /// the row. Its desktop is shown only for the front app's window. False with no slot.
    pub fn assign_restored_desktop(&mut self, id: u64) -> bool {
        let Some(slot) = self.isle.restore.slots.get(&id).cloned() else {
            return false;
        };
        let Some(output) = self.isle.opening_output(id) else {
            return false;
        };
        let identity = Isle::output_identity(&output);
        let row = self.isle.desktops.ensure_row(&identity);
        let plain: Vec<(usize, u64)> = row
            .desktops
            .iter()
            .enumerate()
            .filter(|(_, d)| d.kind == Kind::Normal)
            .map(|(i, d)| (i, d.id))
            .collect();
        let Some(&(index, desktop)) = plain
            .get(slot.window.desktop as usize)
            .or_else(|| plain.last())
        else {
            return false;
        };
        if slot.focus && !slot.window.hidden && !slot.window.minimised {
            row.current = index;
        }
        let record = self.isle.wm.get_mut(id).unwrap();
        record.desktop = desktop;
        record.hidden = slot.window.hidden;
        record.minimised = slot.window.minimised;
        true
    }

    /// At map, once its desktop is set: a restored window goes behind the windows not
    /// restored and those restored after it. Whether it takes focus.
    pub fn restored_mapped(&mut self, id: u64) -> Option<bool> {
        let slot = self.isle.restore.slots.remove(&id)?;
        self.isle
            .restore
            .ranks
            .retain(|w, _| self.isle.wm.get(*w).is_some());
        self.isle.restore.ranks.insert(id, slot.rank);
        let ranks = &self.isle.restore.ranks;
        let records = &mut self.isle.wm.records;
        // The front app's windows stay in front, where they mapped.
        if let Some(from) = records
            .iter()
            .position(|r| r.id == id)
            .filter(|_| !slot.focus)
        {
            let record = records.remove(from);
            let to = records
                .iter()
                .position(|r| r.mapped && ranks.get(&r.id).is_none_or(|rank| *rank > slot.rank))
                .unwrap_or(records.len());
            records.insert(to, record);
        }
        self.isle.restore.dirty.set(true);
        Some(slot.focus && !slot.window.hidden && !slot.window.minimised)
    }

    /// The window went before it mapped: its slot is lost with it.
    pub fn restore_window_gone(&mut self, id: u64) {
        self.isle.restore.slots.remove(&id);
        self.isle.restore.ranks.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(app: &str, output: &str, desktop: u32) -> Seen {
        Seen {
            app: app.into(),
            window: RestoreWindow {
                output: output.into(),
                desktop,
                ..Default::default()
            },
        }
    }

    #[test]
    fn apps_go_back_to_front_by_their_front_window() {
        let windows = [
            seen("a", "one", 0),
            seen("b", "one", 1),
            seen("a", "two", 2),
            seen("c", "one", 0),
        ];
        let record = work_out(
            &windows,
            &["d".into(), "a".into()],
            &["c".into(), "b".into()],
        );
        let ids: Vec<&str> = record.apps.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["d", "b", "a", "c"]);
        assert_eq!(record.apps[2].windows.len(), 2);
        assert_eq!(record.apps[2].windows[1].output, "two");
        assert!(record.apps[0].windows.is_empty());
        assert_eq!(record.front.as_deref(), Some("c"));
    }

    #[test]
    fn the_front_app_is_the_most_recent_kept() {
        let record = work_out(&[seen("a", "one", 0)], &[], &["game".into(), "a".into()]);
        assert_eq!(record.front.as_deref(), Some("a"));
    }

    #[test]
    fn a_full_screen_desktop_counts_as_the_one_beside_it() {
        // plain, plain, full screen, plain, split
        let kinds = [true, true, false, true, false];
        assert_eq!(plain_index(&kinds, 0), 0);
        assert_eq!(plain_index(&kinds, 1), 1);
        assert_eq!(plain_index(&kinds, 2), 1);
        assert_eq!(plain_index(&kinds, 3), 2);
        assert_eq!(plain_index(&kinds, 4), 2);
        assert_eq!(plain_index(&[false, true], 0), 0);
    }
}
