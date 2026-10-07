//! The rows of a Control Centre page, by kind (`structure/control-centre.md` Pages,
//! `surfaces/control-centre.md` Pages).

use std::path::PathBuf;

use freya::prelude::*;
use isle_tokens::Theme;
use isle_ui::{AppIcon, Glyph, Slider, Spinner, Switch, colour, focus_ring, text, use_theme};

/// A row on a module's page.
#[derive(Clone, Debug, PartialEq)]
pub enum PageRow {
    /// A section's name: "Known networks", "Output".
    Heading(String),
    /// A row that chooses one thing or acts on it.
    Choice(Choice),
    /// A level with its name in a column before it.
    Slider(SliderRow),
    /// Something turned on or off.
    Switch {
        id: String,
        label: String,
        on: bool,
        enabled: bool,
    },
    /// A line of text: an empty list's, or why something is missing.
    Status(String),
    /// A level read live, as an input's: a bar, not a control.
    Meter { label: String, level: f32 },
    /// A player in full: art, track, scrubber and transport.
    Media(MediaCard),
    /// Rows folded under a name until opened: "Other networks".
    Group {
        id: String,
        label: String,
        open: bool,
        rows: Vec<PageRow>,
    },
}

/// What stands before a row's label.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Lead {
    #[default]
    None,
    Glyph(String),
    /// An app's icon file.
    Icon(PathBuf),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Choice {
    pub id: String,
    pub label: String,
    /// A second line: a device's state, a network's security.
    pub detail: Option<String>,
    pub lead: Lead,
    pub checked: bool,
    /// Work under way on it: connecting, pairing.
    pub busy: bool,
    /// Why the last action on it failed, shown in its place.
    pub error: Option<String>,
    /// A glyph at the trailing end: a network's signal, a lock.
    pub trailing: Option<String>,
    pub battery: Option<u8>,
    /// It opens a page of its own.
    pub sub_page: bool,
    pub enabled: bool,
}

/// What a player shows and offers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaCard {
    pub player: String,
    pub title: String,
    pub artist: String,
    pub art: Option<PathBuf>,
    pub playing: bool,
    pub can_previous: bool,
    pub can_next: bool,
    /// Where in the track, 0 to 1, when it can seek.
    pub position: Option<f32>,
    pub elapsed: String,
    pub remaining: String,
    pub shuffle: Option<bool>,
    /// `None`, `Track` or `Playlist`.
    pub repeat: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SliderRow {
    pub id: String,
    pub label: String,
    pub lead: Lead,
    pub value: f32,
    pub muted: bool,
    pub enabled: bool,
}

/// What a row asks of the compositor.
#[derive(Clone, Debug, PartialEq)]
pub enum RowEvent {
    Choose(String),
    Open(String),
    Slide(String, f32),
    Switch(String, bool),
    Expand(String),
}

/// A row's height in points.
pub fn row_height(theme: &Theme, row: &PageRow) -> f32 {
    let t = &theme.tokens;
    match row {
        PageRow::Heading(_)
        | PageRow::Switch { .. }
        | PageRow::Status(_)
        | PageRow::Meter { .. } => t.size_row,
        PageRow::Choice(c) if c.detail.is_some() || c.error.is_some() => t.size_row_large,
        PageRow::Choice(_) => t.size_row,
        PageRow::Slider(_) => t.size_row_large,
        PageRow::Media(_) => media_height(theme),
        PageRow::Group { open, rows, .. } => {
            t.size_row + if *open { rows_height(theme, rows) } else { 0.0 }
        }
    }
}

/// A player card's height: its art beside the track, the scrubber, the transport.
pub fn media_height(theme: &Theme) -> f32 {
    let t = &theme.tokens;
    t.space_m + t.size_media_art + t.space_s + t.size_control + t.size_control_large + t.space_m
}

