//! The brief notice inside a window that has taken the keys, naming the chord that takes
//! them back, since the island is away over a full-screen VM (`surfaces/window-chrome.md`).

use std::{cell::RefCell, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{Glass, Glyph, colour, shadows, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::{Layer, Scope, SurfaceDef};

/// The app holding the keys, and the chord that takes them back.
type Text = (String, String);

#[derive(Default)]
pub struct InhibitNotice {
    text: Rc<RefCell<Text>>,
    state: Rc<RefCell<Option<State<Text>>>>,
}

impl InhibitNotice {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Puts the app's name and the chord in force; call it in the tree's root scope.
    pub fn set(&self, app: String, chord: String) {
        *self.text.borrow_mut() = (app.clone(), chord.clone());
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified((app, chord));
        }
    }

    /// The capsule's size for `app` and `chord`.
    pub fn size(theme: &Theme, app: &str, chord: &str) -> SSize<i32, Logical> {
        let t = &theme.tokens;
        let size = text::metrics(theme, text::Style::Callout).0;
        let line =
            format!("{app} is receiving all keyboard shortcuts. Press {chord} to take them back.");
        let w =
            2.0 * t.space_m + t.size_glyph + t.space_s + line.chars().count() as f32 * size * 0.56;
        (w.ceil() as i32 + 8, t.size_control_large.ceil() as i32 + 8).into()
    }
}

impl SurfaceDef for InhibitNotice {
    fn name(&self) -> &'static str {
        "inhibit-notice"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::Placed
    }

    fn takes_keyboard(&self) -> bool {
        false
    }

    fn geometry(&self, _output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size((1, 1).into())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(NoticeApp {
            text: self.text.clone(),
            state: self.state.clone(),
        })
    }

    fn anchor(&self) -> (f64, f64) {
        (0.5, 0.0)
    }

    fn corner_radius(&self, theme: &Theme) -> f32 {
        theme.tokens.size_control_large / 2.0
    }
}

#[derive(Clone)]
struct NoticeApp {
    text: Rc<RefCell<Text>>,
    state: Rc<RefCell<Option<State<Text>>>>,
}

impl App for NoticeApp {
    fn render(&self) -> impl IntoElement {
        use_drop({
            let slot = self.state.clone();
            move || {
                *slot.borrow_mut() = None;
            }
        });
        let state = use_hook(|| {
            let state = State::create(self.text.borrow().clone());
            *self.state.borrow_mut() = Some(state);
            state
        });
        Notice { text: state }
    }
}

#[derive(PartialEq)]
struct Notice {
    text: State<(String, String)>,
}

impl Component for Notice {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let (app, chord) = self.text.read().clone();
        let radius = t.size_control_large / 2.0;
        let ink = colour(t.colour_text);
        rect()
            .expanded()
            .padding(4.)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_control_large))
                    .corner_radius(radius)
                    .map(Some(shadows(&t.shadow_menu)), |el, list| {
                        list.into_iter().fold(el, |el, s| el.shadow(s))
                    })
                    .child(Glass::new(radius))
                    .a11y_role(AccessibilityRole::Alert)
                    // Padded inside, so the glass fills the whole capsule.
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::fill())
                            .padding((0., t.space_m))
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(t.space_s)
                            .child(Glyph::new("isle-keyboard-slash", t.size_glyph, ink))
                            .child(text::line(
                                &theme,
                                text::Style::Callout,
                                ink,
                                format!("{app} is receiving all keyboard shortcuts. Press {chord} to take them back."),
                            )),
                    ),
            )
    }
}
