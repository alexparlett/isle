//! What an output shows, collected front to back through a push closure.

use smithay::{
    backend::renderer::{
        element::{
            AsRenderElements, Kind,
            memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
            surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree},
        },
        gles::GlesRenderer,
    },
    desktop::{LayerMap, layer_map_for_output},
    input::pointer::{CursorImageAttributes, CursorImageStatus},
    output::Output,
    render_elements,
    utils::{Logical, Point, Rectangle, Scale},
    wayland::{compositor::with_states, shell::wlr_layer::Layer},
};
use std::sync::Mutex;

use crate::state::Isle;

/// Where in the stack front to back a collection starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Stage {
    Cursor,
    Prompt,
    Scrim,
    ShellOverlay,
    LayerOverlay,
    ShellTop,
    Windows,
}

render_elements! {
    pub IsleElement<=GlesRenderer>;
    Surface = WaylandSurfaceRenderElement<GlesRenderer>,
    Memory = MemoryRenderBufferRenderElement<GlesRenderer>,
    Solid = smithay::backend::renderer::element::solid::SolidColorRenderElement,
    Window = crate::wm::draw::Placed,
    Shell = isle_shell::ShellElement,
    Wallpaper = crate::wallpaper::WallpaperElement,
}

impl IsleElement {
    /// What kind of thing the element draws, for the Test interface: `client` for any
    /// client's surface.
    #[cfg(feature = "test-hooks")]
    pub fn kind_name(&self) -> &'static str {
        match self {
            IsleElement::Surface(_) | IsleElement::Window(_) => "client",
            IsleElement::Memory(_) => "cursor",
            IsleElement::Solid(_) => "solid",
            IsleElement::Shell(_) => "shell",
            IsleElement::Wallpaper(_) => "wallpaper",
            _ => "other",
        }
    }
}

/// A themed cursor image the backend draws itself, with its hotspot.
pub struct NamedCursor<'a> {
    pub buffer: &'a MemoryRenderBuffer,
    pub hotspot: Point<i32, Logical>,
}

impl Isle {
    /// `cursor` is the themed image for a named cursor, when the backend draws one.
    /// Front to back: the cursor, shell overlays, layer-shell overlay and top, shell surfaces
    /// over windows, windows, layer-shell bottom and background, the shell's background, the
    /// wallpaper.
    pub fn render_elements(
        &self,
        renderer: &mut GlesRenderer,
        output: &Output,
        cursor: Option<NamedCursor<'_>>,
        push: &mut dyn FnMut(IsleElement),
    ) {
        #[cfg(feature = "test-hooks")]
        let mut kinds = Vec::new();
        #[cfg(feature = "test-hooks")]
        let push = &mut |e: IsleElement| {
            kinds.push(e.kind_name());
            push(e)
        };
        self.dim_elements(output, push);
        if self.lock.locked {
            self.render_locked(output, renderer, cursor, true, push);
        } else {
            self.render_elements_from(renderer, output, cursor, Stage::Cursor, push);
        }
        #[cfg(feature = "test-hooks")]
        self.lock.frame_kinds.borrow_mut().insert(output.name(), kinds);
    }

    /// What lies behind shell surfaces of `layer`, front to back, for their glass.
    pub fn render_elements_below(
        &self,
        renderer: &mut GlesRenderer,
        output: &Output,
        layer: isle_shell::Layer,
        push: &mut dyn FnMut(IsleElement),
    ) {
        if self.lock.locked {
            self.render_locked(output, renderer, None, false, push);
            return;
        }
        let from = match layer {
            isle_shell::Layer::Overlay | isle_shell::Layer::Lock => Stage::LayerOverlay,
            isle_shell::Layer::Prompt => Stage::Scrim,
            isle_shell::Layer::Top => Stage::Windows,
            isle_shell::Layer::Background => return,
        };
        self.render_elements_from(renderer, output, None, from, push);
    }

