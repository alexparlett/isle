//! Stage Manager as a layout (B25): sets of windows, the front one on the stage and the rest
//! parked in a column at the leading edge (`compositor.md` Stage Manager).

use super::{geometry::Rect, layout::Layout};

/// One set: an app's windows and any the user joined to it, most recent first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Set {
    pub id: u64,
    pub windows: Vec<u64>,
    /// Staged before, so its arrangement is the user's and it is not centred again.
    pub placed: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stage {
    /// Front first.
    pub sets: Vec<Set>,
    /// The column is shown, so the stage keeps its width clear.
    pub column: bool,
    /// The column's width with its margins, kept clear while it shows.
    pub reserve: i32,
    /// Only the front set's most recent window is staged.
    pub one_at_a_time: bool,
}

impl Layout for Stage {
    fn work_area(&self, area: Rect) -> Rect {
        if !self.column {
            return area;
        }
        Rect::new(
            (area.loc.x + self.reserve, area.loc.y).into(),
            ((area.size.w - self.reserve).max(1), area.size.h).into(),
        )
    }

    fn parked(&self, id: u64) -> bool {
        match self.sets.first() {
            Some(front) if self.one_at_a_time => {
                front.windows.first() != Some(&id) && self.set_of(id).is_some()
            }
            Some(front) => !front.windows.contains(&id) && self.set_of(id).is_some(),
            None => false,
        }
    }

    fn set_of(&self, id: u64) -> Option<u64> {
        self.sets
            .iter()
            .find(|s| s.windows.contains(&id))
            .map(|s| s.id)
    }
}

use std::time::{Duration, Instant};

use isle_shell::stage::{Entry, Frame, StageAction};
use smithay::{output::Output, utils::Point};
use tracing::info;

use super::{desktops::Kind, layout::Arrangement};
use crate::state::{Isle, State};

/// The column's width with `space-l` either side, kept clear on the stage while it shows.
const RESERVE: i32 = 192;

/// The pointer's push into the leading edge that reveals a hidden column.
#[derive(Default)]
pub struct Reveal {
    /// When the push began, by output.
    pushing: Option<(String, Instant)>,
    /// Outputs whose hidden column is shown by a push.
    pub shown: std::collections::HashSet<String>,
}

impl Isle {
    /// The Stage Manager arranging the desktop `output` shows.
    fn stage_in_view(&self, output: &Output) -> Option<(u64, &Stage)> {
        let row = self.desktops.rows.get(&Self::output_identity(output))?;
        let desktop = row.desktops.get(row.current)?;
        match &desktop.layout {
            Arrangement::Stage(stage) => Some((desktop.id, stage)),
            Arrangement::Stacking => None,
        }
    }

    fn stage_mut(&mut self, desk: u64) -> Option<&mut Stage> {
        let (identity, index) = self.desktops.locate(desk).map(|(i, n)| (i.to_owned(), n))?;
        match &mut self.desktops.rows.get_mut(&identity)?.desktops[index].layout {
            Arrangement::Stage(stage) => Some(stage),
            Arrangement::Stacking => None,
        }
    }

    /// The set window `id` is in, on its desktop's Stage Manager.
    pub fn stage_set_of(&self, id: u64) -> Option<u64> {
        let desk = self.wm.get(id)?.desktop;
        let (identity, index) = self.desktops.locate(desk)?;
        self.desktops.rows[identity].desktops[index]
            .layout
            .layout()
            .set_of(id)
    }
}

impl State {
    /// Stage Manager on or off for desktop `index` of `output`.
    pub fn set_stage(&mut self, output: &Output, index: usize, on: bool) -> bool {
        let identity = Isle::output_identity(output);
        let Some(desktop) = self
            .isle
            .desktops
            .ensure_row(&identity)
            .desktops
            .get(index)
            .cloned()
        else {
            return false;
        };
        if desktop.kind != Kind::Normal || desktop.layout.is_stage() == on {
            return false;
        }
        let layout = if on {
            // Each app's windows a set, the most recently used app's in front.
            let mut sets: Vec<Set> = Vec::new();
            let on_desk: Vec<u64> =
                self.isle
                    .wm
                    .recent
                    .iter()
                    .copied()
                    .chain(self.isle.wm.records.iter().rev().map(|r| r.id))
                    .filter(|id| {
                        self.isle.wm.get(*id).is_some_and(|r| {
                            r.desktop == desktop.id && r.parent.is_none() && r.mapped
                        })
                    })
                    .collect();
            for id in on_desk {
                let app = self
                    .isle
                    .owner_app(self.isle.wm.get(id).unwrap())
                    .to_owned();
                match sets.iter_mut().find(|s| {
                    s.windows
                        .first()
                        .and_then(|w| self.isle.wm.get(*w))
                        .is_some_and(|r| self.isle.owner_app(r) == app)
                }) {
                    Some(set) if !set.windows.contains(&id) => set.windows.push(id),
                    Some(_) => {}
                    None => sets.push(Set {
                        id,
                        windows: vec![id],
                        placed: false,
                    }),
                }
            }
            Arrangement::Stage(Stage {
                sets,
                ..Default::default()
            })
        } else {
            Arrangement::Stacking
        };
        let row = self.isle.desktops.rows.get_mut(&identity).unwrap();
        row.desktops[index].layout = layout;
        info!(output = output.name(), index, on, "stage manager");
        self.isle.desktops.save();
        for record in &mut self.isle.wm.records {
            if record.desktop == desktop.id {
                record.staged_out = false;
            }
        }
        self.apply_stage(output);
        self.isle.notify(crate::worker::Event::DesktopsChanged);
        true
    }

