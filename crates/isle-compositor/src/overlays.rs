//! The surfaces at the centre of the screen that are open one at a time
//! (`p1/structure/competition.md` Centre of the screen).

use crate::state::State;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    Spotlight,
    Mission,
    Switcher,
    ForceQuit,
    SessionDialog,
    Toolbar,
    Dashboard,
}

impl State {
    /// Closes every one-at-a-time surface but `keep`, and the menu bar's open group.
    pub fn close_overlays(&mut self, keep: Overlay) {
        if keep != Overlay::Switcher {
            self.cancel_switcher();
        }
        if keep != Overlay::Mission {
            self.finish_mission();
        }
        if keep != Overlay::SessionDialog {
            self.close_session_dialog();
        }
        if keep != Overlay::ForceQuit {
            self.close_force_quit();
        }
        if keep != Overlay::Spotlight {
            self.close_spotlight();
        }
        if keep != Overlay::Toolbar {
            self.close_toolbar();
        }
        if keep != Overlay::Dashboard {
            self.close_dashboard();
        }
        self.close_menu_bar_group();
    }
}
