//! Each output's row of desktops, and full-screen desktops (`compositor.md` Spaces 1, 2, 5–8).

use std::collections::BTreeMap;

use isle_state::{Loaded, StateDir, domains::Desktops as SavedDesktops};
use smithay::{
    output::Output, reexports::wayland_protocols::xdg::shell::server::xdg_toplevel, utils::Point,
};
use tracing::{info, warn};

use super::Mode;
use crate::{
    state::{Isle, State},
    state_writer::Saver,
    worker::Event,
};

/// The most desktops an output's row holds.
pub const MAX_DESKTOPS: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Normal,
    /// A window's own, right of the desktop it came from.
    FullScreen {
        window: u64,
    },
}

#[derive(Clone, Debug)]
pub struct Desktop {
    pub id: u64,
    pub kind: Kind,
}

/// One output's desktops, by the output's identity.
#[derive(Clone, Debug)]
pub struct Row {
    pub desktops: Vec<Desktop>,
    pub current: usize,
}

#[derive(Default)]
pub struct Desktops {
    pub rows: BTreeMap<String, Row>,
    next_id: u64,
    pub saved: SavedDesktops,
    pub saver: Option<Saver<SavedDesktops>>,
}

impl Desktops {
    fn new_desktop(&mut self, kind: Kind) -> Desktop {
        self.next_id += 1;
        Desktop {
            id: self.next_id,
            kind,
        }
    }

    /// The row for `identity`, made with the count it had last time.
    pub fn ensure_row(&mut self, identity: &str) -> &mut Row {
        if !self.rows.contains_key(identity) {
            let count = self
                .saved
                .counts
                .get(identity)
                .copied()
                .unwrap_or(1)
                .clamp(1, MAX_DESKTOPS as u32);
            let desktops = (0..count).map(|_| self.new_desktop(Kind::Normal)).collect();
            self.rows.insert(
                identity.to_owned(),
                Row {
                    desktops,
                    current: 0,
                },
            );
        }
        self.rows.get_mut(identity).unwrap()
    }

    /// The row holding `desktop`, and the desktop's index in it.
    pub fn locate(&self, desktop: u64) -> Option<(&str, usize)> {
        self.rows.iter().find_map(|(identity, row)| {
            row.desktops
                .iter()
                .position(|d| d.id == desktop)
                .map(|i| (identity.as_str(), i))
        })
    }

    pub fn is_current(&self, desktop: u64) -> bool {
        self.locate(desktop)
            .is_some_and(|(identity, i)| self.rows[identity].current == i)
    }

    pub fn current_of(&self, identity: &str) -> Option<u64> {
        let row = self.rows.get(identity)?;
        row.desktops.get(row.current).map(|d| d.id)
    }

    fn save(&mut self) {
        for (identity, row) in &self.rows {
            let normal = row
                .desktops
                .iter()
                .filter(|d| d.kind == Kind::Normal)
                .count() as u32;
            self.saved.counts.insert(identity.clone(), normal);
        }
        if let Some(saver) = &self.saver {
            saver.save(self.saved.clone());
        }
    }
}

impl Isle {
    pub fn load_desktops(&mut self, runtime: &tokio::runtime::Handle) {
        let dir = StateDir::new(StateDir::default_dir());
        let (saved, loaded) = dir.load::<SavedDesktops>();
        if !matches!(loaded, Loaded::Read | Loaded::Missing) {
            warn!(?loaded, "desktops");
        }
        self.desktops.saved = saved;
        self.desktops.saver = Some(Saver::spawn(runtime, dir));
    }

    /// The output showing the row of `desktop`.
    pub fn output_of_desktop(&self, desktop: u64) -> Option<Output> {
        let (identity, _) = self.desktops.locate(desktop)?;
        self.space
            .outputs()
            .find(|o| Self::output_identity(o) == identity)
            .cloned()
    }

