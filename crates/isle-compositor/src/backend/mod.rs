//! Where frames go and input comes from: a window on another compositor, or real hardware.

pub mod winit;

use smithay::{
    backend::{allocator::dmabuf::Dmabuf, renderer::element::RenderElementStates},
    output::Output,
};

use crate::state::Isle;

pub enum Backend {
    Winit(winit::Winit),
}

pub struct Rendered {
    pub damaged: bool,
    pub states: RenderElementStates,
}

impl Backend {
    pub fn seat_name(&self) -> String {
        match self {
            Backend::Winit(_) => "winit".into(),
        }
    }

    pub fn render(&mut self, isle: &mut Isle, output: &Output) -> Result<Rendered, String> {
        match self {
            Backend::Winit(winit) => winit.render(isle, output),
        }
    }

    pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
        match self {
            Backend::Winit(winit) => winit.import_dmabuf(dmabuf),
        }
    }

    pub fn update_cursor(&mut self, isle: &Isle) {
        match self {
            Backend::Winit(winit) => winit.update_cursor(isle),
        }
    }
}
