//! The themed cursor images Isle draws itself, on the DRM backend.

use std::collections::HashMap;

use smithay::{
    backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
    input::pointer::CursorIcon,
    utils::{Logical, Point, Rectangle, Size, Transform},
};
use tracing::warn;
use xcursor::{CursorTheme, parser::Image};

use crate::render::NamedCursor;

/// A loaded image, its hotspot, and at a fractional scale its logical size and source.
type Loaded = Option<Image_>;

struct Image_ {
    buffer: MemoryRenderBuffer,
    hotspot: Point<i32, Logical>,
    size: Option<Size<i32, Logical>>,
    src: Option<Rectangle<f64, Logical>>,
}

pub struct Cursors {
    theme: CursorTheme,
    size: u32,
    /// By icon and scale in thousandths.
    cache: HashMap<(CursorIcon, u32), Loaded>,
}

impl Cursors {
    /// `XCURSOR_THEME` and `XCURSOR_SIZE`, as toolkits read them; P14 sets them from tokens.
    pub fn load() -> Self {
        let name = std::env::var("XCURSOR_THEME").unwrap_or_else(|_| "default".into());
        let size = std::env::var("XCURSOR_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24);
        Self {
            theme: CursorTheme::load(&name),
            size,
            cache: HashMap::new(),
        }
    }

    /// The image for `icon` at the output's `scale`, falling back to the default arrow.
    pub fn get(&mut self, icon: CursorIcon, scale: f64) -> Option<NamedCursor<'_>> {
        let key = (icon, (scale * 1000.0).round() as u32);
        if !self.cache.contains_key(&key) {
            let loaded = self
                .image(icon, scale)
                .or_else(|| self.image(CursorIcon::Default, scale))
                .map(|image| self.prepare(image, scale));
            if loaded.is_none() {
                warn!(?icon, "no cursor image in the theme");
            }
            self.cache.insert(key, loaded);
        }
        self.cache[&key].as_ref().map(|image| NamedCursor {
            buffer: &image.buffer,
            hotspot: image.hotspot,
            size: image.size,
            src: image.src,
        })
    }

    /// At an integer scale the image is the buffer, at that buffer scale. At a fractional
    /// one it is resampled to exactly the pixels it covers, so the cursor plane, which
    /// cannot scale, takes it.
    fn prepare(&self, image: Image, scale: f64) -> Image_ {
        // Xcursor keeps each pixel as a little-endian ARGB word, which is DRM's Argb8888:
        // the cursor plane's format, so the image can go there.
        if scale.fract() == 0.0 {
            let s = scale as u32;
            return Image_ {
                buffer: MemoryRenderBuffer::from_slice(
                    &image.pixels_rgba,
                    Fourcc::Argb8888,
                    (image.width as i32, image.height as i32),
                    s as i32,
                    Transform::Normal,
                    None,
                ),
                hotspot: Point::from(((image.xhot / s) as i32, (image.yhot / s) as i32)),
                size: None,
                src: None,
            };
        }
        let nominal = self.size as f64 / image.size.max(1) as f64;
        let logical = |px: u32| (px as f64 * nominal).round().max(1.0) as i32;
        let size = Size::<i32, Logical>::from((logical(image.width), logical(image.height)));
        let (w, h) = (
            (size.w as f64 * scale).round() as u32,
            (size.h as f64 * scale).round() as u32,
        );
        let pixels = resample(&image.pixels_rgba, image.width, image.height, w, h);
        Image_ {
            buffer: MemoryRenderBuffer::from_slice(
                &pixels,
                Fourcc::Argb8888,
                (w as i32, h as i32),
                1,
                Transform::Normal,
                None,
            ),
            hotspot: Point::from((
                (image.xhot as f64 * nominal).round() as i32,
                (image.yhot as f64 * nominal).round() as i32,
            )),
            size: Some(size),
            src: Some(Rectangle::from_size((w as f64, h as f64).into())),
        }
    }

    fn image(&self, icon: CursorIcon, scale: f64) -> Option<Image> {
        let names = std::iter::once(icon.name()).chain(icon.alt_names().iter().copied());
        let path = names
            .chain(["left_ptr"])
            .find_map(|name| self.theme.load_icon(name))?;
        let bytes = std::fs::read(path).ok()?;
        let images = xcursor::parser::parse_xcursor(&bytes)?;
        let want = (self.size as f64 * scale.ceil()) as i64;
        images
            .into_iter()
            .min_by_key(|image| (image.size as i64 - want).abs())
    }
}

/// Bilinear resampling of premultiplied four-byte pixels.
fn resample(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    let at = |x: u32, y: u32, c: usize| src[((y * sw + x) * 4) as usize + c] as f64;
    for y in 0..dh {
        let fy = ((y as f64 + 0.5) * sh as f64 / dh as f64 - 0.5).clamp(0.0, (sh - 1) as f64);
        let (y0, ty) = (fy.floor() as u32, fy.fract());
        let y1 = (y0 + 1).min(sh - 1);
        for x in 0..dw {
            let fx = ((x as f64 + 0.5) * sw as f64 / dw as f64 - 0.5).clamp(0.0, (sw - 1) as f64);
            let (x0, tx) = (fx.floor() as u32, fx.fract());
            let x1 = (x0 + 1).min(sw - 1);
            for c in 0..4 {
                let top = at(x0, y0, c) * (1.0 - tx) + at(x1, y0, c) * tx;
                let bottom = at(x0, y1, c) * (1.0 - tx) + at(x1, y1, c) * tx;
                out[((y * dw + x) * 4) as usize + c] = (top * (1.0 - ty) + bottom * ty).round() as u8;
            }
        }
    }
    out
}
