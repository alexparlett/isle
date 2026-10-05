use smithay::{
    backend::renderer::utils::{on_commit_buffer_handler, with_renderer_surface_state},
    reexports::wayland_server::{
        Client,
        protocol::{wl_buffer, wl_surface::WlSurface},
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{
            CompositorClientState, CompositorHandler, CompositorState, get_parent,
            is_sync_subsurface,
        },
        shm::{ShmHandler, ShmState},
    },
};

use tracing::info;

use crate::state::{ClientState, State};

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.isle.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);

        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            match self.isle.wm.for_surface(&root).map(|r| (r.id, r.mapped)) {
                Some((id, false)) => self.commit_unmapped(id, &root),
                Some((id, true))
                    if !with_renderer_surface_state(&root, |s| s.buffer().is_some())
                        .unwrap_or(false) =>
                {
                    self.unmap_window(id);
                }
                Some((id, true)) => {
                    let window = self.isle.window_by_id(id).unwrap();
                    window.on_commit();
                    self.isle.queue_redraw_for_window(&window);
                    let record = self.isle.wm.get_mut(id).unwrap();
                    if record.record_at_size == Some(window.geometry().size) {
                        record.record_at_size = None;
                        self.isle.record_place(id);
                    }
                    self.check_borderless(id);
                }
                None => {}
            }
        }

        if self.commit_layer(surface) {
            return;
        }
        self.commit_popup(surface);
        self.handle_resize_commit(surface);
        if let smithay::input::pointer::CursorImageStatus::Surface(cursor) =
            &self.isle.cursor_status
            && cursor == surface
        {
            self.isle.queue_redraw_all();
        }
    }
}

impl State {
    /// A toplevel's first commit gets its initial configure; its first buffer maps it,
    /// centred on the active output and focused.
    fn commit_unmapped(&mut self, id: u64, surface: &WlSurface) {
        let window = self.isle.window_by_id(id).unwrap();
        let toplevel = window.toplevel().unwrap();
        if !toplevel.is_initial_configure_sent() {
            self.plan_window(id);
            toplevel.send_configure();
            return;
        }
        window.on_commit();
        let has_buffer =
            with_renderer_surface_state(surface, |s| s.buffer().is_some()).unwrap_or(false);
        if !has_buffer {
            return;
        }
        let size = window.geometry().size;
        self.isle.refresh_identity(id);
        let Some(location) = self.place_new_window(id) else {
            return;
        };
        let record = self.isle.wm.get_mut(id).unwrap();
        let (app_id, title) = record.names();
        record.mapped = true;
        record.planned = None;
        record.rect = smithay::utils::Rectangle::new(location, size);
        let full_screen = std::mem::take(&mut record.wants_full_screen);
        info!(id, app_id, title, ?location, size = ?size, "mapped");
        self.assign_desktop(id);
        self.isle.sync_space();
        self.set_focus(Some(&window));
        if full_screen {
            self.enter_full_screen(id, false);
        } else {
            self.check_borderless(id);
        }
        if matches!(self.isle.wm.get(id).map(|r| &r.mode), Some(crate::wm::Mode::Floating | crate::wm::Mode::Tiled { .. } | crate::wm::Mode::Filled { .. })) {
            self.animate_arriving(id, None);
        }
    }
}

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.isle.shm_state
    }
}
