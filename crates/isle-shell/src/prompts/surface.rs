//! The prompt's frame: who asks, the title and message, the fields, the choices, the status
//! line, the buttons and the queue line (`surfaces/prompts.md`, The frame).

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use freya::{animation::*, prelude::*};
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{
    AppIcon, Button, CheckRow, Field, FieldText, Glass, Glyph, Hit, Secret, Spinner, button::Kind,
    colour, shadows, text, use_theme,
};
use smithay::utils::{Logical, Rectangle, Size as SSize};
use zeroize::Zeroize;

use super::{Frame, Prompt, PromptAction, Status, more_line};
use crate::surface::{Action, Actions, Layer, Scope, SurfaceDef};

/// The typed text of the prompt on screen, which must outlive a rebuild of its tree.
#[derive(Default)]
struct Typed {
    prompt: u64,
    fields: Vec<(String, FieldText)>,
    choices: HashMap<String, bool>,
    shown: HashMap<String, bool>,
    identity: usize,
}

impl Typed {
    /// The buffers for `prompt`: kept while it is the same prompt, fresh for another.
    fn for_prompt(&mut self, prompt: &Prompt) {
        if self.prompt == prompt.id && self.fields.len() == prompt.fields.len() {
            return;
        }
        self.clear();
        self.prompt = prompt.id;
        self.fields = prompt
            .fields
            .iter()
            .map(|f| (f.id.clone(), FieldText::default()))
            .collect();
        self.choices = prompt
            .choices
            .iter()
            .map(|c| (c.id.clone(), c.checked))
            .collect();
        self.shown.clear();
        self.identity = 0;
    }

    fn clear(&mut self) {
        for (_, text) in &self.fields {
            text.borrow_mut().zeroize();
        }
    }

    fn clear_secrets(&mut self, prompt: &Prompt) {
        for (spec, (_, text)) in prompt.fields.iter().zip(&self.fields) {
            if spec.secret {
                text.borrow_mut().zeroize();
            }
        }
    }
}

#[derive(Default)]
pub struct PromptSurface {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
    typed: Rc<RefCell<Typed>>,
    size: Cell<SSize<i32, Logical>>,
}

impl PromptSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn set_model(&self, frame: Frame) {
        *self.frame.borrow_mut() = frame;
    }

    /// Puts `frame` in force; call it in the tree's root scope.
    pub fn set(&self, frame: Frame) {
        *self.frame.borrow_mut() = frame.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(frame);
        }
    }

    /// Wipes what was typed: the prompt was answered, cancelled or set aside.
    pub fn forget(&self) {
        let mut typed = self.typed.borrow_mut();
        typed.clear();
        typed.fields.clear();
        typed.prompt = 0;
    }

    pub fn set_size(&self, size: SSize<i32, Logical>) {
        self.size.set(size);
    }

    /// The room round the frame for its shadow.
    pub fn margin(theme: &Theme) -> f32 {
        isle_ui::shadow_reach(&theme.tokens.shadow_panel).ceil()
    }

    /// The surface's box: the frame's width and room for it to grow down.
    pub fn size_for(theme: &Theme) -> SSize<i32, Logical> {
        let t = &theme.tokens;
        let m = Self::margin(theme);
        (
            (t.size_panel + 2.0 * m).ceil() as i32,
            (640.0 * t.type_scale_max.max(1.0) + 2.0 * m).ceil() as i32,
        )
            .into()
    }
}

