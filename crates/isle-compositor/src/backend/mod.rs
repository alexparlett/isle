//! Where frames go and input comes from: a window on another compositor, textures for
//! tests, or real hardware.

pub mod headless;
pub mod udev;
pub mod winit;

use smithay::{
    backend::{
        allocator::{Fourcc, dmabuf::Dmabuf},
        renderer::{
            Bind, ExportMem, Offscreen,
            damage::OutputDamageTracker,
            element::RenderElementStates,
            gles::{GlesRenderer, GlesTexture},
        },
    },
    output::Output,
    utils::{Rectangle, Transform},
};

use crate::state::Isle;

/// What shows where nothing is drawn: under a full-screen window that does not cover its
/// output, the wallpaper adding nothing there.
pub const CLEAR: [f32; 4] = [0.11, 0.11, 0.12, 1.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Udev,
    Winit,
    Headless,
}

// One value for the life of the process; its size does not matter.
#[allow(clippy::large_enum_variant)]
pub enum Backend {
    Udev(udev::Udev),
    Winit(winit::Winit),
    Headless(headless::Headless),
}

pub struct Rendered {
    pub damaged: bool,
    pub states: RenderElementStates,
}

/// An output's pixels, RGBA, top row first.
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Backend {
    pub fn seat_name(&self) -> String {
        match self {
            Backend::Udev(udev) => udev.seat_name(),
            Backend::Winit(_) => "winit".into(),
            Backend::Headless(_) => "headless".into(),
        }
    }

    pub fn render(&mut self, isle: &mut Isle, output: &Output) -> Result<Rendered, String> {
        match self {
            Backend::Udev(udev) => udev.render(isle, output),
            Backend::Winit(winit) => winit.render(isle, output),
            Backend::Headless(headless) => headless.render(isle, output),
        }
    }

    /// Whether frames can be drawn now: not while another session has the console.
    pub fn can_draw(&self) -> bool {
        match self {
            Backend::Udev(udev) => smithay::backend::session::Session::is_active(&udev.session),
            Backend::Winit(_) | Backend::Headless(_) => true,
        }
    }

    /// Whether a timer stands in for vblank: nothing scans the frames out.
    pub fn paces_by_timer(&self) -> bool {
        matches!(self, Backend::Headless(_))
    }

    pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
        match self {
            Backend::Udev(udev) => udev.import_dmabuf(dmabuf),
            Backend::Winit(winit) => winit.import_dmabuf(dmabuf),
            Backend::Headless(headless) => headless.import_dmabuf(dmabuf),
        }
    }

    pub fn update_cursor(&mut self, isle: &Isle) {
        match self {
            Backend::Winit(winit) => winit.update_cursor(isle),
            Backend::Udev(_) | Backend::Headless(_) => {}
        }
    }

    /// The renderer, while there is one: the udev backend has none while the session is away.
    pub fn renderer_opt(&mut self) -> Option<&mut GlesRenderer> {
        match self {
            Backend::Udev(udev) => udev.renderer_opt(),
            Backend::Winit(winit) => Some(winit.renderer()),
            Backend::Headless(headless) => Some(&mut headless.renderer),
        }
    }

    fn renderer(&mut self) -> &mut GlesRenderer {
        match self {
            Backend::Udev(udev) => udev.renderer(),
            Backend::Winit(winit) => winit.renderer(),
            Backend::Headless(headless) => &mut headless.renderer,
        }
    }

    /// Draws `output` whole into a texture of its own and reads it back.
    pub fn screenshot(&mut self, isle: &Isle, output: &Output) -> Result<Pixels, String> {
        let mode = output.current_mode().ok_or("the output has no mode")?;
        let size = mode.size;
        let buffer = size.to_logical(1).to_buffer(1, Transform::Normal);
        let renderer = self.renderer();
        let mut texture: GlesTexture =
            Offscreen::<GlesTexture>::create_buffer(renderer, Fourcc::Abgr8888, buffer)
                .map_err(|e| e.to_string())?;
        let mut tracker = OutputDamageTracker::new(
            size,
            output.current_scale().fractional_scale(),
            Transform::Normal,
        );
        let mut framebuffer = renderer.bind(&mut texture).map_err(|e| e.to_string())?;
        let mut elements = Vec::new();
        isle.render_elements(renderer, output, None, &mut |e| elements.push(e));
        tracker
            .render_output(renderer, &mut framebuffer, 0, &elements, CLEAR)
            .map_err(|e| format!("{e:?}"))?;
        let mapping = renderer
            .copy_framebuffer(&framebuffer, Rectangle::from_size(buffer), Fourcc::Abgr8888)
            .map_err(|e| e.to_string())?;
        let rgba = renderer
            .map_texture(&mapping)
            .map_err(|e| e.to_string())?
            .to_vec();
        Ok(Pixels {
            width: size.w as u32,
            height: size.h as u32,
            rgba,
        })
    }
}