    /// The window of `output`'s full-screen desktop while it is in view and has the
    /// keyboard: what VRR follows (B30).
    pub fn full_screen_front(&self, output: &Output) -> Option<u64> {
        let row = self.desktops.rows.get(&Self::output_identity(output))?;
        let Kind::FullScreen { window } = row.desktops.get(row.current)?.kind else {
            return None;
        };
        let focus = self.seat.get_keyboard()?.current_focus()?;
        let focused = self.wm.for_surface(focus.surface()?)?.id;
        (focused == window).then_some(window)
    }

    /// The full-screen window `output` shows, focused or not.
    pub fn full_screen_shown(&self, output: &Output) -> Option<u64> {
        let row = self.desktops.rows.get(&Self::output_identity(output))?;
        match row.desktops.get(row.current)?.kind {
            Kind::FullScreen { window } => Some(window),
            _ => None,
        }
    }

    /// The current desktop of `output`.
    pub fn current_desktop(&mut self, output: &Output) -> u64 {
        let identity = Self::output_identity(output);
        let row = self.desktops.ensure_row(&identity);
        row.desktops[row.current].id
    }

    /// Puts `id` on the current desktop of the output it is now on, if its desktop is on
    /// another output.
    pub fn follow_output(&mut self, id: u64) {
        let Some(output) = self.output_of(id) else {
            return;
        };
        let identity = Self::output_identity(&output);
        let Some(record) = self.wm.get(id) else {
            return;
        };
        if matches!(record.mode, Mode::FullScreen { .. }) {
            return;
        }
        let here = self
            .desktops
            .locate(record.desktop)
            .is_some_and(|(row, _)| row == identity);
        if !here {
            let desktop = self.current_desktop(&output);
            self.wm.get_mut(id).unwrap().desktop = desktop;
        }
    }

    /// The desktop index (from 1) of `id` on its output's row.
    pub fn desktop_index(&self, id: u64) -> u32 {
        self.wm
            .get(id)
            .and_then(|r| self.desktops.locate(r.desktop))
            .map_or(0, |(_, i)| i as u32 + 1)
    }

    pub fn desktop_infos(&self) -> Vec<isle_ipc::DesktopInfo> {
        let mut out = Vec::new();
        for output in self.space.outputs() {
            let identity = Self::output_identity(output);
            let Some(row) = self.desktops.rows.get(&identity) else {
                continue;
            };
            for (i, desktop) in row.desktops.iter().enumerate() {
                let (kind, window) = match desktop.kind {
                    Kind::Normal => ("desktop", 0),
                    Kind::FullScreen { window } => ("full-screen", window),
                };
                out.push(isle_ipc::DesktopInfo {
                    output: output.name(),
                    index: i as u32 + 1,
                    current: row.current == i,
                    kind: kind.into(),
                    window,
                    windows: self
                        .wm
                        .records
                        .iter()
                        .filter(|r| r.mapped && r.desktop == desktop.id)
                        .count() as u32,
                });
            }
        }
        out
    }
}

impl State {
    /// The output Ctrl+arrows act on: the focused window's, else the pointer's.
    fn focus_output(&self) -> Option<Output> {
        self.isle.output_with_focus()
    }

    /// Shows desktop `index` (from 0) of `output`; false when there is none.
    pub fn switch_desktop(&mut self, output: &Output, index: usize) -> bool {
        let identity = Isle::output_identity(output);
        let row = self.isle.desktops.ensure_row(&identity);
        if index >= row.desktops.len() {
            return false;
        }
        if row.current == index {
            return true;
        }
        let step = if index > row.current { 1 } else { -1 };
        self.slide_out(output, step, None);
        let row = self.isle.desktops.ensure_row(&identity);
        row.current = index;
        self.after_switch(output);
        self.slide_in(output, step, None);
        true
    }

