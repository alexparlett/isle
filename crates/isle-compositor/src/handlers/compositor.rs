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
            is_sync_subsurface, with_states,
        },
        shell::xdg::XdgToplevelSurfaceData,
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
            if let Some(index) = self
                .isle
                .unmapped
                .iter()
                .position(|w| w.toplevel().is_some_and(|t| t.wl_surface() == &root))
            {
                self.commit_unmapped(index, &root);
            } else if let Some(window) = self.isle.window_for_surface(&root) {
                window.on_commit();
                self.isle.queue_redraw_for_window(&window);
            }
        }

        self.commit_popup(surface);
        crate::grabs::handle_resize_commit(&mut self.isle.space, surface);
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
    fn commit_unmapped(&mut self, index: usize, surface: &WlSurface) {
        let window = self.isle.unmapped[index].clone();
        let toplevel = window.toplevel().unwrap();
        if !toplevel.is_initial_configure_sent() {
            toplevel.send_configure();
            return;
        }
        window.on_commit();
        let has_buffer =
            with_renderer_surface_state(surface, |s| s.buffer().is_some()).unwrap_or(false);
        if !has_buffer {
            return;
        }
        self.isle.unmapped.remove(index);
        let Some(output) = self.isle.active_output() else {
            return;
        };
        let area = self.isle.space.output_geometry(&output).unwrap();
        let size = window.geometry().size;
        let location = (
            area.loc.x + ((area.size.w - size.w) / 2).max(0),
            area.loc.y + ((area.size.h - size.h) / 2).max(0),
        );
        self.isle.space.map_element(window.clone(), location, false);
        let (app_id, title) = with_states(surface, |states| {
            let data = states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .unwrap()
                .lock()
                .unwrap();
            (
                data.app_id.clone().unwrap_or_default(),
                data.title.clone().unwrap_or_default(),
            )
        });
        info!(app_id, title, ?location, size = ?size, "mapped");
        self.isle.notify(crate::worker::Event::WindowsChanged);
        self.set_focus(Some(&window));
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
