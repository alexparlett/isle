//! The full-output launcher drawn by Freya with Skia on the compositor's own GL context.

use std::time::Instant;

use freya::prelude::*;
use island_freya::{embed::Embedded, launcher};
use spike_base::{
    IslandContent, UiEvent,
    smithay::{
        backend::renderer::gles::{GlesRenderer, GlesTexture},
        utils::{Physical, Size as PhysicalSize},
    },
};

struct FreyaLauncher {
    ui: Embedded,
    launcher: launcher::Launcher,
}

impl IslandContent for FreyaLauncher {
    fn name(&self) -> &'static str {
        "freya"
    }

    // The launcher is specified in output pixels, so it ignores the window's scale.
    fn init(
        &mut self,
        renderer: &mut GlesRenderer,
        size: PhysicalSize<i32, Physical>,
        _scale: f64,
    ) {
        self.ui.init(renderer, size, 1.0);
    }

    fn set_expanded(&mut self, expanded: bool) {
        self.launcher.open.set_if_modified(expanded);
    }

    fn set_animated(&mut self, animated: bool) {
        self.launcher.animated.set_if_modified(animated);
    }

    fn set_layout_size(&mut self, _size: PhysicalSize<i32, Physical>) {}

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
        self.launcher.query.set("co".to_string());
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, launcher) = Embedded::new(|| launcher::app().into_element(), launcher::Launcher::new);
    spike_base::run_launcher(FreyaLauncher { ui, launcher })
}
