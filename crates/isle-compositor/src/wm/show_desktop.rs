//! Show Desktop: every window on an output moved to its nearest side or foot edge, a sliver
//! of it left showing, until anything puts them back (`compositor.md` Stacking 10,
//! `p1/structure/window-chrome.md` Show Desktop; P16 choice 14).

use smithay::{output::Output, utils::Point};
use tracing::info;

use super::geometry::Rect;
use crate::state::{Isle, State};

/// An output's desktop while it is shown: the desktop, and each moved window's frame.
#[derive(Clone, Debug, Default)]
pub struct Shown {
    pub desktop: u64,
    pub frames: Vec<(u64, Rect)>,
}

/// Where `frame` goes on `screen` so `sliver` of it shows at its nearest of the left, right
/// and bottom edges; the top is the island's strip.
pub fn aside(frame: Rect, screen: Rect, sliver: i32) -> Point<i32, smithay::utils::Logical> {
    let cx = frame.loc.x + frame.size.w / 2 - screen.loc.x;
    let cy = frame.loc.y + frame.size.h / 2 - screen.loc.y;
    let (left, right, bottom) = (cx, screen.size.w - cx, screen.size.h - cy);
    if bottom <= left && bottom <= right {
        (frame.loc.x, screen.loc.y + screen.size.h - sliver).into()
    } else if left <= right {
        (screen.loc.x + sliver - frame.size.w, frame.loc.y).into()
    } else {
        (screen.loc.x + screen.size.w - sliver, frame.loc.y).into()
    }
}

impl Isle {
    /// Whether `output`'s desktop in view is shown aside.
    pub fn desktop_shown(&self, output: &Output) -> bool {
        let identity = Self::output_identity(output);
        self.shown_desktops
            .get(&identity)
            .is_some_and(|s| self.desktops.current_of(&identity) == Some(s.desktop))
    }
}

impl State {
    /// ⌘F3, the corner or the menu: shows the desktop of the focused output, or puts its
    /// windows back.
    pub fn toggle_show_desktop(&mut self) {
        let Some(output) = self
            .isle
            .output_with_focus()
            .or_else(|| self.isle.active_output())
        else {
            return;
        };
        if self.isle.desktop_shown(&output) {
            self.put_desktop_back(&output);
        } else {
            self.show_desktop(&output);
        }
    }

    pub fn show_desktop(&mut self, output: &Output) {
        let Some(screen) = self.isle.space.output_geometry(output) else {
            return;
        };
        let identity = Isle::output_identity(output);
        let Some(desktop) = self.isle.desktops.current_of(&identity) else {
            return;
        };
        let sliver = self.isle.shell.theme().tokens.size_sliver as i32;
        let ids: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| {
                r.parent.is_none()
                    && self.isle.wm.is_shown(r.id)
                    && self.isle.output_of(r.id).as_ref() == Some(output)
            })
            .map(|r| r.id)
            .collect();
        let mut frames = Vec::new();
        for id in ids {
            let Some(frame) = self.isle.frame(id) else {
                continue;
            };
            let to = aside(frame, screen, sliver);
            let moved = Rect::new(to, frame.size);
            self.begin_morph(id, moved);
            let bar = self.isle.bar_height(id);
            self.move_window(id, Point::from((to.x, to.y + bar)));
            frames.push((id, frame));
        }
        info!(
            output = output.name(),
            windows = frames.len(),
            "desktop shown"
        );
        self.isle
            .shown_desktops
            .insert(identity, super::show_desktop::Shown { desktop, frames });
        self.isle.queue_redraw(output);
    }

    /// Every window moved aside on `output` back to its place.
    pub fn put_desktop_back(&mut self, output: &Output) {
        let identity = Isle::output_identity(output);
        let Some(shown) = self.isle.shown_desktops.remove(&identity) else {
            return;
        };
        for (id, frame) in shown.frames {
            if self.isle.wm.get(id).is_none() {
                continue;
            }
            self.begin_morph(id, frame);
            let bar = self.isle.bar_height(id);
            self.move_window(id, Point::from((frame.loc.x, frame.loc.y + bar)));
        }
        info!(output = output.name(), "desktop put back");
        self.isle.queue_redraw(output);
    }

    /// Something that puts a shown desktop back happened to window `id`: it was activated,
    /// clicked or mapped.
    pub fn desktop_touched(&mut self, id: u64) {
        let Some(output) = self.isle.output_of(id) else {
            return;
        };
        if self.isle.desktop_shown(&output) {
            self.put_desktop_back(&output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect::new((x, y).into(), (w, h).into())
    }

    #[test]
    fn each_window_goes_to_its_nearest_side_or_the_foot() {
        let screen = rect(0, 0, 3440, 1440);
        assert_eq!(
            aside(rect(100, 200, 800, 600), screen, 24),
            Point::from((24 - 800, 200))
        );
        assert_eq!(
            aside(rect(2600, 200, 800, 600), screen, 24),
            Point::from((3440 - 24, 200))
        );
        assert_eq!(
            aside(rect(1400, 700, 600, 600), screen, 24),
            Point::from((1400, 1440 - 24))
        );
    }
}