/// The height of `rows` laid one under another.
pub fn rows_height(theme: &Theme, rows: &[PageRow]) -> f32 {
    rows.iter().map(|r| row_height(theme, r)).sum()
}

/// The rows of a page.
#[derive(Clone, PartialEq)]
pub struct Rows {
    pub rows: Vec<PageRow>,
    pub on_event: EventHandler<RowEvent>,
}

impl Component for Rows {
    fn render(&self) -> impl IntoElement {
        rect()
            .width(Size::fill())
            .children(self.rows.iter().map(|row| row_element(row, &self.on_event)))
    }
}

fn row_element(row: &PageRow, on_event: &EventHandler<RowEvent>) -> Element {
    match row {
        PageRow::Heading(label) => Heading {
            label: label.clone(),
        }
        .into_element(),
        PageRow::Status(line) => Status { line: line.clone() }.into_element(),
        PageRow::Media(card) => MediaView {
            card: card.clone(),
            on_event: on_event.clone(),
        }
        .into_element(),
        PageRow::Meter { label, level } => Meter {
            label: label.clone(),
            level: *level,
        }
        .into_element(),
        PageRow::Choice(choice) => ChoiceRow {
            choice: choice.clone(),
            on_event: on_event.clone(),
        }
        .into_element(),
        PageRow::Slider(slider) => SliderLine {
            row: slider.clone(),
            on_event: on_event.clone(),
        }
        .into_element(),
        PageRow::Switch {
            id,
            label,
            on,
            enabled,
        } => SwitchLine {
            id: id.clone(),
            label: label.clone(),
            on: *on,
            enabled: *enabled,
            on_event: on_event.clone(),
        }
        .into_element(),
        PageRow::Group {
            id,
            label,
            open,
            rows,
        } => {
            let header = ChoiceRow {
                choice: Choice {
                    id: id.clone(),
                    label: label.clone(),
                    trailing: Some(if *open { "caret-down" } else { "caret-right" }.into()),
                    enabled: true,
                    ..Default::default()
                },
                on_event: {
                    let on_event = on_event.clone();
                    let id = id.clone();
                    (move |_| on_event.call(RowEvent::Expand(id.clone()))).into()
                },
            };
            rect()
                .width(Size::fill())
                .child(header)
                .maybe(*open, |el| {
                    el.children(rows.iter().map(|r| row_element(r, on_event)))
                })
                .into_element()
        }
    }
}

fn lead(theme: &Theme, lead: &Lead) -> Option<Element> {
    let t = &theme.tokens;
    match lead {
        Lead::None => None,
        Lead::Glyph(name) => Some(
            rect()
                .width(Size::px(t.size_app_icon_row))
                .center()
                .child(Glyph::new(
                    name.clone(),
                    t.size_glyph,
                    colour(t.colour_text),
                ))
                .into_element(),
        ),
        Lead::Icon(path) => Some(AppIcon::new(path.clone(), t.size_app_icon_row).into_element()),
    }
}

#[derive(PartialEq)]
struct Heading {
    label: String,
}

impl Component for Heading {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_row))
            .padding((0., t.space_m))
            .main_align(Alignment::End)
            .a11y_role(AccessibilityRole::Heading)
            .a11y_alt(self.label.clone())
            .child(text::line(
                &theme,
                text::Style::Subheadline,
                colour(t.colour_text_2),
                self.label.clone(),
            ))
    }
}

#[derive(PartialEq)]
struct Status {
    line: String,
}

impl Component for Status {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_row))
            .padding((0., t.space_m))
            .main_align(Alignment::Center)
            .child(text::line(
                &theme,
                text::Style::Body,
                colour(t.colour_text_2),
                self.line.clone(),
            ))
    }
}

