//! Windows and the outputs they are on: moved off an output that goes, back when it
//! returns, and never left where no output shows them (`compositor.md`, Outputs 4).

use smithay::{
    desktop::Window,
    output::Output,
    utils::{Logical, Point, Rectangle},
};
use tracing::info;

use crate::{state::Isle, worker::Event};

/// Where a window was before its output went: the output's identity and the window's
/// place relative to that output.
struct Home {
    output: String,
    offset: Point<i32, Logical>,
}

impl Isle {
    pub fn add_output(&mut self, output: &Output, location: Point<i32, Logical>) {
        output
            .user_data()
            .insert_if_missing(crate::redraw::OutputState::cell);
        self.space.map_output(output, location);
        self.apply_output_settings(output);
        self.return_windows_home(output);
        self.rescue_windows();
        self.output_set_changed();
    }

    pub fn remove_output(&mut self, output: &Output) {
        let Some(area) = self.space.output_geometry(output) else {
            return;
        };
        let identity = Self::output_identity(output);
        let leaving: Vec<(Window, Rectangle<i32, Logical>)> = self
            .space
            .elements()
            .filter(|w| self.space.outputs_for_element(w).first() == Some(output))
            .filter_map(|w| Some((w.clone(), self.space.element_geometry(w)?)))
            .collect();
        self.space.unmap_output(output);
        for (window, geometry) in leaving {
            window
                .user_data()
                .insert_if_missing(|| std::cell::RefCell::new(None::<Home>));
            *window
                .user_data()
                .get::<std::cell::RefCell<Option<Home>>>()
                .unwrap()
                .borrow_mut() = Some(Home {
                output: identity.clone(),
                offset: geometry.loc - area.loc,
            });
            if let Some(target) = self.nearest_output(geometry) {
                self.place_whole(&window, geometry, &target, geometry.loc - area.loc);
            }
        }
        self.output_set_changed();
    }

    fn output_set_changed(&mut self) {
        self.update_frame_budget();
        self.sync_shell_outputs();
        self.queue_redraw_all();
        self.notify(Event::DisplaysChanged);
        self.notify(Event::WindowsChanged);
    }

    /// Windows that left this output's identity go back to their place on it.
    fn return_windows_home(&mut self, output: &Output) {
        let identity = Self::output_identity(output);
        let windows: Vec<Window> = self.space.elements().cloned().collect();
        for window in windows {
            let Some(home) = window.user_data().get::<std::cell::RefCell<Option<Home>>>() else {
                continue;
            };
            let offset = match &*home.borrow() {
                Some(home) if home.output == identity => home.offset,
                _ => continue,
            };
            home.borrow_mut().take();
            if let Some(geometry) = self.space.element_geometry(&window) {
                self.place_whole(&window, geometry, output, offset);
                info!(output = output.name(), "a window went back to its output");
            }
        }
    }

    /// Moves any window that no output shows onto the one nearest it.
    pub fn rescue_windows(&mut self) {
        let windows: Vec<Window> = self.space.elements().cloned().collect();
        for window in windows {
            if !self.space.outputs_for_element(&window).is_empty() {
                continue;
            }
            let Some(geometry) = self.space.element_geometry(&window) else {
                continue;
            };
            if let Some(target) = self.nearest_output(geometry) {
                let area = self.space.output_geometry(&target).unwrap();
                self.place_whole(&window, geometry, &target, geometry.loc - area.loc);
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

    /// Puts `window` at `offset` on `output`, moved so it is whole on it.
    fn place_whole(
        &mut self,
        window: &Window,
        geometry: Rectangle<i32, Logical>,
        output: &Output,
        offset: Point<i32, Logical>,
    ) {
        let Some(area) = self.space.output_geometry(output) else {
            return;
        };
        let size = geometry.size;
        let x = (area.loc.x + offset.x).clamp(
            area.loc.x,
            (area.loc.x + area.size.w - size.w).max(area.loc.x),
        );
        let y = (area.loc.y + offset.y).clamp(
            area.loc.y,
            (area.loc.y + area.size.h - size.h).max(area.loc.y),
        );
        let element_offset = self.space.element_location(window).unwrap_or_default() - geometry.loc;
        self.space
            .map_element(window.clone(), Point::from((x, y)) + element_offset, false);
    }
}
