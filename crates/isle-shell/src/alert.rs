//! The alert the compositor hangs on a frozen window when close is pressed: the app is not
//! responding, Force Quit or Wait (`surfaces/window-chrome.md`, Alert).

use std::{cell::RefCell, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{Button, Hit, button::Kind, colour, shadows, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::surface::{Action, Actions, Layer, Scope, SurfaceDef};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertAction {
    ForceQuit,
    Wait,
}

/// One frozen window's alert, by its app's name.
pub struct NotRespondingAlert {
    name: Rc<RefCell<String>>,
}

impl NotRespondingAlert {
    pub fn new(name: String) -> Rc<Self> {
        Rc::new(Self {
            name: Rc::new(RefCell::new(name)),
        })
    }

    pub fn margin(theme: &Theme) -> f32 {
        isle_ui::shadow_reach(&theme.tokens.shadow_window).ceil()
    }

    /// The surface's box: the alert and room for its shadow, which the compositor centres
    /// under the window's bar.
    pub fn size_for(theme: &Theme) -> SSize<i32, Logical> {
        let t = &theme.tokens;
        let m = Self::margin(theme);
        let width = t.size_alert * theme.appearance.text_scale.max(1.0);
        (
            (width + 2.0 * m).ceil() as i32,
            (160.0 * t.type_scale_max.max(1.0) + m).ceil() as i32,
        )
            .into()
    }
}

impl SurfaceDef for NotRespondingAlert {
    fn name(&self) -> &'static str {
        "not-responding"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::Attached
    }

    fn takes_keyboard(&self) -> bool {
        true
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(output)
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(AlertApp {
            name: self.name.borrow().clone(),
        })
    }

    fn hit_shapes(&self) -> bool {
        true
    }
}

#[derive(Clone, PartialEq)]
struct AlertApp {
    name: String,
}

fn send(actions: &Actions, action: AlertAction) {
    actions.send(Action::Alert(action));
}

impl App for AlertApp {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let margin = NotRespondingAlert::margin(&theme);
        let width = t.size_alert * theme.appearance.text_scale.max(1.0);
        let (force, wait, keys) = (actions.clone(), actions.clone(), actions.clone());
        let button = (width - 2.0 * t.space_xl - t.space_s) / 2.0;
        rect()
            .expanded()
            .padding((0., margin, margin, margin))
            .on_global_key_down(move |e: Event<KeyboardEventData>| match &e.key {
                Key::Named(NamedKey::Enter | NamedKey::Escape) => send(&keys, AlertAction::Wait),
                _ => {}
            })
            .child(
                rect()
                    .width(Size::px(width))
                    .background(colour(t.colour_window))
                    .corner_radius((0., 0., t.radius_window, t.radius_window))
                    .map(Some(shadows(&t.shadow_window)), |el, list| {
                        list.into_iter().fold(el, |el, s| el.shadow(s))
                    })
                    .padding((t.space_l, t.space_xl))
                    .cross_align(Alignment::Center)
                    .child(Hit)
                    .child(text::line(
                        &theme,
                        text::Style::Headline,
                        colour(t.colour_text),
                        format!("{} is not responding", self.name),
                    ))
                    .child(rect().height(Size::px(t.space_xs)))
                    .child(
                        label()
                            .width(Size::fill())
                            .font_size(t.type_style_callout_size)
                            .color(colour(t.colour_text_2))
                            .text_align(TextAlign::Center)
                            .text("You will lose any unsaved changes."),
                    )
                    .child(rect().height(Size::px(t.space_l)))
                    .child(
                        rect()
                            .horizontal()
                            .spacing(t.space_s)
                            .child(
                                rect().width(Size::px(button)).child(
                                    Button::new("Force Quit")
                                        .kind(Kind::Destructive)
                                        .wide()
                                        .on_press(move |_| send(&force, AlertAction::ForceQuit)),
                                ),
                            )
                            .child(
                                rect().width(Size::px(button)).child(
                                    Button::new("Wait")
                                        .kind(Kind::Default)
                                        .wide()
                                        .on_press(move |_| send(&wait, AlertAction::Wait)),
                                ),
                            ),
                    ),
            )
    }
}