/// The transport's buttons, which a row's events name: `previous`, `play-pause`, `next`.
pub fn transport(card: &MediaCard, on_event: &EventHandler<RowEvent>) -> Element {
    let button = |glyph: &str, alt: &str, id: &'static str, enabled: bool| {
        let on_event = on_event.clone();
        isle_ui::Button::icon(glyph, alt)
            .kind(isle_ui::button::Kind::Plain)
            .enabled(enabled)
            .on_press(move |_| on_event.call(RowEvent::Choose(id.into())))
            .into_element()
    };
    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(4.)
        .child(button("skip-back", "Previous track", "previous", card.can_previous))
        .child(if card.playing {
            button("pause", "Pause", "play-pause", true)
        } else {
            button("play", "Play", "play-pause", true)
        })
        .child(button("skip-forward", "Next track", "next", card.can_next))
        .into_element()
}

#[derive(PartialEq)]
struct MediaView {
    card: MediaCard,
    on_event: EventHandler<RowEvent>,
}

impl Component for MediaView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let c = &self.card;
        let on_event = self.on_event.clone();
        let raise = on_event.clone();
        let toggle = |id: &'static str, glyph: &str, alt: &str, on: bool| {
            let on_event = on_event.clone();
            isle_ui::Button::icon(glyph, alt)
                .kind(isle_ui::button::Kind::Plain)
                .on_press(move |_| on_event.call(RowEvent::Choose(id.into())))
                .into_element()
                .maybe_with(on, t)
        };
        rect()
            .width(Size::fill())
            .height(Size::px(media_height(&theme)))
            .padding(t.space_m)
            .spacing(t.space_s)
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_m)
                    .child(
                        rect()
                            .width(Size::px(t.size_media_art))
                            .height(Size::px(t.size_media_art))
                            .corner_radius(t.radius_control)
                            .overflow(Overflow::Clip)
                            .background(colour(t.colour_fill_strong))
                            .center()
                            .map(c.art.clone(), |el, art| {
                                el.child(AppIcon::new(art, t.size_media_art))
                            })
                            .maybe(c.art.is_none(), |el| {
                                el.child(Glyph::new(
                                    "music-note",
                                    t.size_glyph_large,
                                    colour(t.colour_text_2),
                                ))
                            }),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .child(text::line(
                                &theme,
                                text::Style::Headline,
                                colour(t.colour_text),
                                c.title.clone(),
                            ))
                            .child(text::line(
                                &theme,
                                text::Style::Body,
                                colour(t.colour_text_2),
                                c.artist.clone(),
                            ))
                            .child(
                                rect()
                                    .on_press(move |_| raise.call(RowEvent::Choose("raise".into())))
                                    .a11y_role(AccessibilityRole::Link)
                                    .a11y_focusable(true)
                                    .child(text::line(
                                        &theme,
                                        text::Style::Subheadline,
                                        colour(t.colour_text_2),
                                        c.player.clone(),
                                    )),
                            ),
                    ),
            )
            .child(
                rect()
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .spacing(t.space_s)
                    .child(text::line(
                        &theme,
                        text::Style::Footnote,
                        colour(t.colour_text_2),
                        c.elapsed.clone(),
                    ))
                    .child(rect().width(Size::flex(1.)).map(c.position, |el, position| {
                        let on_event = on_event.clone();
                        el.child(
                            Slider::new(position, "Position", move |v| {
                                on_event.call(RowEvent::Slide("position".into(), v))
                            })
                            .glass(true)
                            .scrubber(),
                        )
                    }))
                    .child(text::line(
                        &theme,
                        text::Style::Footnote,
                        colour(t.colour_text_2),
                        c.remaining.clone(),
                    )),
            )
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .maybe(c.shuffle.is_some(), |el| {
                        el.child(toggle(
                            "shuffle",
                            "shuffle",
                            if c.shuffle == Some(true) { "Shuffle, on" } else { "Shuffle, off" },
                            c.shuffle == Some(true),
                        ))
                    })
                    .child(rect().width(Size::flex(1.)))
                    .child(transport(c, &self.on_event))
                    .child(rect().width(Size::flex(1.)))
                    .maybe(c.repeat.is_some(), |el| {
                        let mode = c.repeat.clone().unwrap_or_default();
                        el.child(toggle(
                            "repeat",
                            if mode == "Track" { "repeat-once" } else { "repeat" },
                            match mode.as_str() {
                                "Track" => "Repeat, one",
                                "Playlist" => "Repeat, all",
                                _ => "Repeat, off",
                            },
                            mode != "None",
                        ))
                    }),
            )
    }
}