    pub fn apply_stage_all(&mut self) {
        let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
        for output in outputs {
            self.apply_stage(&output);
        }
    }

    /// Parks every set but the front one, centres a set staged for the first time, and
    /// shows the column when it is due.
    pub fn apply_stage(&mut self, output: &Output) {
        let settings = &self.isle.settings.stage_manager;
        let (recent, one) = (
            settings.recent_apps,
            settings.windows == isle_settings::schema::StageWindows::OneAtATime,
        );
        let revealed = self.isle.stage_reveal.shown.contains(&output.name());
        let Some((desk, _)) = self.isle.stage_in_view(output) else {
            if self.isle.shell.is_open_on("stage", output) {
                self.isle.shell.set_open_on("stage", output, false);
                self.dispatch_shell();
            }
            return;
        };
        let stage = self.isle.stage_mut(desk).unwrap();
        stage.one_at_a_time = one;
        stage.column = recent;
        stage.reserve = RESERVE;
        stage.sets.retain(|s| !s.windows.is_empty());
        let stage = stage.clone();
        for record in &mut self.isle.wm.records {
            if record.desktop == desk {
                record.staged_out = stage.parked(record.id);
            }
        }
        // A set staged for the first time is centred; after that its place is the user's.
        if let Some(front) = stage.sets.first()
            && !front.placed
        {
            let area = self.isle.work_area(output);
            for &id in &front.windows {
                if let Some(frame) = self.isle.frame(id) {
                    let loc: Point<i32, smithay::utils::Logical> = Point::from((
                        area.loc.x + (area.size.w - frame.size.w).max(0) / 2,
                        area.loc.y + (area.size.h - frame.size.h).max(0) / 2,
                    ));
                    let bar = self.isle.bar_height(id);
                    self.isle.move_window(id, Point::from((loc.x, loc.y + bar)));
                }
            }
            if let Some(s) = self.isle.stage_mut(desk) {
                s.sets[0].placed = true;
            }
        }
        self.isle.sync_space();
        self.push_stage_frame(output, &stage);
        let show = (recent || revealed) && stage.sets.len() > 1;
        if self.isle.shell.is_open_on("stage", output) != show {
            self.isle.shell.set_open_on("stage", output, show);
        }
        self.dispatch_shell();
        self.isle.queue_redraw(output);
    }

    fn push_stage_frame(&mut self, output: &Output, stage: &Stage) {
        let scale = output.current_scale().fractional_scale();
        let (w, h) = isle_shell::stage::ENTRY;
        let mut entries = Vec::new();
        for set in stage.sets.iter().skip(1) {
            let images = set
                .windows
                .iter()
                .take(3)
                .filter_map(|id| self.render_preview(*id, w as f64, h as f64, scale))
                .map(isle_shell::switcher::surface::PreviewImage)
                .collect();
            let app = set
                .windows
                .first()
                .and_then(|id| self.isle.wm.get(*id))
                .map(|r| self.isle.owner_app(r).to_owned())
                .unwrap_or_default();
            entries.push(Entry {
                set: set.id,
                name: self.isle.app_name(&app),
                icon: self
                    .isle
                    .app_icon(&app, 64)
                    .map(|p| p.to_string_lossy().into_owned()),
                images,
            });
        }
        let frame = Frame {
            entries,
            hover: None,
        };
        let surface = self.isle.stage_surface.clone();
        surface.set_model(&output.name(), frame.clone());
        self.isle.shell.with_roots("stage", |o| {
            if o.name() == output.name() {
                surface.set(&o.name(), frame.clone());
            }
        });
    }

