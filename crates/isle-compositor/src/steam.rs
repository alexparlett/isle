//! Steam's desktop client as an app (`gaming.md` Steam's desktop client, Steam's Big
//! Picture): Quit through Steam, its toasts left alone, Big Picture a game that minimising
//! closes.

use std::time::{Duration, Instant};

use smithay::xwayland::X11Surface;
use tracing::info;

use crate::state::State;

/// Steam shows its main window again this soon after Big Picture closes, on whatever
/// desktop is current; Isle puts it back.
const RETURN_WINDOW: Duration = Duration::from_secs(10);

/// Steam's desktop ids, native and Flatpak, and how each is asked to exit.
const QUITS: &[(&str, &str)] = &[
    ("steam", "steam -shutdown"),
    ("com.valvesoftware.Steam", "flatpak run com.valvesoftware.Steam -shutdown"),
];

fn is_steam_class(window: &X11Surface) -> bool {
    window.class().eq_ignore_ascii_case("steam")
}

/// A Steam notification: placed where Steam asks, never focused, listed or barred.
pub fn is_toast(window: &X11Surface) -> bool {
    let title = window.title();
    is_steam_class(window) && title.starts_with("notificationtoasts_") && title.ends_with("_desktop")
}

/// Big Picture by its window: class `steam`, its title naming it.
pub fn is_big_picture(class: &str, title: &str) -> bool {
    class.eq_ignore_ascii_case("steam") && title.contains("Big Picture")
}

/// Where Big Picture is: the window once seen, and where Steam's main window was when it
/// opened.
#[derive(Default)]
pub struct BigPicture {
    pub window: Option<u64>,
    /// Steam's main window and its desktop.
    home: Option<(u64, u64)>,
    closed_at: Option<Instant>,
}

impl BigPicture {
    /// Steam's main window may go back to its desktop now.
    fn returning(&self, now: Instant) -> Option<(u64, u64)> {
        let closed = self.closed_at?;
        (now.duration_since(closed) <= RETURN_WINDOW)
            .then_some(self.home)
            .flatten()
    }
}

impl State {
    /// Quit for Steam asks Steam to exit; closing its windows only hides it in the tray.
    pub fn quit_steam(&mut self, app: &str) -> bool {
        let Some((_, command)) = QUITS.iter().find(|(id, _)| *id == app) else {
            return false;
        };
        info!(app, "quit through Steam");
        self.launch(crate::launch::Launch::Command((*command).into()))
    }

    /// A Steam window's title changed or it mapped: Big Picture is marked once, so later
    /// titles do not lose it.
    pub fn check_big_picture(&mut self, id: u64) {
        let Some(record) = self.isle.wm.get(id) else {
            return;
        };
        let Some(x11) = record.x11().cloned() else {
            return;
        };
        if self.isle.big_picture.window == Some(id) || !is_big_picture(&x11.class(), &x11.title()) {
            return;
        }
        let main = self
            .isle
            .wm
            .records
            .iter()
            .find(|r| {
                r.id != id
                    && r.mapped
                    && r.parent.is_none()
                    && r.x11().is_some_and(|x| is_steam_class(x) && !is_toast(x))
            })
            .map(|r| (r.id, r.desktop));
        info!(id, ?main, "Big Picture");
        self.isle.big_picture = BigPicture {
            window: Some(id),
            home: main,
            closed_at: None,
        };
        self.isle.wm.get_mut(id).unwrap().big_picture = true;
        self.isle.refresh_game(id);
        // Sized to its screen, it has the screen's own desktop as if it had asked.
        let covers = self.isle.output_of(id).is_some_and(|output| {
            self.isle.space.output_geometry(&output).map(|g| g.size) == Some(x11.geometry().size)
        });
        let full_screen = matches!(self.isle.wm.get(id).map(|r| &r.mode), Some(crate::wm::Mode::FullScreen { .. }));
        if covers && !full_screen {
            self.enter_full_screen(id, true);
        }
    }

    /// Steam minimises Big Picture rather than closing it: its desktop goes, the view
    /// returns, and it is listed nowhere until Steam shows it again.
    pub fn big_picture_minimised(&mut self, id: u64) -> bool {
        if self.isle.big_picture.window != Some(id) {
            return false;
        }
        info!(id, "Big Picture closed");
        self.leave_full_screen(id);
        self.minimise(id);
        if let Some(record) = self.isle.wm.get_mut(id) {
            record.parked = true;
        }
        self.isle.big_picture.closed_at = Some(Instant::now());
        self.isle.notify(crate::worker::Event::WindowsChanged);
        true
    }

    /// Steam showed Big Picture again.
    pub fn big_picture_shown(&mut self, id: u64) {
        if let Some(record) = self.isle.wm.get_mut(id)
            && record.parked
        {
            record.parked = false;
            self.isle.big_picture.closed_at = None;
            self.isle.notify(crate::worker::Event::WindowsChanged);
        }
    }

    /// Steam's main window came back after Big Picture closed: it goes to the desktop it was
    /// on when Big Picture opened.
    pub fn steam_window_back(&mut self, id: u64) {
        let Some((main, desktop)) = self.isle.big_picture.returning(Instant::now()) else {
            return;
        };
        if main != id || self.isle.desktops.locate(desktop).is_none() {
            return;
        }
        let Some(record) = self.isle.wm.get_mut(id) else {
            return;
        };
        if record.desktop != desktop {
            info!(id, desktop, "Steam's window back on its desktop");
            record.desktop = desktop;
            self.isle.sync_space();
            self.isle.notify(crate::worker::Event::WindowsChanged);
        }
        self.isle.big_picture.closed_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_picture_by_class_and_title() {
        assert!(is_big_picture("steam", "Steam Big Picture Mode"));
        assert!(is_big_picture("Steam", "Big Picture"));
        assert!(!is_big_picture("steam", "Steam"));
        assert!(!is_big_picture("steam_app_570", "Big Picture"));
    }

    #[test]
    fn the_main_window_returns_only_soon_after() {
        let now = Instant::now();
        let mut bp = BigPicture {
            window: Some(9),
            home: Some((3, 1)),
            closed_at: Some(now),
        };
        assert_eq!(bp.returning(now + Duration::from_secs(2)), Some((3, 1)));
        assert_eq!(bp.returning(now + Duration::from_secs(11)), None);
        bp.closed_at = None;
        assert_eq!(bp.returning(now), None);
    }
}
