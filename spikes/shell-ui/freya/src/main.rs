//! The island drawn by Freya with Skia on the compositor's own GL context.

use std::time::Instant;

use freya::prelude::*;
use island_freya::{embed::Embedded, island};
use spike_base::{
    IslandContent, SHOT_TEXT, UiEvent,
    smithay::{
        backend::renderer::gles::{GlesRenderer, GlesTexture},
        utils::{Physical, Size as PhysicalSize},
    },
};

struct FreyaIsland {
    ui: Embedded,
    island: island::Island,
}

impl IslandContent for FreyaIsland {
    fn name(&self) -> &'static str {
        "freya"
    }

    fn init(&mut self, renderer: &mut GlesRenderer, size: PhysicalSize<i32, Physical>, scale: f64) {
        self.ui.init(renderer, size, scale);
    }

    fn set_expanded(&mut self, expanded: bool) {
        self.island.expanded.set_if_modified(expanded);
    }

    fn set_layout_size(&mut self, size: PhysicalSize<i32, Physical>) {
        let scale = self.ui.scale() as f32;
        self.island
            .layout
            .set_if_modified(Some((size.w as f32 / scale, size.h as f32 / scale)));
    }

    fn event(&mut self, event: UiEvent) {
        self.ui.event(event);
    }

    fn tick(&mut self, _now: Instant) -> bool {
        self.ui.tick()
    }

    fn render(&mut self, renderer: &mut GlesRenderer) -> GlesTexture {
        self.ui.render(renderer)
    }

    fn prepare_shot(&mut self) {
        self.island.query.set(SHOT_TEXT.to_string());
        self.island.volume.set(0.6);
        self.ui.request_focus(self.island.field);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, island) = Embedded::new(|| island::app().into_element(), island::Island::new);
    spike_base::run(FreyaIsland { ui, island })
}
