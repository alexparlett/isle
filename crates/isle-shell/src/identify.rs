//! Each screen's name in its top leading corner, while Settings identifies the displays.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_ui::{Glass, Glyph, colour, shadows, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::{Here, Layer, Scope, SurfaceDef};

#[derive(Default)]
pub struct IdentifySurface {
    /// Each output's name for people, by its connector.
    labels: Rc<RefCell<BTreeMap<String, String>>>,
}

impl IdentifySurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The names to show, before the surface opens.
    pub fn set(&self, labels: BTreeMap<String, String>) {
        *self.labels.borrow_mut() = labels;
    }
}

impl SurfaceDef for IdentifySurface {
    fn name(&self) -> &'static str {
        "identify"
    }

    fn layer(&self) -> Layer {
        Layer::Overlay
    }

    fn scope(&self) -> Scope {
        Scope::EveryOutput
    }

    fn takes_keyboard(&self) -> bool {
        false
    }

    fn geometry(&self, _output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::new((24, 56).into(), (360, 96).into())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(IdentifyApp {
            labels: self.labels.clone(),
        })
    }

    fn anchor(&self) -> (f64, f64) {
        (0.0, 0.0)
    }

    fn corner_radius(&self, theme: &isle_tokens::Theme) -> f32 {
        theme.tokens.radius_panel
    }

    fn fades(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct IdentifyApp {
    labels: Rc<RefCell<BTreeMap<String, String>>>,
}

impl App for IdentifyApp {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let here = use_consume::<Here>().0;
        let label = self.labels.borrow().get(&here).cloned().unwrap_or(here);
        let ink = colour(t.colour_text);
        rect()
            .expanded()
            .corner_radius(t.radius_panel)
            .map(Some(shadows(&t.shadow_panel)), |el, list| {
                list.into_iter().fold(el, |el, s| el.shadow(s))
            })
            .child(Glass::new(t.radius_panel))
            .child(
                rect()
                    .expanded()
                    .padding((0., t.space_xl))
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_m)
                    .a11y_role(AccessibilityRole::Alert)
                    .child(Glyph::new("monitor", 32., ink))
                    .child(text::line(&theme, text::Style::Title, ink, label)),
            )
    }
}