trait Toggled {
    fn maybe_with(self, on: bool, t: &isle_tokens::Tokens) -> Element;
}

impl Toggled for Element {
    /// A toggle that is on sits in a filled well.
    fn maybe_with(self, on: bool, t: &isle_tokens::Tokens) -> Element {
        rect()
            .corner_radius(t.radius_control)
            .background(if on {
                colour(t.colour_fill_strong)
            } else {
                Color::TRANSPARENT
            })
            .child(self)
            .into_element()
    }
}

#[derive(PartialEq)]
struct Meter {
    label: String,
    level: f32,
}

impl Component for Meter {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_row))
            .padding((0., t.space_m))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .child(rect().width(Size::px(t.size_slider_label)).child(text::line(
                &theme,
                text::Style::Body,
                colour(t.colour_text_2),
                self.label.clone(),
            )))
            .child(rect().width(Size::flex(1.)).a11y_alt(self.label.clone()).child(
                isle_ui::LevelBar {
                    level: self.level,
                    track: colour(t.colour_fill_strong),
                    fill: colour(t.colour_text),
                },
            ))
    }
}

#[derive(PartialEq)]
struct ChoiceRow {
    choice: Choice,
    on_event: EventHandler<RowEvent>,
}

impl Component for ChoiceRow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let mut hover = use_state(|| false);
        let c = &self.choice;
        let height = row_height(&theme, &PageRow::Choice(c.clone()));
        let event = if c.sub_page {
            RowEvent::Open(c.id.clone())
        } else {
            RowEvent::Choose(c.id.clone())
        };
        let on_event = self.on_event.clone();
        let enabled = c.enabled;
        let ink = if enabled {
            colour(t.colour_text)
        } else {
            colour(t.colour_text_disabled)
        };
        let second = c.error.clone().or(c.detail.clone());
        let mut alt = c.label.clone();
        if let Some(second) = &second {
            alt = format!("{alt}, {second}");
        }
        if let Some(level) = c.battery {
            alt = format!("{alt}, battery {level}%");
        }
        rect()
            .width(Size::fill())
            .height(Size::px(height))
            .padding((0., t.space_m))
            .corner_radius(t.radius_row)
            .background(if *hover.read() && enabled {
                colour(t.colour_fill_strong)
            } else {
                Color::TRANSPARENT
            })
            .a11y_id(a11y)
            .a11y_focusable(enabled)
            .a11y_role(if c.sub_page {
                AccessibilityRole::Button
            } else {
                AccessibilityRole::MenuItemRadio
            })
            .a11y_alt(alt)
            .maybe(focus() == Focus::Keyboard, |el| {
                el.border(focus_ring(&theme))
            })
            .on_pointer_enter(move |_| hover.set_if_modified(true))
            .on_pointer_leave(move |_| hover.set_if_modified(false))
            .on_press(move |e: Event<PressEventData>| {
                e.stop_propagation();
                if enabled {
                    on_event.call(event.clone());
                }
            })
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .maybe_child(lead(&theme, &c.lead))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(text::line(&theme, text::Style::Body, ink, c.label.clone()))
                    .map(second, |el, line| {
                        let tone = if c.error.is_some() {
                            t.colour_text
                        } else {
                            t.colour_text_2
                        };
                        el.child(text::line(
                            &theme,
                            text::Style::Subheadline,
                            colour(tone),
                            line,
                        ))
                    }),
            )
            .map(c.battery, |el, level| {
                el.child(text::line(
                    &theme,
                    text::Style::Subheadline,
                    colour(t.colour_text_2),
                    format!("{level}%"),
                ))
            })
            .maybe(c.busy, |el| el.child(Spinner::new(t.size_glyph, "Working")))
            .map(c.trailing.clone(), |el, glyph| {
                el.child(Glyph::new(
                    glyph,
                    t.size_glyph_small,
                    colour(t.colour_text_2),
                ))
            })
            .maybe(c.checked, |el| {
                el.child(Glyph::new("check", t.size_glyph_small, ink))
            })
            .maybe(c.sub_page, |el| {
                el.child(Glyph::new(
                    "caret-right",
                    t.size_glyph_small,
                    colour(t.colour_text_2),
                ))
            })
    }
}

