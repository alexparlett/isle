//! ext-background-effect-v1: the blur behind the region of a window that asks for it.

use smithay::{
    reexports::wayland_server::DisplayHandle,
    utils::{Logical, Rectangle},
    wayland::{
        background_effect::{
            BackgroundEffectState, BackgroundEffectSurfaceCachedState, ExtBackgroundEffectHandler,
        },
        compositor::{RectangleKind, RegionAttributes, with_states},
        seat::WaylandFocus,
    },
};

use crate::state::{Isle, State};

pub fn create_global(dh: &DisplayHandle) {
    BackgroundEffectState::new::<State>(dh);
}

impl ExtBackgroundEffectHandler for State {}

/// The box a region's added rectangles cover; what it takes away is drawn through anyway.
pub fn bounds(region: &RegionAttributes) -> Option<Rectangle<i32, Logical>> {
    region
        .rects
        .iter()
        .filter(|(kind, _)| matches!(kind, RectangleKind::Add))
        .map(|(_, r)| *r)
        .reduce(|a, b| a.merge(b))
}

/// `blur` cut to `window`, each side that stops short of the window's edge grown by `radius`
/// under the window's own content, so the glass's rounded corners fall only on the window's.
pub fn backdrop_rect(
    blur: Rectangle<i32, Logical>,
    window: Rectangle<i32, Logical>,
    radius: i32,
) -> Option<Rectangle<i32, Logical>> {
    let cut = blur.intersection(window)?;
    let (mut x0, mut y0) = (cut.loc.x, cut.loc.y);
    let (mut x1, mut y1) = (x0 + cut.size.w, y0 + cut.size.h);
    let (wx1, wy1) = (window.loc.x + window.size.w, window.loc.y + window.size.h);
    if x0 > window.loc.x {
        x0 = (x0 - radius).max(window.loc.x);
    }
    if y0 > window.loc.y {
        y0 = (y0 - radius).max(window.loc.y);
    }
    if x1 < wx1 {
        x1 = (x1 + radius).min(wx1);
    }
    if y1 < wy1 {
        y1 = (y1 + radius).min(wy1);
    }
    Some(Rectangle::new((x0, y0).into(), (x1 - x0, y1 - y0).into()))
}

impl Isle {
    /// Each still window on `output` asking for a blur: its id and the glass's box relative
    /// to the output. None under Reduce Transparency.
    pub fn window_backdrops(
        &self,
        output: &smithay::output::Output,
    ) -> Vec<(u64, Rectangle<f64, Logical>)> {
        let theme = self.shell.theme();
        if theme.appearance.reduce_transparency {
            return Vec::new();
        }
        let Some(origin) = self.space.output_geometry(output) else {
            return Vec::new();
        };
        let radius = theme.tokens.radius_window as i32;
        self.space
            .elements_for_output(output)
            .filter_map(|window| {
                let id = crate::wm::window_id(window);
                let record = self.wm.get(id)?;
                if record.motion.is_some() || record.morph.is_some() {
                    return None;
                }
                let surface = window.wl_surface()?;
                let region = with_states(&surface, |states| {
                    states
                        .cached_state
                        .get::<BackgroundEffectSurfaceCachedState>()
                        .current()
                        .blur_region
                        .clone()
                })?;
                let rect = self.space.element_geometry(window)?;
                let surface_at = rect.loc - window.geometry().loc;
                let mut blur = bounds(&region)?;
                blur.loc += surface_at;
                let glass = backdrop_rect(blur, rect, radius)?;
                Some((
                    id,
                    Rectangle::new((glass.loc - origin.loc).to_f64(), glass.size.to_f64()),
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new((x, y).into(), (w, h).into())
    }

    #[test]
    fn a_sidebar_grows_under_the_content_only() {
        let window = rect(100, 100, 900, 700);
        assert_eq!(
            backdrop_rect(rect(100, 100, 220, 700), window, 10),
            Some(rect(100, 100, 230, 700))
        );
        assert_eq!(backdrop_rect(rect(0, 0, 50, 50), window, 10), None);
        let region = RegionAttributes {
            rects: vec![
                (RectangleKind::Add, rect(0, 0, 10, 10)),
                (RectangleKind::Add, rect(20, 0, 10, 30)),
            ],
        };
        assert_eq!(bounds(&region), Some(rect(0, 0, 30, 30)));
    }
}
