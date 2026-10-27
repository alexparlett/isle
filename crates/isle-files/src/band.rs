//! The band: a rectangle dragged from a view's empty space that selects the items it passes
//! (`p1/surfaces/files.md` § Icons view, Band).

use std::path::PathBuf;

use freya::prelude::*;
use isle_ui::colour;

use crate::tab::Tab;

/// A band being dragged, in the view's content coordinates so it holds through a scroll.
#[derive(Clone, Debug, PartialEq)]
pub struct Band {
    from: (f32, f32),
    to: (f32, f32),
    /// The selection as the band began, which ⌘ or ⇧ adds the band's items to.
    base: Vec<PathBuf>,
    adding: bool,
}

impl Band {
    /// Its left, top, right and bottom.
    pub fn span(&self) -> (f32, f32, f32, f32) {
        let (a, b) = (self.from, self.to);
        (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
    }
}

/// A view's band and where its scrolled area sits in the window.
#[derive(Clone, Copy, PartialEq)]
pub struct Bander {
    band: State<Option<Band>>,
    origin: State<(f32, f32)>,
    controller: ScrollController,
}

pub fn use_band(controller: ScrollController) -> Bander {
    Bander {
        band: use_state(|| None),
        origin: use_state(|| (0., 0.)),
        controller,
    }
}

impl Bander {
    /// The scrolled area's place, from its `on_sized`.
    pub fn sized(mut self, area: Area) {
        self.origin.set_if_modified((area.origin.x, area.origin.y));
    }

    fn content(self, global: (f64, f64)) -> (f32, f32) {
        let (ox, oy) = *self.origin.peek();
        let (sx, sy): (i32, i32) = self.controller.into();
        (global.0 as f32 - ox - sx as f32, global.1 as f32 - oy - sy as f32)
    }

    /// A press on empty space: a plain one clears the selection, with ⌘ or ⇧ it is kept.
    pub fn begin(mut self, mut tab: State<Tab>, e: &MouseEventData) {
        let adding = e.modifiers.intersects(Modifiers::META | Modifiers::SHIFT);
        if !adding {
            tab.write().deselect();
        }
        let at = self.content((e.global_location.x, e.global_location.y));
        self.band.set(Some(Band { from: at, to: at, base: tab.peek().selection.clone(), adding }));
    }

    /// The pointer moved: the selection becomes what the band covers, with what it began with
    /// when adding.
    pub fn drag(mut self, mut tab: State<Tab>, global: (f64, f64), hits: impl Fn((f32, f32, f32, f32)) -> Vec<PathBuf>) {
        let Some(mut band) = self.band.peek().clone() else { return };
        band.to = self.content(global);
        let covered = hits(band.span());
        let mut selection = if band.adding { band.base.clone() } else { Vec::new() };
        for path in covered {
            match selection.iter().position(|p| *p == path) {
                Some(i) if band.adding && band.base.contains(&path) => {
                    selection.remove(i);
                }
                Some(_) => {}
                None => selection.push(path),
            }
        }
        if tab.peek().selection != selection {
            let mut t = tab.write();
            t.cursor = selection.last().cloned();
            t.selection = selection;
        }
        self.band.set(Some(band));
    }

    pub fn end(mut self) {
        if self.band.peek().is_some() {
            self.band.set(None);
        }
    }

    pub fn dragging(self) -> bool {
        self.band.peek().is_some()
    }

    /// The band drawn over the view.
    pub fn overlay(self, theme: &isle_tokens::Theme) -> Option<Element> {
        let t = &theme.tokens;
        let (x0, y0, x1, y1) = self.band.read().as_ref()?.span();
        let (sx, sy): (i32, i32) = self.controller.into();
        Some(
            rect()
                .position(Position::new_absolute().left(x0 + sx as f32).top(y0 + sy as f32))
                .layer(Layer::Overlay)
                .width(Size::px(x1 - x0))
                .height(Size::px(y1 - y0))
                .background(colour(t.colour_accent).with_a(38))
                .border(Border::new().fill(colour(t.colour_accent)).width(1.).alignment(BorderAlignment::Inner))
                .into_element(),
        )
    }
}

/// Whether two spans, left, top, right and bottom, meet.
pub fn meets(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_band_dragged_up_and_left_spans_the_same() {
        let band = Band { from: (50., 80.), to: (10., 20.), base: Vec::new(), adding: false };
        assert_eq!(band.span(), (10., 20., 50., 80.));
        assert!(meets(band.span(), (40., 70., 60., 90.)));
        assert!(!meets(band.span(), (50., 0., 60., 10.)));
    }
}
