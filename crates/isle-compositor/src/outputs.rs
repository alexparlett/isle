//! Windows and the outputs they are on: moved off an output that goes, back when it
//! returns, and never left where no output shows them (`compositor.md`, Outputs 4).

use smithay::{
    output::Output,
    utils::{Logical, Point, Rectangle},
};
use tracing::info;

use crate::{state::Isle, worker::Event};

impl Isle {
    pub fn add_output(&mut self, output: &Output, location: Point<i32, Logical>) {
        output
            .user_data()
            .insert_if_missing(crate::redraw::OutputState::cell);
        self.space.map_output(output, location);
        self.desktops.ensure_row(&Self::output_identity(output));
        self.apply_output_settings(output);
        self.return_windows_home(output);
        self.adopt_strays(output);
        self.rescue_windows();
        self.output_set_changed();
        self.idle.output_events.push((output.clone(), true));
    }

    /// Every window on the output's desktops moves whole onto the nearest output, joining
    /// its current desktop, and remembers where it was to go back.
    pub fn remove_output(&mut self, output: &Output) {
        let Some(area) = self.space.output_geometry(output) else {
            return;
        };
        self.end_casts_of_output(output);
        // A screen going ends the pick: its frozen frame has nowhere to be.
        if self.freeze.take().is_some() {
            self.shell.close("capture-readout");
            self.cursor_status = smithay::input::pointer::CursorImageStatus::default_named();
        }
        let identity = Self::output_identity(output);
        self.wallpapers.forget(&identity);
        self.idle.output_events.push((output.clone(), false));
        let leaving: Vec<(u64, Rectangle<i32, Logical>)> = self
            .wm
            .records
            .iter()
            .filter(|r| r.mapped)
            .filter(|r| match self.desktops.locate(r.desktop) {
                Some((row, _)) => row == identity,
                None => self.output_of(r.id).as_ref() == Some(output),
            })
            .filter_map(|r| Some((r.id, self.frame(r.id)?)))
            .collect();
        self.space.unmap_output(output);
        if let Some(row) = self.desktops.rows.get_mut(&identity) {
            row.desktops
                .retain(|d| d.kind == crate::wm::desktops::Kind::Normal);
            row.current = row.current.min(row.desktops.len().saturating_sub(1));
        }
        for (id, frame) in leaving {
            let mut frame = frame;
            let record = self.wm.get_mut(id).unwrap();
            let mut desktop = record.desktop;
            if let crate::wm::Mode::FullScreen {
                origin,
                restore,
                previous,
                ..
            } = record.mode.clone()
            {
                record.mode = *previous;
                desktop = origin;
                frame = restore;
                record.configure_size(restore.size);
                if let Some(toplevel) = record.window.toplevel() {
                    toplevel.with_pending_state(|state| {
                        state.states.unset(
                            smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Fullscreen,
                        );
                    });
                    toplevel.send_pending_configure();
                }
            }
            let offset = frame.loc - area.loc;
            self.wm.get_mut(id).unwrap().home = Some((identity.clone(), desktop, offset));
            if let Some(target) = self.nearest_output(frame) {
                let current = self.current_desktop(&target);
                self.wm.get_mut(id).unwrap().desktop = current;
                self.place_whole(id, frame, &target, offset);
            }
        }
        self.output_set_changed();
    }

    fn output_set_changed(&mut self) {
        self.update_frame_budget();
        self.sync_shell_outputs();
        self.sync_space();
        self.queue_redraw_all();
        self.notify(Event::DisplaysChanged);
        self.discover_brightness();
        self.notify(Event::DesktopsChanged);
        self.notify(Event::WindowsChanged);
    }

    /// Windows that left this output's identity go back to their place and desktop on it.
    fn return_windows_home(&mut self, output: &Output) {
        let identity = Self::output_identity(output);
        let returning: Vec<(u64, u64, Point<i32, Logical>)> = self
            .wm
            .records
            .iter()
            .filter_map(|r| match &r.home {
                Some((home, desktop, offset)) if *home == identity => {
                    Some((r.id, *desktop, *offset))
                }
                _ => None,
            })
            .collect();
        for (id, desktop, offset) in returning {
            let current = self.current_desktop(output);
            let exists = self
                .desktops
                .locate(desktop)
                .is_some_and(|(row, _)| row == identity);
            let record = self.wm.get_mut(id).unwrap();
            record.home = None;
            record.desktop = if exists { desktop } else { current };
            if let Some(frame) = self.frame(id) {
                self.place_whole(id, frame, output, offset);
                info!(output = output.name(), "a window went back to its output");
            }
        }
    }