impl SurfaceDef for PromptSurface {
    fn name(&self) -> &'static str {
        "prompt"
    }

    fn layer(&self) -> Layer {
        Layer::Prompt
    }

    fn scope(&self) -> Scope {
        Scope::Placed
    }

    fn takes_keyboard(&self) -> bool {
        true
    }

    fn geometry(&self, _output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(self.size.get())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(PromptApp {
            frame: self.frame.clone(),
            state: self.state.clone(),
            typed: self.typed.clone(),
        })
    }

    fn anchor(&self) -> (f64, f64) {
        (0.5, 0.0)
    }

    fn hit_shapes(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct PromptApp {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
    typed: Rc<RefCell<Typed>>,
}

impl App for PromptApp {
    fn render(&self) -> impl IntoElement {
        use_drop({
            let slot = self.state.clone();
            move || {
                *slot.borrow_mut() = None;
            }
        });
        let state = use_hook(|| {
            let state = State::create(self.frame.borrow().clone());
            *self.state.borrow_mut() = Some(state);
            state
        });
        PromptView {
            frame: state,
            typed: Shared(self.typed.clone()),
        }
    }
}

#[derive(Clone)]
struct Shared(Rc<RefCell<Typed>>);

impl PartialEq for Shared {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(PartialEq)]
struct PromptView {
    frame: State<Frame>,
    typed: Shared,
}

fn send(actions: &Actions, action: PromptAction) {
    actions.send(Action::Prompt(action));
}

/// The prompt's answer from what was typed and chosen.
fn answer(prompt: &Prompt, typed: &Typed) -> PromptAction {
    PromptAction::Answer {
        id: prompt.id,
        values: typed
            .fields
            .iter()
            .map(|(id, text)| (id.clone(), Secret::new(text.borrow().as_str().to_owned())))
            .collect(),
        choices: prompt
            .choices
            .iter()
            .map(|c| {
                let on = typed.choices.get(&c.id).copied().unwrap_or(c.checked);
                (c.id.clone(), on)
            })
            .collect(),
        identity: typed.identity,
    }
}

impl Component for PromptView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = self.frame.read().clone();
        let actions = Actions::get();
        let margin = PromptSurface::margin(&theme);
        // Typing and toggles change the shared buffers; this draws them again.
        let mut version = use_state(|| 0u32);
        let _ = *version.read();
        let mut details_open = use_state(|| false);
        // The glass fills its box, so the frame takes its content's measured height.
        let mut content_height = use_state(|| 0.0f32);

        let Some(prompt) = frame.prompt.clone() else {
            return rect().expanded().into_element();
        };
        let typed = self.typed.0.clone();
        typed.borrow_mut().for_prompt(&prompt);

        // A wrong answer clears the secrets and shakes the first field.
        let seen = use_hook(|| Rc::new(Cell::new((prompt.id, prompt.wrongs))));
        let mut shake =
            use_animation(|_| AnimNum::new(0., 1.).time(320).function(Function::Linear));
        if seen.get() != (prompt.id, prompt.wrongs) {
            let same = seen.get().0 == prompt.id;
            seen.set((prompt.id, prompt.wrongs));
            if same {
                typed.borrow_mut().clear_secrets(&prompt);
                if !frame.still && !theme.appearance.reduce_motion {
                    shake.start();
                }
            }
        }
        let p = shake.read().value();
        let offset = if p > 0.0 && p < 1.0 {
            t.motion_shake_distance * (1.0 - p) * (p * 4.0 * std::f32::consts::PI).sin()
        } else {
            0.0
        };

        let ink = colour(t.colour_text);
        let ink_2 = colour(t.colour_text_2);
        let mismatch = prompt.confirm && {
            let typed = typed.borrow();
            let texts: Vec<String> = typed
                .fields
                .iter()
                .map(|(_, f)| f.borrow().as_str().to_owned())
                .collect();
            texts.len() >= 2 && !texts[1].is_empty() && texts[0] != texts[1]
        };
        let checking = prompt.status == Status::Checking;

        // Read at the moment of the key, not of the render: a field keeps its first closure.
        let frame_state = self.frame;
        let submit = {
            let (actions, typed) = (actions.clone(), typed.clone());
            move || {
                let frame = frame_state.peek();
                let Some(prompt) = &frame.prompt else {
                    return;
                };
                let typed = typed.borrow();
                let matches = !prompt.confirm || {
                    let texts: Vec<String> = typed
                        .fields
                        .iter()
                        .map(|(_, f)| f.borrow().as_str().to_owned())
                        .collect();
                    texts.len() < 2 || texts[0] == texts[1]
                };
                if frame.armed
                    && prompt.status != Status::Checking
                    && prompt.action.is_some()
                    && matches
                {
                    send(&actions, answer(prompt, &typed));
                }
            }
        };
        let cancel = {
            let (actions, id) = (actions.clone(), prompt.id);
            move || send(&actions, PromptAction::Cancel(id))
        };

        // Who asks: the app's icon with the kind's badge, else the kind's glyph alone.
        let who = match (&prompt.icon, &prompt.glyph) {
            (_, Some(glyph)) => rect().child(Glyph::new(glyph.clone(), 48., ink)),
            (Some(icon), None) => rect()
                .width(Size::px(48.))
                .height(Size::px(48.))
                .child(AppIcon::new(icon.clone(), 48.))
                .child(
                    rect()
                        .position(Position::new_absolute().right(-4.).bottom(-4.))
                        .width(Size::px(20.))
                        .height(Size::px(20.))
                        .corner_radius(10.)
                        .background(colour(t.material_solid))
                        .center()
                        .child(Glyph::new(prompt.kind.badge(), t.size_glyph_small, ink).fill(true)),
                ),
            (None, None) => rect().child(Glyph::new(prompt.kind.badge(), 48., ink)),
        };

        let mut body = rect()
            .width(Size::fill())
            .cross_align(Alignment::Center)
            .child(who)
            .child(rect().height(Size::px(t.space_m)))
            .child(text::line(
                &theme,
                text::Style::Title,
                ink,
                prompt.title.clone(),
            ))
            .maybe_child((!prompt.message.is_empty()).then(|| {
                rect().padding((t.space_xs, 0., 0., 0.)).child(
                    label()
                        .width(Size::fill())
                        .font_size(t.type_style_callout_size)
                        .color(ink_2)
                        .text_align(TextAlign::Center)
                        .max_lines(t.limit_prompt_lines as usize)
                        .text_overflow(TextOverflow::Ellipsis)
                        .text(prompt.message.clone()),
                )
            }));

        if let Some(code) = &prompt.code {
            body = body
                .child(rect().height(Size::px(t.space_l)))
                .child(code_view(&theme, code, prompt.typed));
        }

        let mut fields = rect().width(Size::fill()).spacing(t.space_m);
        if prompt.identities.len() > 1 {
            let chosen = typed.borrow().identity.min(prompt.identities.len() - 1);
            let count = prompt.identities.len();
            let typed_i = typed.clone();
            fields = fields.child(
                Button::new(format!("{} ▾", prompt.identities[chosen]))
                    .wide()
                    .on_press(move |_| {
                        let mut t = typed_i.borrow_mut();
                        t.identity = (t.identity + 1) % count;
                        drop(t);
                        *version.write() += 1;
                    }),
            );
        }
        let first_secret = prompt.fields.iter().position(|f| f.secret);
        for (i, spec) in prompt.fields.iter().enumerate() {
            let Some((_, text_buf)) = typed.borrow().fields.get(i).cloned() else {
                continue;
            };
            let shown = typed.borrow().shown.get(&spec.id).copied().unwrap_or(false);
            let submit = submit.clone();
            let field = Field::new(text_buf, spec.label.clone())
                .secret(spec.secret)
                .shown(shown)
                .mono(spec.mono)
                .enabled(!checking)
                .error(
                    Some(i) == first_secret && prompt.wrongs > 0 && prompt.status != Status::None,
                )
                .auto_focus(i == 0)
                .on_submit(move |_| submit())
                .on_change(move |_| *version.write() += 1);
            let field = if Some(i) == first_secret && offset != 0.0 {
                rect()
                    .width(Size::fill())
                    .padding((0., 0., 0., offset.max(0.0)))
                    .child(field)
            } else {
                rect().width(Size::fill()).child(field)
            };
            fields = fields.child(
                rect()
                    .width(Size::fill())
                    .spacing(t.space_xs)
                    .child(text::line(
                        &theme,
                        text::Style::Subheadline,
                        ink_2,
                        spec.label.clone(),
                    ))
                    .child(field),
            );
            if spec.show {
                let (typed_s, id) = (typed.clone(), spec.id.clone());
                fields = fields.child(CheckRow::new("Show password", shown).on_toggle(
                    move |on: bool| {
                        typed_s.borrow_mut().shown.insert(id.clone(), on);
                        *version.write() += 1;
                    },
                ));
            }
        }
        for choice in &prompt.choices {
            let on = typed
                .borrow()
                .choices
                .get(&choice.id)
                .copied()
                .unwrap_or(choice.checked);
            let (typed_c, id) = (typed.clone(), choice.id.clone());
            fields = fields.child(CheckRow::new(choice.label.clone(), on).on_toggle(
                move |on: bool| {
                    typed_c.borrow_mut().choices.insert(id.clone(), on);
                    *version.write() += 1;
                },
            ));
        }
        if !prompt.fields.is_empty() || !prompt.choices.is_empty() || prompt.identities.len() > 1 {
            body = body.child(rect().height(Size::px(t.space_l))).child(fields);
        }

        // The status line's slot is always there, so a message never moves the buttons.
        let status = if mismatch {
            Status::Warning("The passwords don't match.".into())
        } else {
            prompt.status.clone()
        };
        let line = match &status {
            Status::None => rect().height(Size::px(16.)),
            Status::Checking => rect()
                .height(Size::px(16.))
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .child(Spinner::new(12., "Checking"))
                .child(text::line(&theme, text::Style::Callout, ink_2, "Checking")),
            Status::Warning(words) => rect()
                .height(Size::px(16.))
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .child(Glyph::new("warning-circle", t.size_glyph_small, ink))
                .child(text::line(&theme, text::Style::Callout, ink, words.clone())),
            Status::Info(words) => rect()
                .height(Size::px(16.))
                .horizontal()
                .cross_align(Alignment::Center)
                .child(text::line(
                    &theme,
                    text::Style::Callout,
                    ink_2,
                    words.clone(),
                )),
        };
        body = body
            .child(rect().height(Size::px(t.space_xs)))
            .child(rect().width(Size::fill()).child(line));

        if let (Some(details), true) = (&prompt.details, *details_open.read()) {
            body = body.child(rect().height(Size::px(t.space_m))).child(
                rect()
                    .width(Size::fill())
                    .max_height(Size::px(160.))
                    .corner_radius(t.radius_small)
                    .background(colour(t.colour_fill))
                    .padding(t.space_s)
                    .child(
                        label()
                            .font_family("JetBrains Mono")
                            .font_size(t.type_style_footnote_size)
                            .color(ink_2)
                            .text(details.clone()),
                    ),
            );
        }

        let action_kind = if frame.armed {
            Kind::Default
        } else {
            Kind::Regular
        };
        // Cancel and the action are equal, wide enough for the longer label.
        let longest = prompt
            .action
            .iter()
            .chain([&prompt.cancel])
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0) as f32;
        let button_width = (longest * t.type_style_body_size * 0.6 + 24.0)
            .max(t.size_button_minimum)
            .ceil();
        let buttons = rect()
            .width(Size::fill())
            .content(Content::flex())
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .maybe_child(prompt.details.as_ref().map(|_| {
                let open = *details_open.read();
                rect().child(
                    Button::new("Details")
                        .kind(Kind::Plain)
                        .on_press(move |_| details_open.set(!open)),
                )
            }))
            .child(rect().width(Size::flex(1.)))
            .child(rect().width(Size::px(button_width)).child(
                Button::new(prompt.cancel.clone()).wide().on_press({
                    let cancel = cancel.clone();
                    move |_| cancel()
                }),
            ))
            .maybe_child(prompt.action.clone().map(|action| {
                let submit = submit.clone();
                rect().width(Size::px(button_width)).child(
                    Button::new(action)
                        .kind(action_kind)
                        .wide()
                        .enabled(!checking && !mismatch)
                        .on_press(move |_| submit()),
                )
            }));
        body = body
            .child(rect().height(Size::px(t.space_l)))
            .child(buttons);
        if let Some(more) = more_line(frame.more) {
            body = body
                .child(rect().height(Size::px(t.space_m)))
                .child(text::line(&theme, text::Style::Subheadline, ink_2, more));
        }

        let contrast = theme.appearance.increase_contrast;
        let solid = frame.solid || theme.appearance.reduce_transparency;
        let escape = cancel.clone();
        // With no field to take it, Return is the default button's.
        let fieldless = prompt.fields.is_empty();
        let enter = submit.clone();
        rect()
            .expanded()
            .padding(margin)
            .cross_align(Alignment::Center)
            .on_global_key_down(move |e: Event<KeyboardEventData>| match &e.key {
                Key::Named(NamedKey::Escape) => escape(),
                Key::Named(NamedKey::Enter) if fieldless => enter(),
                _ => {}
            })
            .child(
                rect()
                    .width(Size::px(t.size_panel))
                    .corner_radius(t.radius_panel)
                    .background(if solid {
                        colour(t.material_solid)
                    } else if contrast {
                        colour(t.material_glass_tinted)
                    } else {
                        Color::TRANSPARENT
                    })
                    .map(Some(shadows(&t.shadow_panel)), |el, list| {
                        list.into_iter().fold(el, |el, s| el.shadow(s))
                    })
                    .maybe(contrast, |el| {
                        el.border(
                            Border::new()
                                .fill(colour(t.colour_hairline_strong))
                                .width(t.stroke_hairline)
                                .alignment(BorderAlignment::Inner),
                        )
                    })
                    .maybe(*content_height.read() > 0.0, |el| {
                        el.height(Size::px(*content_height.read() + 2.0 * t.space_xl))
                    })
                    .maybe(!solid && *content_height.read() > 0.0, |el| {
                        el.child(Glass::new(t.radius_panel))
                    })
                    .child(Hit)
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding(t.space_xl)
                            .child(body.on_sized(move |e: Event<SizedEventData>| {
                                let h = e.area.height();
                                if (h - *content_height.peek()).abs() > 0.5 {
                                    content_height.set(h);
                                }
                            })),
                    ),
            )
            .into_element()
    }
}

