use smithay::{
    desktop::{LayerSurface, WindowSurfaceType, layer_map_for_output},
    output::Output,
    reexports::wayland_server::protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    wayland::{
        compositor::with_states,
        shell::wlr_layer::{
            KeyboardInteractivity, Layer, LayerSurface as WlrLayerSurface, LayerSurfaceData,
            WlrLayerShellHandler, WlrLayerShellState,
        },
    },
};

use crate::state::State;

impl WlrLayerShellHandler for State {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.isle.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: WlrLayerSurface,
        output: Option<WlOutput>,
        _layer: Layer,
        namespace: String,
    ) {
        let output = output
            .as_ref()
            .and_then(Output::from_resource)
            .or_else(|| self.isle.active_output());
        let Some(output) = output else {
            surface.send_close();
            return;
        };
        let mut map = layer_map_for_output(&output);
        let _ = map.map_layer(&LayerSurface::new(surface, namespace));
    }

    fn layer_destroyed(&mut self, surface: WlrLayerSurface) {
        let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
        for output in outputs {
            let mut map = layer_map_for_output(&output);
            let layer = map
                .layers()
                .find(|l| l.layer_surface() == &surface)
                .cloned();
            if let Some(layer) = layer {
                map.unmap_layer(&layer);
                drop(map);
                self.isle.queue_redraw(&output);
            }
        }
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        if keyboard
            .current_focus()
            .is_none_or(|f| f.surface() == Some(surface.wl_surface()))
        {
            self.focus_next();
        }
    }
}

impl State {
    /// Arranges a layer surface's output on its commit, sending the initial configure
    /// after, so the size the client asked for is respected. True when it was a layer.
    pub(super) fn commit_layer(&mut self, surface: &WlSurface) -> bool {
        let Some(output) = self
            .isle
            .space
            .outputs()
            .find(|o| {
                layer_map_for_output(o)
                    .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                    .is_some()
            })
            .cloned()
        else {
            return false;
        };
        let initial_configure_sent = with_states(surface, |states| {
            states
                .data_map
                .get::<LayerSurfaceData>()
                .map(|d| d.lock().unwrap().initial_configure_sent)
                .unwrap_or(true)
        });
        let exclusive = {
            let mut map = layer_map_for_output(&output);
            map.arrange();
            let layer = map
                .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                .cloned()
                .unwrap();
            if !initial_configure_sent {
                layer.layer_surface().send_configure();
            }
            (initial_configure_sent
                && layer.cached_state().keyboard_interactivity == KeyboardInteractivity::Exclusive
                && matches!(layer.layer(), Layer::Top | Layer::Overlay))
            .then_some(layer)
        };
        if let Some(layer) = exclusive {
            let keyboard = self.isle.seat.get_keyboard().unwrap();
            if keyboard.current_focus().as_ref().and_then(|f| f.surface()) != Some(layer.wl_surface()) {
                let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                self.focus_keyboard(Some(layer.wl_surface().into()), serial);
            }
        }
        self.isle.queue_redraw(&output);
        true
    }
}

impl crate::state::Isle {
    /// A top or overlay layer under `pos` that takes the keyboard when clicked.
    pub fn layer_taking_focus_at(
        &self,
        pos: smithay::utils::Point<f64, smithay::utils::Logical>,
    ) -> Option<WlSurface> {
        let output = self.space.output_under(pos).next()?;
        let origin = self.space.output_geometry(output)?.loc.to_f64();
        let map = layer_map_for_output(output);
        [Layer::Overlay, Layer::Top]
            .into_iter()
            .find_map(|layer| map.layer_under(layer, pos - origin))
            .filter(|layer| layer.can_receive_keyboard_focus())
            .map(|layer| layer.wl_surface().clone())
    }
}
