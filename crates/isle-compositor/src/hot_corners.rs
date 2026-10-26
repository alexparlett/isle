//! The screens' corners: an action fires when the pointer rests in a corner, once a visit,
//! with the modifiers the corner needs (`mission-control.md`, Hot corners; `p13-settings.md`
//! choice 26).

use std::time::Duration;

use isle_settings::schema::Corner;
use smithay::{
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::{Logical, Point, Rectangle},
};
use tracing::info;

use crate::state::{Isle, State};

/// How near a screen's corner the pointer counts as in it, in points.
const REACH: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Which {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Default)]
pub struct HotCorners {
    /// The corner the pointer is in, by output, and whether it fired this visit.
    at: Option<(String, Which, bool)>,
    rest: Option<RegistrationToken>,
}

/// Which corner of `screen` the pointer at `at` is in; a corner where another screen meets
/// it is not one, so the pointer passes between screens.
pub fn corner_at(
    screen: Rectangle<f64, Logical>,
    at: Point<f64, Logical>,
    leads_on: impl Fn(Point<f64, Logical>) -> bool,
) -> Option<Which> {
    let (x0, y0) = (screen.loc.x, screen.loc.y);
    let (x1, y1) = (x0 + screen.size.w, y0 + screen.size.h);
    let left = at.x <= x0 + REACH;
    let right = at.x >= x1 - REACH - 1.0;
    let top = at.y <= y0 + REACH;
    let bottom = at.y >= y1 - REACH - 1.0;
    let (which, beyond) = match (left, right, top, bottom) {
        (true, _, true, _) => (Which::TopLeft, [(-1.0, 0.0), (0.0, -1.0), (-1.0, -1.0)]),
        (_, true, true, _) => (Which::TopRight, [(1.0, 0.0), (0.0, -1.0), (1.0, -1.0)]),
        (true, _, _, true) => (Which::BottomLeft, [(-1.0, 0.0), (0.0, 1.0), (-1.0, 1.0)]),
        (_, true, _, true) => (Which::BottomRight, [(1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]),
        _ => return None,
    };
    let open = beyond
        .iter()
        .all(|(dx, dy)| !leads_on((at.x + dx * (REACH + 2.0), at.y + dy * (REACH + 2.0)).into()));
    open.then_some(which)
}

impl Isle {
    /// The corner's action and the modifiers it needs.
    fn corner_setting(&self, which: Which) -> (Corner, isle_keys::chord::Mods) {
        let c = &self.settings.hot_corners;
        let (action, mods) = match which {
            Which::TopLeft => (c.top_left, &c.top_left_modifiers),
            Which::TopRight => (c.top_right, &c.top_right_modifiers),
            Which::BottomLeft => (c.bottom_left, &c.bottom_left_modifiers),
            Which::BottomRight => (c.bottom_right, &c.bottom_right_modifiers),
        };
        (action, isle_keys::chord::Mods::parse(mods).unwrap_or_default())
    }
}

impl State {
    /// The pointer moved: entering a corner starts its rest, leaving re-arms it.
    pub fn hot_corner_motion(&mut self) {
        let Some(pointer) = self.isle.seat.get_pointer() else {
            return;
        };
        let at = pointer.current_location();
        let found = self.isle.space.output_under(at).next().cloned().and_then(|output| {
            let screen = self.isle.space.output_geometry(&output)?.to_f64();
            let which = corner_at(screen, at, |p| self.isle.space.output_under(p).next().is_some())?;
            Some((output.name(), which))
        });
        let same = match (&self.isle.hot_corners.at, &found) {
            (Some((o, w, _)), Some((o2, w2))) => o == o2 && w == w2,
            _ => false,
        };
        if same {
            return;
        }
        if let Some(token) = self.isle.hot_corners.rest.take() {
            self.isle.loop_handle.remove(token);
        }
        self.isle.hot_corners.at = found.clone().map(|(o, w)| (o, w, false));
        let Some((_, which)) = found else {
            return;
        };
        if self.isle.corner_setting(which).0 == Corner::None {
            return;
        }
        let ms = self.isle.shell.theme().tokens.timing_hover_dwell_ms;
        self.isle.hot_corners.rest = self
            .isle
            .loop_handle
            .insert_source(
                Timer::from_duration(Duration::from_millis(ms as u64)),
                move |_, _, state| {
                    state.isle.hot_corners.rest = None;
                    state.hot_corner_rested(which);
                    TimeoutAction::Drop
                },
            )
            .ok();
    }

    fn hot_corner_rested(&mut self, which: Which) {
        let Some((output, at, fired)) = self.isle.hot_corners.at.clone() else {
            return;
        };
        if at != which || fired {
            return;
        }
        let (action, needs) = self.isle.corner_setting(which);
        let grabbed = self.isle.seat.get_pointer().is_some_and(|p| p.is_grabbed());
        let game = self
            .isle
            .space
            .outputs()
            .find(|o| o.name() == output)
            .cloned()
            .and_then(|o| self.isle.full_screen_front(&o))
            .is_some_and(|id| self.isle.wm.get(id).is_some_and(|r| r.game));
        if action == Corner::None
            || self.isle.lock.locked
            || grabbed
            || !self.isle.buttons_down.is_empty()
            || self.pointer_constrained()
            || game
            || self.isle.bar.open.is_some()
            || self.held_mods() != needs
        {
            return;
        }
        if let Some((_, _, fired)) = &mut self.isle.hot_corners.at {
            *fired = true;
        }
        info!(?which, ?action, "hot corner");
        match action {
            Corner::None => {}
            Corner::MissionControl => self.toggle_mission(false),
            Corner::AppWindows => self.toggle_mission(true),
            Corner::Desktop => {}
            Corner::NotificationCentre => {
                if self.isle.centre_open() {
                    self.close_control_centre();
                } else {
                    self.open_notification_centre(None);
                }
            }
            Corner::Apps => self.toggle_spotlight(isle_search::Scope::Apps),
            Corner::LockScreen => self.lock("hot corner"),
            Corner::DisplaySleep => self.displays_sleep(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_corner_is_one_where_no_screen_meets_it() {
        let left: Rectangle<f64, Logical> = Rectangle::new((0.0, 0.0).into(), (1920.0, 1080.0).into());
        let right: Rectangle<f64, Logical> = Rectangle::new((1920.0, 0.0).into(), (1920.0, 1080.0).into());
        let leads_on = |p: Point<f64, Logical>| left.contains(p) || right.contains(p);
        assert_eq!(corner_at(left, (0.0, 0.0).into(), leads_on), Some(Which::TopLeft));
        assert_eq!(corner_at(left, (0.0, 1079.0).into(), leads_on), Some(Which::BottomLeft));
        assert_eq!(corner_at(left, (1919.0, 0.0).into(), leads_on), None, "the screens meet");
        assert_eq!(corner_at(right, (3839.0, 1079.0).into(), leads_on), Some(Which::BottomRight));
        assert_eq!(corner_at(left, (900.0, 0.0).into(), leads_on), None);
    }
}
