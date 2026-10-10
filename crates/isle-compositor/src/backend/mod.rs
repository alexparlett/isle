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
    pub planes: crate::present::Planes,
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

    /// Turns an output's screen off, or lets its next frame light it again.
    pub fn set_power(&mut self, output: &Output, on: bool) {
        if let Backend::Udev(udev) = self {
            udev.set_power(output, on);
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
        let size = output.current_mode().ok_or("the output has no mode")?.size;
        let renderer = self.renderer();
        let mut elements = Vec::new();
        isle.render_elements(renderer, output, None, &mut |e| elements.push(e));
        let mut texture = render_offscreen(renderer, output, &elements)?;
        read_back(renderer, &mut texture, Rectangle::from_size(size), false)
    }

    /// Draws what a picture of `output` holds into a texture of its own, the pointer in it
    /// when `pointer`.
    pub fn capture_screen(
        &mut self,
        isle: &Isle,
        output: &Output,
        pointer: bool,
    ) -> Result<GlesTexture, String> {
        let scale = output.current_scale().integer_scale().max(1) as u32;
        let (renderer, cursor) = match self {
            Backend::Udev(udev) => udev
                .renderer_with_cursor(isle, scale)
                .ok_or("no renderer")?,
            Backend::Winit(winit) => (winit.renderer(), None),
            Backend::Headless(headless) => (&mut headless.renderer, None),
        };
        let mut elements = Vec::new();
        isle.capture_elements(renderer, output, cursor, pointer, &mut |e| elements.push(e));
        render_offscreen(renderer, output, &elements)
    }

    /// What a picture of `output` holds, as `Isle::capture_elements` gives it, with the
    /// themed pointer when `pointer`.
    pub fn capture_elements(
        &mut self,
        isle: &Isle,
        output: &Output,
        pointer: bool,
        push: &mut dyn FnMut(crate::render::IsleElement),
    ) -> Option<()> {
        let scale = output.current_scale().integer_scale().max(1) as u32;
        let (renderer, cursor) = match self {
            Backend::Udev(udev) => udev.renderer_with_cursor(isle, scale)?,
            Backend::Winit(winit) => (winit.renderer(), None),
            Backend::Headless(headless) => (&mut headless.renderer, None),
        };
        isle.capture_elements(renderer, output, cursor, pointer, push);
        Some(())
    }

    /// The pointer's image at an integer `scale` from the top left of a `side`-pixel square,
    /// straight RGBA, and its hotspot in those pixels; `None` while it is hidden or has no
    /// image Isle can draw.
    pub fn cursor_image(
        &mut self,
        isle: &Isle,
        scale: u32,
        side: i32,
    ) -> Result<Option<(Pixels, smithay::utils::Point<i32, smithay::utils::Physical>)>, String> {
        use smithay::{
            backend::renderer::element::{
                Kind as ElementKind, memory::MemoryRenderBufferRenderElement,
                surface::render_elements_from_surface_tree,
            },
            input::pointer::{CursorImageAttributes, CursorImageStatus},
            wayland::compositor::with_states,
        };
        use crate::render::IsleElement;
        let (renderer, named) = match self {
            Backend::Udev(udev) => udev.renderer_with_cursor(isle, scale).ok_or("no renderer")?,
            Backend::Winit(winit) => (winit.renderer(), None),
            Backend::Headless(headless) => (&mut headless.renderer, None),
        };
        let mut elements = Vec::new();
        let hotspot = match &isle.cursor_status {
            CursorImageStatus::Surface(surface) => {
                let hotspot = with_states(surface, |states| {
                    states
                        .data_map
                        .get::<std::sync::Mutex<CursorImageAttributes>>()
                        .map(|a| a.lock().unwrap().hotspot)
                        .unwrap_or_default()
                });
                for element in render_elements_from_surface_tree(
                    renderer,
                    surface,
                    (0, 0),
                    scale as f64,
                    1.0,
                    ElementKind::Cursor,
                ) {
                    elements.push(IsleElement::Surface(element));
                }
                hotspot
            }
            CursorImageStatus::Named(_) => {
                let Some(named) = named else {
                    return Ok(None);
                };
                let element = MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    (0.0, 0.0),
                    named.buffer,
                    None,
                    None,
                    None,
                    ElementKind::Cursor,
                )
                .map_err(|e| e.to_string())?;
                elements.push(IsleElement::Memory(element));
                named.hotspot
            }
            CursorImageStatus::Hidden => return Ok(None),
        };
        if elements.is_empty() {
            return Ok(None);
        }
        let size = (side, side).into();
        let mut texture =
            crate::wm::motion::render_to_texture(renderer, size, scale as f64, &elements)?;
        let pixels = read_back(renderer, &mut texture, Rectangle::from_size(size), true)?;
        Ok(Some((pixels, hotspot.to_physical(scale as i32))))
    }

    /// Reads `rect` of `texture` back.
    pub fn read_back(
        &mut self,
        texture: &mut GlesTexture,
        rect: Rectangle<i32, smithay::utils::Physical>,
        straight: bool,
    ) -> Result<Pixels, String> {
        let renderer = self.renderer_opt().ok_or("no renderer")?;
        read_back(renderer, texture, rect, straight)
    }
}

/// `elements`, placed on `output`, drawn into a new texture of the output's mode size.
fn render_offscreen(
    renderer: &mut GlesRenderer,
    output: &Output,
    elements: &[crate::render::IsleElement],
) -> Result<GlesTexture, String> {
    let size = output.current_mode().ok_or("the output has no mode")?.size;
    let buffer = size.to_logical(1).to_buffer(1, Transform::Normal);
    let mut texture: GlesTexture =
        Offscreen::<GlesTexture>::create_buffer(renderer, Fourcc::Abgr8888, buffer)
            .map_err(|e| e.to_string())?;
    let mut tracker = OutputDamageTracker::new(
        size,
        output.current_scale().fractional_scale(),
        Transform::Normal,
    );
    {
        let mut framebuffer = renderer.bind(&mut texture).map_err(|e| e.to_string())?;
        tracker
            .render_output(renderer, &mut framebuffer, 0, elements, CLEAR)
            .map_err(|e| format!("{e:?}"))?;
    }
    Ok(texture)
}

/// `rect` of `texture` as RGBA, top row first; `straight` undoes premultiplied alpha.
fn read_back(
    renderer: &mut GlesRenderer,
    texture: &mut GlesTexture,
    rect: Rectangle<i32, smithay::utils::Physical>,
    straight: bool,
) -> Result<Pixels, String> {
    let framebuffer = renderer.bind(texture).map_err(|e| e.to_string())?;
    let buffer = Rectangle::new(
        (rect.loc.x, rect.loc.y).into(),
        (rect.size.w, rect.size.h).into(),
    );
    let mapping = renderer
        .copy_framebuffer(&framebuffer, buffer, Fourcc::Abgr8888)
        .map_err(|e| e.to_string())?;
    drop(framebuffer);
    let mut rgba = renderer
        .map_texture(&mapping)
        .map_err(|e| e.to_string())?
        .to_vec();
    if straight {
        for px in rgba.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a != 0 && a != 255 {
                for c in &mut px[..3] {
                    *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
    }
    Ok(Pixels {
        width: rect.size.w as u32,
        height: rect.size.h as u32,
        rgba,
    })
}