    pub(crate) fn after_switch(&mut self, output: &Output) {
        self.isle.sync_space();
        let on_output: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| {
                self.isle.wm.is_shown(r.id) && self.isle.output_of(r.id).as_ref() == Some(output)
            })
            .map(|r| r.id)
            .collect();
        let next = self
            .isle
            .wm
            .recent
            .iter()
            .copied()
            .find(|id| on_output.contains(id))
            .or_else(|| on_output.last().copied());
        match next {
            Some(id) => {
                let window = self.isle.window_by_id(id);
                self.set_focus(window.as_ref());
            }
            None => {
                if self
                    .focused_window()
                    .is_some_and(|f| !self.isle.wm.is_shown(f))
                {
                    self.focus_next();
                }
            }
        }
        self.isle.queue_redraw(output);
        self.isle.notify(Event::DesktopsChanged);
        self.isle.notify(Event::WindowsChanged);
    }

    /// Ctrl+← and Ctrl+→: along the row, without wrapping.
    pub fn step_desktop(&mut self, step: i32) {
        let Some(output) = self.focus_output() else {
            return;
        };
        let identity = Isle::output_identity(&output);
        let row = self.isle.desktops.ensure_row(&identity);
        let next = row.current as i32 + step;
        if next >= 0 && (next as usize) < row.desktops.len() {
            self.switch_desktop(&output, next as usize);
        }
    }

    /// Ctrl+1…9: a number past the end adds desktops up to it.
    pub fn jump_desktop(&mut self, index: usize) {
        let Some(output) = self.focus_output() else {
            return;
        };
        let identity = Isle::output_identity(&output);
        while self.isle.desktops.ensure_row(&identity).desktops.len() <= index {
            if self.add_desktop(&output).is_none() {
                return;
            }
        }
        self.switch_desktop(&output, index);
    }

    /// Adds a desktop at the end of `output`'s row; its index, or `None` at the limit.
    pub fn add_desktop(&mut self, output: &Output) -> Option<usize> {
        let identity = Isle::output_identity(output);
        if self.isle.desktops.ensure_row(&identity).desktops.len() >= MAX_DESKTOPS {
            return None;
        }
        let desktop = self.isle.desktops.new_desktop(Kind::Normal);
        let row = self.isle.desktops.rows.get_mut(&identity).unwrap();
        row.desktops.push(desktop);
        let index = row.desktops.len() - 1;
        self.isle.desktops.save();
        self.isle.notify(Event::DesktopsChanged);
        Some(index)
    }

    /// Removes desktop `index` of `output`; its windows join the desktop before it, or after
    /// it for the first. The last normal desktop and full-screen ones are not removed here.
    pub fn remove_desktop(&mut self, output: &Output, index: usize) -> bool {
        let identity = Isle::output_identity(output);
        let row = self.isle.desktops.ensure_row(&identity);
        let normal = row
            .desktops
            .iter()
            .filter(|d| d.kind == Kind::Normal)
            .count();
        let Some(desktop) = row.desktops.get(index) else {
            return false;
        };
        if desktop.kind != Kind::Normal || normal < 2 {
            return false;
        }
        let gone = row.desktops.remove(index).id;
        let into = row.desktops[index.saturating_sub(1).min(row.desktops.len() - 1)].id;
        if row.current >= index && row.current > 0 {
            row.current -= 1;
        }
        for record in &mut self.isle.wm.records {
            if record.desktop == gone {
                record.desktop = into;
            }
        }
        self.isle.desktops.save();
        self.after_switch(output);
        true
    }

    /// Moves `id` to desktop `index` of its output's row and shows it there.
    pub fn move_to_desktop(&mut self, id: u64, index: usize) -> bool {
        let Some(output) = self.isle.output_of(id) else {
            return false;
        };
        if matches!(
            self.isle.wm.get(id).map(|r| &r.mode),
            Some(Mode::FullScreen { .. }) | None
        ) {
            return false;
        }
        let identity = Isle::output_identity(&output);
        let row = self.isle.desktops.ensure_row(&identity);
        let Some(target) = row.desktops.get(index).filter(|d| d.kind == Kind::Normal) else {
            return false;
        };
        let target = target.id;
        self.isle.wm.get_mut(id).unwrap().desktop = target;
        for child in self.dialogs_of(id) {
            self.isle.wm.get_mut(child).unwrap().desktop = target;
        }
        self.switch_desktop(&output, index);
        self.activate(id);
        true
    }

    /// Moves `id` to desktop `index` of `output`'s row, leaving the view where it is; on
    /// another output it keeps its place from the screen's top left, kept on screen.
    pub fn place_on_desktop(&mut self, id: u64, output: &Output, index: usize) -> bool {
        let Some(record) = self.isle.wm.get(id) else {
            return false;
        };
        if matches!(record.mode, Mode::FullScreen { .. }) || record.parent.is_some() {
            return false;
        }
        let identity = Isle::output_identity(output);
        let row = self.isle.desktops.ensure_row(&identity);
        let Some(target) = row.desktops.get(index).filter(|d| d.kind == Kind::Normal) else {
            return false;
        };
        let target = target.id;
        let from = self.isle.output_of(id);
        if from.as_ref() != Some(output)
            && let (Some(from), Some(loc)) = (from, self.isle.wm.get(id).map(|r| {
                self.isle.space.element_location(&r.window).unwrap_or(r.rect.loc)
            }))
            && let (Some(was), Some(now)) = (
                self.isle.space.output_geometry(&from),
                self.isle.space.output_geometry(output),
            )
        {
            let size = self.isle.frame(id).map_or((0, 0).into(), |f| f.size);
            let rel = loc - was.loc;
            let x = rel.x.clamp(0, (now.size.w - size.w).max(0));
            let y = rel.y.clamp(0, (now.size.h - size.h).max(0));
            self.isle.move_window(id, now.loc + Point::from((x, y)));
        }
        self.isle.wm.get_mut(id).unwrap().desktop = target;
        for child in self.dialogs_of(id) {
            self.isle.wm.get_mut(child).unwrap().desktop = target;
        }
        info!(id, index, "placed on a desktop");
        self.after_switch(output);
        true
    }

    /// Moves desktop `from` of `output`'s row to `to`, the view staying on the desktop it
    /// shows.
    pub fn reorder_desktop(&mut self, output: &Output, from: usize, to: usize) -> bool {
        let identity = Isle::output_identity(output);
        let row = self.isle.desktops.ensure_row(&identity);
        if from >= row.desktops.len() || to >= row.desktops.len() || from == to {
            return false;
        }
        let shown = row.desktops[row.current].id;
        let desktop = row.desktops.remove(from);
        row.desktops.insert(to, desktop);
        row.current = row.desktops.iter().position(|d| d.id == shown).unwrap_or(0);
        self.isle.desktops.save();
        self.isle.notify(Event::DesktopsChanged);
        true
    }

    /// Ctrl+Shift+Cmd+← and →: the focused window one desktop along, the view with it.
    pub fn carry_window(&mut self, step: i32) {
        let Some(id) = self.focused_window() else {
            return;
        };
        let Some(record) = self.isle.wm.get(id) else {
            return;
        };
        let Some((identity, index)) = self.isle.desktops.locate(record.desktop) else {
            return;
        };
        let len = self.isle.desktops.rows[identity].desktops.len() as i32;
        let next = index as i32 + step;
        if next >= 0 && next < len {
            self.move_to_desktop(id, next as usize);
        }
    }

    fn dialogs_of(&self, id: u64) -> Vec<u64> {
        let mut out = Vec::new();
        let mut take = vec![id];
        while let Some(next) = take.pop() {
            for record in self
                .isle
                .wm
                .records
                .iter()
                .filter(|r| r.parent == Some(next))
            {
                out.push(record.id);
                take.push(record.id);
            }
        }
        out
    }

    /// Gives `id` a full-screen desktop right of the one it is on. `borderless` is a window
    /// that covers its output by itself, which is not told it is full screen (D48).
    pub fn enter_full_screen(&mut self, id: u64, borderless: bool) -> bool {
        let Some(output) = self.isle.output_of(id) else {
            return false;
        };
        let Some(frame) = self.isle.frame(id) else {
            return false;
        };
        let Some(record) = self.isle.wm.get(id) else {
            return false;
        };
        if matches!(record.mode, Mode::FullScreen { .. }) || record.parent.is_some() {
            return false;
        }
        let identity = Isle::output_identity(&output);
        let (desktop_was, previous, mapped) =
            (record.desktop, Box::new(record.mode.clone()), record.mapped);
        let origin = match self.isle.desktops.locate(desktop_was) {
            Some((row, _)) if row == identity => desktop_was,
            _ => self.isle.current_desktop(&output),
        };
        // An X11 window may have no surface yet to hold the keyboard; the window last given
        // focus is it then.
        let focused = self.focused_window() == Some(id)
            || !mapped
            || self.isle.wm.recent.first() == Some(&id);
        if focused && origin == self.isle.current_desktop(&output) {
            self.slide_out(&output, 1, Some(id));
        }
        let desktop = self
            .isle
            .desktops
            .new_desktop(Kind::FullScreen { window: id });
        let desktop_id = desktop.id;
        let row = self.isle.desktops.ensure_row(&identity);
        let at = row
            .desktops
            .iter()
            .position(|d| d.id == origin)
            .map_or(row.desktops.len(), |i| i + 1);
        row.desktops.insert(at, desktop);
        if row.current >= at {
            row.current += 1;
        }
        let record = self.isle.wm.get_mut(id).unwrap();
        record.mode = Mode::FullScreen {
            origin,
            restore: frame,
            previous,
            borderless,
        };
        record.desktop = desktop_id;
        for child in self.dialogs_of(id) {
            self.isle.wm.get_mut(child).unwrap().desktop = desktop_id;
        }
        info!(id, borderless, "full screen");
        // The view moves first, so the window is shown on its new desktop as it grows.
        if focused {
            self.isle.desktops.rows.get_mut(&identity).unwrap().current = at;
        }
        self.isle.sync_space();
        if !borderless {
            self.send_full_screen(id, true);
            let area = self.isle.space.output_geometry(&output).unwrap_or_default();
            if mapped {
                self.begin_morph(id, area);
            }
            if let Some(record) = self.isle.wm.get(id) {
                record.configure_size(area.size);
            }
            self.isle.move_window(id, area.loc);
        }
        self.after_switch(&output);
        if focused {
            let window = self.isle.window_by_id(id);
            self.set_focus(window.as_ref());
        }
        true
    }

    /// Ends `id`'s full screen: its desktop goes, and it and the view return to its origin.
    pub fn leave_full_screen(&mut self, id: u64) -> bool {
        let Some(record) = self.isle.wm.get(id) else {
            return false;
        };
        if matches!(record.mode, Mode::FullScreen { .. }) {
            info!(id, "full screen left");
        }
        let Mode::FullScreen {
            origin,
            restore,
            previous,
            borderless,
        } = record.mode.clone()
        else {
            return false;
        };
        let desktop = record.desktop;
        let viewing = self.isle.desktops.is_current(desktop);
        let output = self.isle.output_of_desktop(desktop);
        let target = self.remove_full_screen_desktop(desktop, origin);
        let record = self.isle.wm.get_mut(id).unwrap();
        record.mode = *previous;
        if let Some(target) = target {
            record.desktop = target;
        }
        for child in self.dialogs_of(id) {
            if let Some(target) = target {
                self.isle.wm.get_mut(child).unwrap().desktop = target;
            }
        }
        // The view moves back first, so the window is shown on its origin as it shrinks.
        if viewing
            && let Some((identity, index)) = target.and_then(|t| self.isle.desktops.locate(t))
        {
            let identity = identity.to_owned();
            self.isle.desktops.rows.get_mut(&identity).unwrap().current = index;
        }
        self.isle.sync_space();
        if !borderless {
            self.send_full_screen(id, false);
            self.send_mode_states(id);
            self.set_frame(id, restore);
        }
        if let Some(output) = output {
            self.after_switch(&output);
            if viewing {
                self.slide_in(&output, -1, Some(id));
            }
        }
        true
    }

    /// Removes a full-screen desktop; returns the desktop its window goes back to.
    fn remove_full_screen_desktop(&mut self, desktop: u64, origin: u64) -> Option<u64> {
        let (identity, index) = self.isle.desktops.locate(desktop)?;
        let identity = identity.to_owned();
        let row = self.isle.desktops.rows.get_mut(&identity).unwrap();
        row.desktops.remove(index);
        if row.current > index || row.current >= row.desktops.len() {
            row.current = row.current.saturating_sub(1);
        }
        let target = if row.desktops.iter().any(|d| d.id == origin) {
            origin
        } else {
            row.desktops[index.saturating_sub(1).min(row.desktops.len() - 1)].id
        };
        Some(target)
    }

    /// A full-screen window that closes takes its desktop with it; the view goes back.
    pub fn full_screen_window_gone(&mut self, desktop: u64, origin: u64) {
        let viewing = self.isle.desktops.is_current(desktop);
        let output = self.isle.output_of_desktop(desktop);
        let Some(target) = self.remove_full_screen_desktop(desktop, origin) else {
            return;
        };
        if viewing && let Some((identity, index)) = self.isle.desktops.locate(target) {
            let identity = identity.to_owned();
            self.isle.desktops.rows.get_mut(&identity).unwrap().current = index;
        }
        if let Some(output) = output {
            self.after_switch(&output);
            if viewing {
                self.slide_in(&output, -1, None);
            }
        }
    }

    pub fn toggle_full_screen(&mut self, id: u64) {
        if matches!(
            self.isle.wm.get(id).map(|r| &r.mode),
            Some(Mode::FullScreen { .. })
        ) {
            self.leave_full_screen(id);
        } else {
            self.enter_full_screen(id, false);
        }
    }

    fn send_full_screen(&self, id: u64, on: bool) {
        if let Some(x11) = self.isle.wm.get(id).and_then(|r| r.x11()) {
            let _ = x11.set_fullscreen(on);
            return;
        }
        let Some(toplevel) = self.isle.wm.get(id).and_then(|r| r.window.toplevel()) else {
            return;
        };
        toplevel.with_pending_state(|state| {
            if on {
                state.states.set(xdg_toplevel::State::Fullscreen);
            } else {
                state.states.unset(xdg_toplevel::State::Fullscreen);
            }
        });
        if toplevel.is_initial_configure_sent() {
            toplevel.send_pending_configure();
        }
    }

    /// After a commit: a window that now covers its output exactly by itself gets a
    /// full-screen desktop, and a borderless one that no longer does leaves it.
    pub fn check_borderless(&mut self, id: u64) {
        let Some(record) = self.isle.wm.get(id) else {
            return;
        };
        if !record.mapped || record.parent.is_some() || self.isle.bar_height(id) > 0 {
            return;
        }
        let Some(frame) = self.isle.frame(id) else {
            return;
        };
        let covers = self
            .isle
            .space
            .outputs()
            .filter_map(|o| self.isle.space.output_geometry(o))
            .any(|g| g == frame);
        match record.mode {
            Mode::FullScreen {
                borderless: true, ..
            } if !covers => {
                self.leave_full_screen(id);
            }
            Mode::Floating if covers => {
                self.enter_full_screen(id, true);
            }
            _ => {}
        }
    }
}
