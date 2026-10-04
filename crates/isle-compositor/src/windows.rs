//! Windows and outputs as the D-Bus API and the CLI see them.

use isle_ipc::{DisplayInfo, WindowInfo};
use smithay::{
    desktop::Window,
    wayland::{compositor::with_states, shell::xdg::XdgToplevelSurfaceData},
};

use crate::{state::Isle, worker::Event};

/// A window's id, stable for its life and never reused.
struct WindowId(u64);

impl Isle {
    pub fn notify(&self, event: Event) {
        let _ = self.events.send(event);
    }

    pub fn assign_window_id(&mut self, window: &Window) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        window.user_data().insert_if_missing(|| WindowId(id));
    }

    pub fn window_id(window: &Window) -> u64 {
        window.user_data().get::<WindowId>().map_or(0, |id| id.0)
    }

    pub fn window_by_id(&self, id: u64) -> Option<Window> {
        self.space
            .elements()
            .find(|w| Self::window_id(w) == id)
            .cloned()
    }

    pub fn window_infos(&self) -> Vec<WindowInfo> {
        let focus = self.seat.get_keyboard().and_then(|k| k.current_focus());
        self.space
            .elements()
            .filter_map(|window| {
                let toplevel = window.toplevel()?;
                let (app_id, title) = with_states(toplevel.wl_surface(), |states| {
                    let data = states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()?
                        .lock()
                        .ok()?;
                    Some((
                        data.app_id.clone().unwrap_or_default(),
                        data.title.clone().unwrap_or_default(),
                    ))
                })?;
                let output = self
                    .space
                    .outputs_for_element(window)
                    .first()
                    .map(|o| o.name())
                    .unwrap_or_default();
                let geometry = self.space.element_geometry(window).unwrap_or_default();
                Some(WindowInfo {
                    x: geometry.loc.x,
                    y: geometry.loc.y,
                    width: geometry.size.w,
                    height: geometry.size.h,
                    id: Self::window_id(window),
                    app_id,
                    title,
                    output,
                    focused: focus.as_ref().and_then(|f| f.surface()) == Some(toplevel.wl_surface()),
                })
            })
            .collect()
    }

    pub fn display_infos(&self) -> Vec<DisplayInfo> {
        self.space
            .outputs()
            .map(|output| {
                let properties = output.physical_properties();
                let mode = output.current_mode();
                let location = self
                    .space
                    .output_geometry(output)
                    .map(|g| g.loc)
                    .unwrap_or_default();
                DisplayInfo {
                    name: output.name(),
                    make: properties.make,
                    model: properties.model,
                    serial: properties.serial_number,
                    width: mode.map_or(0, |m| m.size.w),
                    height: mode.map_or(0, |m| m.size.h),
                    refresh: mode.map_or(0, |m| m.refresh),
                    scale: output.current_scale().fractional_scale(),
                    x: location.x,
                    y: location.y,
                }
            })
            .collect()
    }
}
