//! The base with a blank island: the memory and frame-time floor.

use std::time::Instant;

use spike_base::{
    IslandContent, UiEvent,
    smithay::{
        backend::allocator::Fourcc,
        backend::renderer::{
            ImportMem,
            gles::{GlesRenderer, GlesTexture},
        },
        utils::{Physical, Size},
    },
};

#[derive(Default)]
struct Empty {
    texture: Option<GlesTexture>,
}

impl IslandContent for Empty {
    fn name(&self) -> &'static str {
        "empty"
    }

    fn init(&mut self, renderer: &mut GlesRenderer, size: Size<i32, Physical>, _scale: f64) {
        let pixels = vec![0u8; (size.w * size.h * 4) as usize];
        self.texture = Some(
            renderer
                .import_memory(&pixels, Fourcc::Abgr8888, (size.w, size.h).into(), false)
                .unwrap(),
        );
    }

    fn set_expanded(&mut self, _expanded: bool) {}
    fn prepare_shot(&mut self) {}
    fn set_layout_size(&mut self, _size: Size<i32, Physical>) {}
    fn event(&mut self, _event: UiEvent) {}

    fn tick(&mut self, _now: Instant) -> bool {
        false
    }

    fn render(&mut self, _renderer: &mut GlesRenderer) -> GlesTexture {
        self.texture.clone().unwrap()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--launcher") {
        spike_base::run_launcher(Empty::default())
    } else {
        spike_base::run(Empty::default())
    }
}