    fn render_elements_from(
        &self,
        renderer: &mut GlesRenderer,
        output: &Output,
        cursor: Option<NamedCursor<'_>>,
        from: Stage,
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

        match (&self.cursor_status, pointer.filter(|_| from <= Stage::Cursor)) {
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
        // The drag's icon, under the cursor and over every surface.
        if let (Some(icon), Some(at)) = (&self.dnd_icon, pointer.filter(|_| from <= Stage::Cursor)) {
            let location = at - geometry.loc.to_f64();
            for element in render_elements_from_surface_tree(
                renderer,
                icon,
                location.to_physical(scale).to_i32_round(),
                scale,
                1.0,
                Kind::Unspecified,
            ) {
                push(IsleElement::Surface(element));
            }
        }

        let shell = |layer, push: &mut dyn FnMut(IsleElement)| {
            self.shell
                .elements(output, layer, &mut |e| push(IsleElement::Shell(e)))
        };
        // A prompt and the island over its scrim, the scrim over the rest.
        let prompting = self.prompt_showing().is_some();
        if from <= Stage::Prompt && prompting {
            shell(isle_shell::Layer::Prompt, push);
            self.shell
                .elements_of(output, "island", &mut |e| push(IsleElement::Shell(e)));
        }
        if from <= Stage::Scrim && prompting {
            self.scrim_elements(output, push);
        }
        if from <= Stage::ShellOverlay {
            if prompting {
                self.shell.elements_except(
                    output,
                    isle_shell::Layer::Overlay,
                    "island",
                    &mut |e| push(IsleElement::Shell(e)),
                );
            } else {
                shell(isle_shell::Layer::Overlay, push);
            }
        }
        let layers = layer_map_for_output(output);
        if from <= Stage::LayerOverlay {
            push_layers(&layers, Layer::Overlay, renderer, scale, push);
            push_layers(&layers, Layer::Top, renderer, scale, push);
        }
        if from <= Stage::ShellTop {
            shell(isle_shell::Layer::Top, push);
        }

        // X11 windows left alone, front last, at the place they ask for, over windows.
        for window in self.unmanaged.iter().rev() {
            let (Some(x11), Some(surface)) = (window.x11_surface(), {
                use smithay::wayland::seat::WaylandFocus;
                window.wl_surface().map(|s| s.into_owned())
            }) else {
                continue;
            };
            let at = (x11.last_configure().loc - geometry.loc).to_physical_precise_round(scale);
            for element in render_elements_from_surface_tree(
                renderer,
                &surface,
                at,
                scale,
                1.0,
                Kind::Unspecified,
            ) {
                push(IsleElement::Surface(element));
            }
        }
        let preview = self.zone_preview.as_ref().filter(|p| &p.output == output);
        let output_scale = scale.x;
        for leaving in self.leaving.iter().filter(|l| &l.output == output) {
            let snapshot = &leaving.snapshot;
            let from = Rectangle::new(snapshot.frame.loc - geometry.loc, snapshot.frame.size);
            let mut to = leaving.motion.apply(snapshot.frame);
            to.loc -= geometry.loc.to_f64();
            push(IsleElement::Window(crate::wm::draw::place(
                snapshot_piece(renderer, snapshot, geometry.loc, output_scale, leaving.motion.opacity()),
                from,
                to,
                output_scale,
            )));
        }
        for window in self.space.elements_for_output(output).rev() {
            let id = crate::wm::window_id(window);
            let Some(frame) = self.frame(id) else {
                continue;
            };
            let record = self.wm.get(id);
            let motion = record.and_then(|r| r.motion);
            let morph = record.and_then(|r| r.morph.as_ref());
            let (mut to, alpha) = match (morph, motion) {
                (Some(morph), _) => (morph.frame(frame), morph.crossfade.value() as f32),
                (None, Some(motion)) => (motion.apply(frame), motion.opacity()),
                (None, None) => (frame.to_f64(), 1.0),
            };
            to.loc -= geometry.loc.to_f64();
            let from = Rectangle::new(frame.loc - geometry.loc, frame.size);
            let mut pieces = Vec::new();
            self.window_pieces(renderer, id, geometry.loc, output_scale, true, alpha, &mut |p| {
                pieces.push(p)
            });
            for piece in pieces {
                push(IsleElement::Window(crate::wm::draw::place(piece, from, to, output_scale)));
            }
            if let Some(morph) = morph {
                let snapshot = &morph.snapshot;
                let held = Rectangle::new(snapshot.frame.loc - geometry.loc, snapshot.frame.size);
                let fade = 1.0 - morph.crossfade.value() as f32;
                push(IsleElement::Window(crate::wm::draw::place(
                    snapshot_piece(renderer, snapshot, geometry.loc, output_scale, fade),
                    held,
                    to,
                    output_scale,
                )));
            }
            if let Some(preview) = preview.filter(|p| p.under == crate::wm::window_id(window)) {
                let at = (preview.rect.loc - geometry.loc).to_physical_precise_round(scale);
                push(IsleElement::Solid(
                    smithay::backend::renderer::element::solid::SolidColorRenderElement::from_buffer(
                        &preview.buffer,
                        at,
                        scale,
                        1.0,
                        Kind::Unspecified,
                    ),
                ));
            }
        }

        push_layers(&layers, Layer::Bottom, renderer, scale, push);
        push_layers(&layers, Layer::Background, renderer, scale, push);
        shell(isle_shell::Layer::Background, push);
        self.wallpaper_elements(output, &mut |e| push(IsleElement::Wallpaper(e)));
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

/// A window's last frame as a piece, at its frame relative to `origin`.
fn snapshot_piece(
    renderer: &mut GlesRenderer,
    snapshot: &crate::wm::motion::Snapshot,
    origin: Point<i32, Logical>,
    scale: f64,
    alpha: f32,
) -> crate::wm::draw::WindowPiece {
    let _ = renderer;
    let at = (snapshot.frame.loc - origin).to_f64().to_physical(scale);
    crate::wm::draw::WindowPiece::Snapshot(
        smithay::backend::renderer::element::texture::TextureRenderElement::from_texture_buffer(
            at,
            &snapshot.buffer,
            Some(alpha),
            None,
            Some(snapshot.frame.size),
            Kind::Unspecified,
        ),
    )
}
