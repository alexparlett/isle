//! `isle_titlebar_v1`: a window that takes server-side decorations asks for its bar to be
//! transparent, as on macOS, and Isle draws only the traffic lights over the window's own
//! toolbar at the inset it names (`p13-settings.md` choice 14).

use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
    protocol::wl_surface::WlSurface,
};

use crate::state::{Isle, State};

#[allow(
    missing_docs,
    clippy::all,
    non_upper_case_globals,
    non_camel_case_types,
    unused_imports
)]
pub mod protocol {
    use smithay::reexports::wayland_server;
    use smithay::reexports::wayland_server::protocol::*;

    pub mod __interfaces {
        use smithay::reexports::wayland_server::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../../data/protocols/isle-titlebar-v1.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_server_code!("../../data/protocols/isle-titlebar-v1.xml");
}

use protocol::{
    isle_titlebar_manager_v1::{self, IsleTitlebarManagerV1},
    isle_titlebar_v1::{self, IsleTitlebarV1},
};

pub fn create_global(dh: &DisplayHandle) {
    dh.create_global::<State, IsleTitlebarManagerV1, ()>(1, ());
}

impl Isle {
    /// The lights' inset over `id`, when it asked for a transparent bar.
    pub fn titlebar_overlay(&self, id: u64) -> Option<(i32, i32)> {
        let surface = self.wm.get(id)?.surface()?;
        self.titlebar_overlays.get(&surface.id()).copied()
    }
}

impl State {
    fn set_titlebar_overlay(&mut self, surface: &WlSurface, inset: Option<(i32, i32)>) {
        match inset {
            Some(inset) => self.isle.titlebar_overlays.insert(surface.id(), inset),
            None => self.isle.titlebar_overlays.remove(&surface.id()),
        };
        if let Some(id) = self.isle.wm.for_surface(surface).map(|r| r.id) {
            self.isle.sync_bars();
            self.isle.place_bar(id);
            self.isle.queue_redraw_all();
        }
    }
}

impl GlobalDispatch<IsleTitlebarManagerV1, ()> for State {
    fn bind(
        _state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<IsleTitlebarManagerV1>,
        _data: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        init.init(resource, ());
    }
}

impl Dispatch<IsleTitlebarManagerV1, ()> for State {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &IsleTitlebarManagerV1,
        request: isle_titlebar_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        if let isle_titlebar_manager_v1::Request::GetTitlebar { id, surface } = request {
            init.init(id, surface);
        }
    }
}

impl Dispatch<IsleTitlebarV1, WlSurface> for State {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &IsleTitlebarV1,
        request: isle_titlebar_v1::Request,
        surface: &WlSurface,
        _dh: &DisplayHandle,
        _init: &mut DataInit<'_, Self>,
    ) {
        match request {
            isle_titlebar_v1::Request::SetOverlay { x, y } => {
                state.set_titlebar_overlay(surface, Some((x, y)));
                let height = state.isle.shell.theme().tokens.size_titlebar.round() as u32;
                resource.controls(isle_shell::chrome::LIGHTS_END as u32, height);
            }
            isle_titlebar_v1::Request::UnsetOverlay | isle_titlebar_v1::Request::Destroy => {
                state.set_titlebar_overlay(surface, None);
            }
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: smithay::reexports::wayland_server::backend::ClientId,
        _resource: &IsleTitlebarV1,
        surface: &WlSurface,
    ) {
        state.isle.titlebar_overlays.remove(&surface.id());
    }
}
