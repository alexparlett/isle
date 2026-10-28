//! The wallpaper's menu: Wallpaper Settings… and Edit widgets (`p1/structure/wallpaper.md`
//! Pointer).

use isle_ui::{MenuEntry, MenuEvent, MenuItem};
use smithay::utils::{Logical, Point};

use crate::state::State;

const SETTINGS: u64 = 1;
const EDIT_WIDGETS: u64 = 2;

impl State {
    /// A secondary click on the wallpaper at `at` in the global space.
    pub fn open_wallpaper_menu(&mut self, at: Point<f64, Logical>) {
        let Some(output) = self
            .isle
            .space
            .outputs()
            .find(|o| self.isle.space.output_geometry(o).is_some_and(|g| g.to_f64().contains(at)))
            .cloned()
        else {
            return;
        };
        let entries = vec![
            MenuEntry::Item(MenuItem::new(SETTINGS, "Wallpaper Settings…")),
            MenuEntry::Item(MenuItem::new(EDIT_WIDGETS, "Edit widgets")),
        ];
        self.open_menu_at(entries, &output, at.to_i32_round());
        self.isle.wallpapers.menu = Some(output.name());
    }

    /// The window menu's surface chose for the wallpaper; false when the menu is not its.
    pub fn wallpaper_menu_event(&mut self, event: &MenuEvent) -> bool {
        if self.isle.wallpapers.menu.is_none() {
            return false;
        }
        match event {
            MenuEvent::Chosen(id) => {
                self.close_surface("window-menu");
                let output = self
                    .isle
                    .wallpapers
                    .menu
                    .take()
                    .and_then(|name| self.isle.space.outputs().find(|o| o.name() == name).cloned());
                match *id {
                    SETTINGS => {
                        self.open_settings(Some("wallpaper"), None);
                    }
                    EDIT_WIDGETS => self.toggle_dashboard(output, true, false),
                    _ => {}
                }
            }
            MenuEvent::Back => {
                self.isle.wallpapers.menu = None;
                self.close_surface("window-menu");
            }
            _ => {}
        }
        true
    }
}
