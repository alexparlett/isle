//! The themed cursor images Isle draws itself, on the DRM backend.

use std::collections::HashMap;

use smithay::{
    backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
    input::pointer::CursorIcon,
    utils::{Logical, Point, Transform},
};
use tracing::warn;
use xcursor::{CursorTheme, parser::Image};

use crate::render::NamedCursor;

/// A loaded image and its hotspot, or `None` when the theme has nothing for the icon.
type Loaded = Option<(MemoryRenderBuffer, Point<i32, Logical>)>;

pub struct Cursors {
    theme: CursorTheme,
    size: u32,
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

    /// The image for `icon` at an integer `scale`, falling back to the default arrow.
    pub fn get(&mut self, icon: CursorIcon, scale: u32) -> Option<NamedCursor<'_>> {
        if !self.cache.contains_key(&(icon, scale)) {
            let loaded = self
                .image(icon, scale)
                .or_else(|| self.image(CursorIcon::Default, scale))
                .map(|image| {
                    let buffer = MemoryRenderBuffer::from_slice(
                        &image.pixels_rgba,
                        Fourcc::Abgr8888,
                        (image.width as i32, image.height as i32),
                        scale as i32,
                        Transform::Normal,
                        None,
                    );
                    let hotspot =
                        Point::from(((image.xhot / scale) as i32, (image.yhot / scale) as i32));
                    (buffer, hotspot)
                });
            if loaded.is_none() {
                warn!(?icon, "no cursor image in the theme");
            }
            self.cache.insert((icon, scale), loaded);
        }
        self.cache[&(icon, scale)]
            .as_ref()
            .map(|(buffer, hotspot)| NamedCursor {
                buffer,
                hotspot: *hotspot,
            })
    }

    fn image(&self, icon: CursorIcon, scale: u32) -> Option<Image> {
        let names = std::iter::once(icon.name()).chain(icon.alt_names().iter().copied());
        let path = names
            .chain(["left_ptr"])
            .find_map(|name| self.theme.load_icon(name))?;
        let bytes = std::fs::read(path).ok()?;
        let images = xcursor::parser::parse_xcursor(&bytes)?;
        let want = self.size * scale;
        images
            .into_iter()
            .min_by_key(|image| (image.size as i64 - want as i64).abs())
    }
}