    /// Windows whose output went while no other was lit come whole onto `output`, joining
    /// its current desktop.
    fn adopt_strays(&mut self, output: &Output) {
        let present: Vec<String> = self.space.outputs().map(Self::output_identity).collect();
        let strays: Vec<(u64, Point<i32, Logical>)> = self
            .wm
            .records
            .iter()
            .filter(|r| r.mapped)
            .filter(|r| {
                self.desktops
                    .locate(r.desktop)
                    .is_some_and(|(row, _)| !present.iter().any(|p| p == row))
            })
            .map(|r| (r.id, r.home.as_ref().map(|h| h.2).unwrap_or_default()))
            .collect();
        for (id, offset) in strays {
            let current = self.current_desktop(output);
            self.wm.get_mut(id).unwrap().desktop = current;
            if let Some(frame) = self.frame(id) {
                self.place_whole(id, frame, output, offset);
            }
        }
    }

    /// Moves any window that no output shows onto the one nearest it.
    pub fn rescue_windows(&mut self) {
        let ids: Vec<u64> = self
            .wm
            .records
            .iter()
            .filter(|r| r.mapped)
            .map(|r| r.id)
            .collect();
        for id in ids {
            let Some(frame) = self.frame(id) else {
                continue;
            };
            if self.space.outputs().any(|o| {
                self.space
                    .output_geometry(o)
                    .is_some_and(|g| g.overlaps(frame))
            }) {
                continue;
            }
            if let Some(target) = self.nearest_output(frame) {
                let area = self.space.output_geometry(&target).unwrap();
                self.place_whole(id, frame, &target, frame.loc - area.loc);
                self.follow_output(id);
            }
        }
    }

    /// `pos` if an output shows it, else the nearest point an output does.
    pub fn clamp_to_outputs(&self, pos: Point<f64, Logical>) -> Point<f64, Logical> {
        if self.space.output_under(pos).next().is_some() {
            return pos;
        }
        self.space
            .outputs()
            .filter_map(|o| self.space.output_geometry(o))
            .map(|area| {
                let area = area.to_f64();
                Point::from((
                    pos.x.clamp(area.loc.x, area.loc.x + area.size.w - 1.0),
                    pos.y.clamp(area.loc.y, area.loc.y + area.size.h - 1.0),
                ))
            })
            .min_by(|a, b| {
                let da = (a.x - pos.x).powi(2) + (a.y - pos.y).powi(2);
                let db = (b.x - pos.x).powi(2) + (b.y - pos.y).powi(2);
                da.total_cmp(&db)
            })
            .unwrap_or(pos)
    }

    fn nearest_output(&self, geometry: Rectangle<i32, Logical>) -> Option<Output> {
        let centre = geometry.loc + geometry.size.downscale(2).to_point();
        self.space
            .outputs()
            .filter_map(|o| Some((o.clone(), self.space.output_geometry(o)?)))
            .min_by_key(|(_, area)| {
                let clamped = Point::<i32, Logical>::from((
                    centre.x.clamp(area.loc.x, area.loc.x + area.size.w),
                    centre.y.clamp(area.loc.y, area.loc.y + area.size.h),
                ));
                let d = centre - clamped;
                i64::from(d.x).pow(2) + i64::from(d.y).pow(2)
            })
            .map(|(o, _)| o)
    }

    /// Puts the window at `offset` on `output`, moved so its frame is whole on it.
    fn place_whole(
        &mut self,
        id: u64,
        frame: Rectangle<i32, Logical>,
        output: &Output,
        offset: Point<i32, Logical>,
    ) {
        let Some(area) = self.space.output_geometry(output) else {
            return;
        };
        let size = frame.size;
        let x = (area.loc.x + offset.x).clamp(
            area.loc.x,
            (area.loc.x + area.size.w - size.w).max(area.loc.x),
        );
        let y = (area.loc.y + offset.y).clamp(
            area.loc.y,
            (area.loc.y + area.size.h - size.h).max(area.loc.y),
        );
        let bar = self.bar_height(id);
        self.move_window(id, Point::from((x, y + bar)));
    }
}
