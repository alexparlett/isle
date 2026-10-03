//! What an output shows, collected front to back through a push closure.

use smithay::{
    backend::renderer::{
        ImportAll, ImportMem, Renderer, Texture,
        element::{
            AsRenderElements, Kind,
            surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree},
        },
    },
    desktop::{LayerMap, layer_map_for_output},
    input::pointer::{CursorImageAttributes, CursorImageStatus},
    output::Output,
    utils::{Point, Scale},
    wayland::{compositor::with_states, shell::wlr_layer::Layer},
};
use std::sync::Mutex;

use crate::state::Isle;

pub type IsleElement<R> = WaylandSurfaceRenderElement<R>;

impl Isle {
    pub fn render_elements<R>(
        &self,
        renderer: &mut R,
        output: &Output,
        push: &mut dyn FnMut(IsleElement<R>),
    ) where
        R: Renderer + ImportAll + ImportMem,
        R::TextureId: Texture + Clone + 'static,
    {
        let Some(geometry) = self.space.output_geometry(output) else {
            return;
        };
        let scale = Scale::from(output.current_scale().fractional_scale());

        if let CursorImageStatus::Surface(surface) = &self.cursor_status {
            let hotspot = with_states(surface, |states| {
                states
                    .data_map
                    .get::<Mutex<CursorImageAttributes>>()
                    .map(|a| a.lock().unwrap().hotspot)
                    .unwrap_or_default()
            });
            if let Some(pointer) = self.seat.get_pointer() {
                let location =
                    pointer.current_location() - geometry.loc.to_f64() - hotspot.to_f64();
                for element in render_elements_from_surface_tree(
                    renderer,
                    surface,
                    location.to_physical(scale).to_i32_round(),
                    scale,
                    1.0,
                    Kind::Cursor,
                ) {
                    push(element);
                }
            }
        }

        let layers = layer_map_for_output(output);
        push_layers(&layers, Layer::Overlay, renderer, scale, push);
        push_layers(&layers, Layer::Top, renderer, scale, push);

        for window in self.space.elements_for_output(output).rev() {
            let Some(location) = self.space.element_location(window) else {
                continue;
            };
            let location: Point<i32, _> = location - window.geometry().loc - geometry.loc;
            for element in AsRenderElements::<R>::render_elements::<IsleElement<R>>(
                window,
                renderer,
                location.to_physical_precise_round(scale),
                scale,
                1.0,
            ) {
                push(element);
            }
        }

        push_layers(&layers, Layer::Bottom, renderer, scale, push);
        push_layers(&layers, Layer::Background, renderer, scale, push);
    }
}

fn push_layers<R>(
    layers: &LayerMap,
    kind: Layer,
    renderer: &mut R,
    scale: Scale<f64>,
    push: &mut dyn FnMut(IsleElement<R>),
) where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Texture + Clone + 'static,
{
    for layer in layers.layers_on(kind).rev() {
        let Some(at) = layers.layer_geometry(layer) else {
            continue;
        };
        for element in AsRenderElements::<R>::render_elements::<IsleElement<R>>(
            layer,
            renderer,
            at.loc.to_physical_precise_round(scale),
            scale,
            1.0,
        ) {
            push(element);
        }
    }
}
