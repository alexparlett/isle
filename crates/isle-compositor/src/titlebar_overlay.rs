//! `isle_titlebar_v1`: a window that takes server-side decorations asks for its bar to be
//! transparent, as on macOS, and Isle draws only the traffic lights over the window's own
//! toolbar at the inset it names (`p13-settings.md` choice 14); and a window floats over its
//! caller without its keyboard, as Quick Look does (`p15-files.md` choice 27).

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
    isle_float_v1::{self, IsleFloatV1},
    isle_titlebar_manager_v1::{self, IsleTitlebarManagerV1},
    isle_titlebar_v1::{self, IsleTitlebarV1},
};

pub fn create_global(dh: &DisplayHandle) {
    dh.create_global::<State, IsleTitlebarManagerV1, ()>(2, ());
}

/// A window floating over its caller.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Float {
    /// An xdg-foreign handle, or `isle:` and a shell surface's name.
    pub over: String,
    /// It takes the keyboard when it activates itself, not only on a click into its top band.
    pub on_request: bool,
    /// Where in the caller it opens from.
    pub from: Option<(i32, i32, i32, i32)>,
}

impl Float {
    /// The shell surface it floats over, when its caller is one of Isle's own.
    pub fn over_shell(&self) -> Option<&str> {
        self.over.strip_prefix("isle:")
    }
}

impl Isle {
    /// The float `id` is, when it is one.
    pub fn float_of(&self, id: u64) -> Option<&Float> {
        let surface = self.wm.get(id)?.surface()?;
        self.floats.get(&surface.id())
    }

    /// The window a float hangs from: its caller's, through the handle it exported.
    pub fn float_parent(&self, surface: &WlSurface) -> Option<u64> {
        let float = self.floats.get(&surface.id())?;
        if float.over_shell().is_some() {
            return None;
        }
        let caller = self.xdg_foreign_state.exported_surface(&float.over)?;
        self.wm.for_surface(&caller).map(|r| r.id)
    }

    /// The window a click on `window` at `at` focuses: a float takes the keyboard only from a
    /// click into its top band and otherwise leaves it with its caller, raised with it.
    pub fn click_focus(
        &mut self,
        window: Option<smithay::desktop::Window>,
        at: smithay::utils::Point<f64, smithay::utils::Logical>,
    ) -> Option<smithay::desktop::Window> {
        let id = crate::wm::window_id(window.as_ref()?);
        if self.float_of(id).is_none() {
            return window;
        }
        let frame = self.frame(id)?;
        let band = self.shell.theme().tokens.size_toolbar as f64;
        if at.y - (frame.loc.y as f64) < band {
            return window;
        }
        self.wm.raise(id);
        let caller = self.wm.get(id)?.parent?;
        self.window_by_id(caller)
    }

    /// The windows floating over a shell surface, drawn above the shell's overlays.
    pub fn floats_over_shell(&self) -> Vec<u64> {
        self.wm
            .records
            .iter()
            .filter(|r| r.mapped && r.surface().and_then(|s| self.floats.get(&s.id())).is_some_and(|f| f.over_shell().is_some()))
            .map(|r| r.id)
            .collect()
    }

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
        state: &mut Self,
        _client: &Client,
        _resource: &IsleTitlebarManagerV1,
        request: isle_titlebar_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        match request {
            isle_titlebar_manager_v1::Request::GetTitlebar { id, surface } => {
                init.init(id, surface);
            }
            isle_titlebar_manager_v1::Request::GetFloat { id, surface } => {
                state.isle.floats.insert(surface.id(), Float::default());
                init.init(id, surface);
            }
            isle_titlebar_manager_v1::Request::Destroy => {}
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

impl Dispatch<IsleFloatV1, WlSurface> for State {
    fn request(
        state: &mut Self,
        _client: &Client,
        _resource: &IsleFloatV1,
        request: isle_float_v1::Request,
        surface: &WlSurface,
        _dh: &DisplayHandle,
        _init: &mut DataInit<'_, Self>,
    ) {
        let Some(float) = state.isle.floats.get_mut(&surface.id()) else {
            return;
        };
        match request {
            isle_float_v1::Request::SetFloatOver { handle } => float.over = handle,
            isle_float_v1::Request::SetKeyboard { keyboard } => {
                float.on_request = matches!(keyboard.into_result(), Ok(isle_float_v1::Keyboard::OnRequest));
            }
            isle_float_v1::Request::OpenFrom { x, y, width, height } => float.from = Some((x, y, width, height)),
            isle_float_v1::Request::Destroy => {
                state.isle.floats.remove(&surface.id());
            }
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: smithay::reexports::wayland_server::backend::ClientId,
        _resource: &IsleFloatV1,
        surface: &WlSurface,
    ) {
        state.isle.floats.remove(&surface.id());
    }
}