#[derive(PartialEq)]
struct SliderLine {
    row: SliderRow,
    on_event: EventHandler<RowEvent>,
}

impl Component for SliderLine {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let r = &self.row;
        let on_event = self.on_event.clone();
        let id = r.id.clone();
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_row_large))
            .padding((0., t.space_m))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .maybe_child(lead(&theme, &r.lead))
            .child(
                rect()
                    .width(Size::px(t.size_slider_label))
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(t.colour_text),
                        r.label.clone(),
                    )),
            )
            .child(
                rect().width(Size::flex(1.)).child(
                    Slider::new(r.value, r.label.clone(), move |v| {
                        on_event.call(RowEvent::Slide(id.clone(), v))
                    })
                    .glass(true)
                    .muted(r.muted)
                    .enabled(r.enabled),
                ),
            )
    }
}

#[derive(PartialEq)]
struct SwitchLine {
    id: String,
    label: String,
    on: bool,
    enabled: bool,
    on_event: EventHandler<RowEvent>,
}

impl Component for SwitchLine {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let on_event = self.on_event.clone();
        let id = self.id.clone();
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_row))
            .padding((0., t.space_m))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .child(rect().width(Size::flex(1.)).child(text::line(
                &theme,
                text::Style::Body,
                colour(t.colour_text),
                self.label.clone(),
            )))
            .child(
                Switch::new(self.on, self.label.clone(), move |on| {
                    on_event.call(RowEvent::Switch(id.clone(), on))
                })
                .enabled(self.enabled),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> Theme {
        isle_tokens::Source::shipped()
            .theme(&isle_tokens::Appearance::default())
            .unwrap()
    }

    fn choice(detail: bool) -> PageRow {
        PageRow::Choice(Choice {
            id: "a".into(),
            label: "A".into(),
            detail: detail.then(|| "Connected".into()),
            ..Default::default()
        })
    }

    #[test]
    fn rows_take_their_kinds_heights() {
        let theme = theme();
        let t = &theme.tokens;
        assert_eq!(row_height(&theme, &choice(false)), t.size_row);
        assert_eq!(row_height(&theme, &choice(true)), t.size_row_large);
        assert_eq!(
            row_height(&theme, &PageRow::Slider(SliderRow::default())),
            t.size_row_large
        );
        assert_eq!(
            row_height(&theme, &PageRow::Heading("Output".into())),
            t.size_row
        );
    }

    #[test]
    fn a_folded_group_is_one_row_and_an_open_one_holds_its_rows() {
        let theme = theme();
        let t = &theme.tokens;
        let group = |open| PageRow::Group {
            id: "other".into(),
            label: "Other networks".into(),
            open,
            rows: vec![choice(false), choice(true)],
        };
        assert_eq!(row_height(&theme, &group(false)), t.size_row);
        assert_eq!(
            row_height(&theme, &group(true)),
            2.0 * t.size_row + t.size_row_large
        );
        assert_eq!(
            rows_height(&theme, &[choice(false), group(true)]),
            3.0 * t.size_row + t.size_row_large
        );
    }
}
