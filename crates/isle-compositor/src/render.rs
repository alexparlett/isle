//! What an output shows, collected front to back through a push closure.

use smithay::{
    backend::renderer::{
        element::{
            AsRenderElements, Kind,
            memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
            surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree},
            texture::TextureRenderElement,
        },
        gles::{GlesRenderer, GlesTexture},
    },
    desktop::{LayerMap, layer_map_for_output},
    input::pointer::{CursorImageAttributes, CursorImageStatus},
    output::Output,
    render_elements,
    utils::{Logical, Point, Scale},
    wayland::{compositor::with_states, shell::wlr_layer::Layer},
};
use std::sync::Mutex;

use crate::state::Isle;

render_elements! {
    pub IsleElement<=GlesRenderer>;
    Surface = WaylandSurfaceRenderElement<GlesRenderer>,
    Memory = MemoryRenderBufferRenderElement<GlesRenderer>,
    Shell = TextureRenderElement<GlesTexture>,
}

/// A themed cursor image the backend draws itself, with its hotspot.
pub struct NamedCursor<'a> {
    pub buffer: &'a MemoryRenderBuffer,
    pub hotspot: Point<i32, Logical>,
}

impl Isle {
    /// `cursor` is the themed image for a named cursor, when the backend draws one.
    /// Front to back: the cursor, shell overlays, layer-shell overlay and top, shell surfaces
    /// over windows, windows, layer-shell bottom and background, the shell's background.
    pub fn render_elements(
        &self,
        renderer: &mut GlesRenderer,
        output: &Output,
        cursor: Option<NamedCursor<'_>>,
        push: &mut dyn FnMut(IsleElement),
    ) {
        let Some(geometry) = self.space.output_geometry(output) else {
            return;
        };
        let scale = Scale::from(output.current_scale().fractional_scale());
        let pointer = self
            .seat
            .get_pointer()
            .map(|p| p.current_location())
            .filter(|p| geometry.to_f64().contains(*p));

        match (&self.cursor_status, pointer) {
            (CursorImageStatus::Surface(surface), Some(at)) => {
                let hotspot = with_states(surface, |states| {
                    states
                        .data_map
                        .get::<Mutex<CursorImageAttributes>>()
                        .map(|a| a.lock().unwrap().hotspot)
                        .unwrap_or_default()
                });
                let location = at - geometry.loc.to_f64() - hotspot.to_f64();
                for element in render_elements_from_surface_tree(
                    renderer,
                    surface,
                    location.to_physical(scale).to_i32_round(),
                    scale,
                    1.0,
                    Kind::Cursor,
                ) {
                    push(IsleElement::Surface(element));
                }
            }
            (CursorImageStatus::Named(_), Some(at)) => {
                if let Some(cursor) = cursor {
                    let location = at - geometry.loc.to_f64() - cursor.hotspot.to_f64();
                    if let Ok(element) = MemoryRenderBufferRenderElement::from_buffer(
                        renderer,
                        location.to_physical(scale),
                        cursor.buffer,
                        None,
                        None,
                        None,
                        Kind::Cursor,
                    ) {
                        push(IsleElement::Memory(element));
                    }
                }
            }
            _ => {}
        }

        let shell = |layer, push: &mut dyn FnMut(IsleElement)| {
            self.shell
                .elements(output, layer, &mut |e| push(IsleElement::Shell(e)))
        };
        shell(isle_shell::Layer::Overlay, push);
        let layers = layer_map_for_output(output);
        push_layers(&layers, Layer::Overlay, renderer, scale, push);
        push_layers(&layers, Layer::Top, renderer, scale, push);
        shell(isle_shell::Layer::Top, push);

        for window in self.space.elements_for_output(output).rev() {
            let Some(location) = self.space.element_location(window) else {
                continue;
            };
            let location: Point<i32, _> = location - window.geometry().loc - geometry.loc;
            for element in AsRenderElements::<GlesRenderer>::render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
                window,
                renderer,
                location.to_physical_precise_round(scale),
                scale,
                1.0,
            ) {
                push(IsleElement::Surface(element));
            }
        }

        push_layers(&layers, Layer::Bottom, renderer, scale, push);
        push_layers(&layers, Layer::Background, renderer, scale, push);
        shell(isle_shell::Layer::Background, push);
    }
}

fn push_layers(
    layers: &LayerMap,
    kind: Layer,
    renderer: &mut GlesRenderer,
    scale: Scale<f64>,
    push: &mut dyn FnMut(IsleElement),
) {
    for layer in layers.layers_on(kind).rev() {
        let Some(at) = layers.layer_geometry(layer) else {
            continue;
        };
        for element in AsRenderElements::<GlesRenderer>::render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
            layer,
            renderer,
            at.loc.to_physical_precise_round(scale),
            scale,
            1.0,
        ) {
            push(IsleElement::Surface(element));
        }
    }
}