/// A code to confirm, large; or one to type on the device, in cells marked as typed.
fn code_view(theme: &Theme, code: &str, typed: Option<usize>) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let ink = colour(t.colour_text);
    let large = |s: String| {
        label()
            .font_size(t.type_style_large_title_size)
            .font_weight(t.type_style_large_title_weight as i32)
            .color(ink)
            .text(s)
    };
    match typed {
        Some(done) if code.chars().count() <= 8 => rect()
            .horizontal()
            .spacing(t.space_xs)
            .a11y_alt(format!("{done} of {} digits typed", code.chars().count()))
            .children(code.chars().enumerate().map(|(i, digit)| {
                let typed = i < done;
                rect()
                    .width(Size::px(t.size_code_cell))
                    .height(Size::px(t.size_row_large))
                    .corner_radius(t.radius_small)
                    .background(if typed {
                        colour(t.colour_fill_strong)
                    } else {
                        colour(t.colour_fill)
                    })
                    .center()
                    .child(
                        label()
                            .font_size(t.type_style_large_title_size)
                            .font_weight(if typed { 600 } else { 400 })
                            .color(if typed { ink } else { colour(t.colour_text_2) })
                            .text(digit.to_string()),
                    )
                    .into_element()
            })),
        _ => rect().child(large(code.to_owned())),
    }
}
