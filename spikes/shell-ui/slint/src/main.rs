//! The island drawn by Slint's software renderer into a CPU buffer, uploaded by dirty region.

use std::time::Instant;

use chrono::Timelike;
use island_slint::{Surface, slint_interpreter::Value};
use spike_base::{
    IslandContent, SHOT_TEXT, UiEvent,
    smithay::{
        backend::renderer::gles::{GlesRenderer, GlesTexture},
        utils::{Physical, Size},
    },
};

struct SlintIsland {
    surface: Surface,
    scale: f64,
    second: Option<u32>,
}

impl IslandContent for SlintIsland {
    fn name(&self) -> &'static str {
        "slint"
    }

    fn init(&mut self, renderer: &mut GlesRenderer, size: Size<i32, Physical>, scale: f64) {
        self.scale = scale;
        self.surface.init(renderer, size, scale);
    }

    fn set_expanded(&mut self, expanded: bool) {
        self.surface.set("expanded", Value::Bool(expanded));
    }

    fn set_layout_size(&mut self, size: Size<i32, Physical>) {
        self.surface.set("layout-width", Value::Number(size.w as f64 / self.scale));
        self.surface.set("layout-height", Value::Number(size.h as f64 / self.scale));
    }

    fn event(&mut self, event: UiEvent) {
        self.surface.event(event);
    }

    fn tick(&mut self, _now: Instant) -> bool {
        let second = chrono::Local::now().num_seconds_from_midnight();
        if self.second != Some(second) {
            self.second = Some(second);
            self.surface.set("seconds", Value::Number(second as f64));
        }
        self.surface.draw()
    }

    fn render(&mut self, renderer: &mut GlesRenderer) -> GlesTexture {
        self.surface.upload(renderer, "island")
    }

    fn poll_reload(&mut self) -> bool {
        self.surface.poll_reload()
    }

    fn prepare_shot(&mut self) {
        self.surface.set("query", Value::String(SHOT_TEXT.into()));
        self.surface.set("volume", Value::Number(0.6));
        self.surface.invoke("focus-search");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    spike_base::run(SlintIsland { surface: Surface::new("island.slint", "Island")?, scale: 1.0, second: None })
}