    /// A window mapped on a Stage Manager desktop joins its app's set, which comes forward.
    pub fn stage_mapped(&mut self, id: u64) {
        let Some(record) = self.isle.wm.get(id) else {
            return;
        };
        if record.parent.is_some() {
            return;
        }
        let (desk, app) = (record.desktop, self.isle.owner_app(record).to_owned());
        let owner = |isle: &Isle, w: u64| isle.wm.get(w).map(|r| isle.owner_app(r).to_owned());
        let Some(stage) = self.isle.stage_mut(desk).cloned() else {
            return;
        };
        let at = stage.sets.iter().position(|s| {
            s.windows
                .first()
                .and_then(|w| owner(&self.isle, *w))
                .as_deref()
                == Some(app.as_str())
        });
        let stage = self.isle.stage_mut(desk).unwrap();
        let mut set = match at {
            Some(i) => stage.sets.remove(i),
            None => Set {
                id,
                windows: Vec::new(),
                placed: false,
            },
        };
        set.windows.retain(|w| *w != id);
        set.windows.insert(0, id);
        stage.sets.insert(0, set);
        if let Some(output) = self.isle.output_of_desktop(desk) {
            self.apply_stage(&output);
        }
    }

    /// Activating a window on a Stage Manager desktop swaps its set onto the stage.
    pub fn stage_activated(&mut self, id: u64) {
        let Some(desk) = self.isle.wm.get(id).map(|r| r.desktop) else {
            return;
        };
        let Some(stage) = self.isle.stage_mut(desk) else {
            return;
        };
        let Some(i) = stage.sets.iter().position(|s| s.windows.contains(&id)) else {
            return;
        };
        let mut set = stage.sets.remove(i);
        set.windows.retain(|w| *w != id);
        set.windows.insert(0, id);
        stage.sets.insert(0, set);
        if let Some(output) = self.isle.output_of_desktop(desk) {
            self.apply_stage(&output);
        }
    }

    /// A window gone leaves its set.
    pub fn stage_window_gone(&mut self, id: u64) {
        let mut touched = Vec::new();
        for row in self.isle.desktops.rows.values_mut() {
            for desktop in &mut row.desktops {
                if let Arrangement::Stage(stage) = &mut desktop.layout
                    && stage.sets.iter().any(|s| s.windows.contains(&id))
                {
                    for set in &mut stage.sets {
                        set.windows.retain(|w| *w != id);
                    }
                    stage.sets.retain(|s| !s.windows.is_empty());
                    touched.push(desktop.id);
                }
            }
        }
        for desk in touched {
            if let Some(output) = self.isle.output_of_desktop(desk) {
                self.apply_stage(&output);
            }
        }
    }

    pub fn stage_action(&mut self, action: StageAction) {
        match action {
            StageAction::Hover(_) => {}
            StageAction::Choose(set) => {
                let front = self
                    .isle
                    .desktops
                    .rows
                    .values()
                    .flat_map(|r| &r.desktops)
                    .find_map(|d| match &d.layout {
                        Arrangement::Stage(stage) => stage
                            .sets
                            .iter()
                            .find(|s| s.id == set)
                            .and_then(|s| s.windows.first().copied()),
                        Arrangement::Stacking => None,
                    });
                if let Some(id) = front {
                    self.activate(id);
                }
            }
        }
    }

    /// With recent apps hidden, a push into the leading edge held for the hover dwell shows
    /// the column; leaving it hides it again.
    pub fn stage_edge_motion(&mut self) {
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        let Some(output) = self.isle.space.output_under(at).next().cloned() else {
            return;
        };
        if self.isle.settings.stage_manager.recent_apps
            || self.isle.stage_in_view(&output).is_none()
        {
            return;
        }
        let Some(screen) = self.isle.space.output_geometry(&output) else {
            return;
        };
        let x = at.x - screen.loc.x as f64;
        let name = output.name();
        let shown = self.isle.stage_reveal.shown.contains(&name);
        if shown && x > RESERVE as f64 {
            self.isle.stage_reveal.shown.remove(&name);
            self.apply_stage(&output);
            return;
        }
        if x > 1.0 {
            self.isle.stage_reveal.pushing = None;
            return;
        }
        let dwell =
            Duration::from_millis(self.isle.shell.theme().tokens.timing_hover_dwell_ms as u64);
        match &self.isle.stage_reveal.pushing {
            Some((o, since)) if *o == name && since.elapsed() >= dwell && !shown => {
                self.isle.stage_reveal.shown.insert(name);
                self.apply_stage(&output);
            }
            Some((o, _)) if *o == name => {}
            _ => {
                self.isle.stage_reveal.pushing = Some((name, Instant::now()));
                let token = self.isle.loop_handle.insert_source(
                    smithay::reexports::calloop::timer::Timer::from_duration(dwell),
                    |_, _, state| {
                        state.stage_edge_motion();
                        smithay::reexports::calloop::timer::TimeoutAction::Drop
                    },
                );
                let _ = token;
            }
        }
    }
}
