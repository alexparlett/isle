//! Settings' window: the sidebar with search, the account row and the panes; the toolbar
//! with back, forward and the title; the content with the rejected-edit notice and the pane
//! (`p1/structure/settings.md`, The window; `p1/surfaces/settings.md`, Anatomy).

use freya::prelude::*;
use isle_app::{Chord, Command, Desktop, Menu, Request, ToolbarExt, use_commands, worker};
use isle_ui::{Glyph, TextField, colour, form, text, use_theme};
use serde::{Deserialize, Serialize};

use crate::{
    account::{self, Account},
    nav::{self, History, Location},
    panes::{self, PANES, Pane},
};

/// What Settings remembers between launches: the pane last shown (D62).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Memory {
    pub pane: Option<String>,
}

impl isle_state::Domain for Memory {
    const NAME: &'static str = "settings-app";
    const VERSION: u32 = 1;
}

/// Where the memory is kept: the state directory, or a test's own.
#[derive(Clone)]
pub struct MemoryDir(pub std::path::PathBuf);

fn memory_dir() -> std::path::PathBuf {
    try_consume_context::<MemoryDir>()
        .map(|d| d.0)
        .unwrap_or_else(isle_state::StateDir::default_dir)
}

fn remember(dir: &std::path::Path, pane: &str) {
    let dir = isle_state::StateDir::new(dir);
    let memory = Memory {
        pane: Some(pane.to_owned()),
    };
    if let Err(err) = dir.save(&memory) {
        tracing::warn!("the pane shown is not kept: {err}");
    }
}

fn first_location(dir: &std::path::Path) -> Location {
    let dir = isle_state::StateDir::new(dir);
    let (memory, _) = dir.load::<Memory>();
    memory
        .pane
        .and_then(|p| Location::at(&p, None))
        .unwrap_or_else(|| Location::pane(panes::FIRST))
}

/// Where a launch or an activation asks to go, if anywhere.
fn asked(request: &Request) -> Option<Location> {
    match request {
        Request::Open { uris, .. } => uris.iter().find_map(|u| Location::from_uri(u)),
        Request::Action { name, params, .. } if name == "open" => {
            Location::at(params.first()?, params.get(1).map(String::as_str))
        }
        _ => None,
    }
}

/// Settings' state that every part of the window reads.
#[derive(Clone, Copy)]
struct Nav {
    location: State<Location>,
    history: State<History>,
    memory: State<std::path::PathBuf>,
}

impl Nav {
    fn go(&self, to: Location) {
        let (mut location, mut history) = (self.location, self.history);
        let from = location.peek().clone();
        if from == to {
            return;
        }
        history.write().left(Location { row: None, ..from });
        if to.pane != location.peek().pane {
            remember(&self.memory.peek(), &to.pane);
        }
        location.set(to);
    }

    fn back(&self) {
        let (mut location, mut history) = (self.location, self.history);
        let current = location.peek().clone();
        if let Some(to) = history.write().back(current) {
            location.set(to);
        }
    }

    fn forward(&self) {
        let (mut location, mut history) = (self.location, self.history);
        let current = location.peek().clone();
        if let Some(to) = history.write().forward(current) {
            location.set(to);
        }
    }
}

#[derive(PartialEq)]
pub struct Frame;

impl Component for Frame {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let desktop = Desktop::get();
        let nav = use_provide_context(|| {
            let memory = memory_dir();
            Nav {
                location: State::create(first_location(&memory)),
                history: State::create(History::default()),
                memory: State::create(memory),
            }
        });
        let query = use_state(String::new);
        let mut account = use_state(Account::default);
        let worker = worker();
        use_hook(move || {
            let job = worker.spawn(account::mine());
            spawn(async move {
                if let Ok(mine) = job.await {
                    account.set(mine);
                }
            })
        });

        // Launches and activations that name a pane.
        use_side_effect(move || {
            let pending: Vec<Request> = desktop.requests.read().clone();
            if pending.is_empty() {
                return;
            }
            let mut requests = desktop.requests;
            requests.write().clear();
            if let Some(to) = pending.iter().rev().find_map(asked) {
                nav.go(to);
            }
        });

        // The app's own commands: back, forward, find, and every pane in View.
        let history = nav.history.read().clone();
        let mut commands = vec![
            Command::new("back", "Back", Menu::View, 0)
                .chord(Chord::cmd('['))
                .enabled(history.can_back()),
            Command::new("forward", "Forward", Menu::View, 0)
                .chord(Chord::cmd(']'))
                .enabled(history.can_forward()),
            Command::new("find", "Find", Menu::Edit, 2).chord(Chord::cmd('f')),
        ];
        commands.extend(
            PANES
                .iter()
                .map(|p| Command::new(format!("pane:{}", p.id), p.name, Menu::View, 1 + p.group)),
        );
        use_commands(commands);
        use_side_effect(move || {
            let chosen: Vec<String> = desktop.commands.read().clone();
            if chosen.is_empty() {
                return;
            }
            let mut queue = desktop.commands;
            queue.write().clear();
            for id in chosen {
                match id.as_str() {
                    "back" => nav.back(),
                    "forward" => nav.forward(),
                    other => {
                        if let Some(pane) = other.strip_prefix("pane:") {
                            nav.go(Location::pane(pane));
                        }
                    }
                }
            }
        });

        let location = nav.location.read().clone();
        let pane = panes::find(&location.pane).unwrap_or(&PANES[0]);
        rect()
            .expanded()
            .horizontal()
            .content(Content::flex())
            .background(colour(t.colour_window))
            .on_mouse_down(move |e: Event<MouseEventData>| match e.button {
                Some(MouseButton::Back) => nav.back(),
                Some(MouseButton::Forward) => nav.forward(),
                _ => {}
            })
            .child(Sidebar {
                query,
                account: account.read().clone(),
                shown: pane.id,
            })
            .child(PaneContent { pane, location })
    }
}

#[derive(PartialEq)]
struct Sidebar {
    query: State<String>,
    account: Account,
    shown: &'static str,
}

impl Component for Sidebar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let nav = use_consume::<Nav>();
        let query = self.query.read().clone();
        let results = nav::search(&query);
        let searching = !query.trim().is_empty();
        let list: Element = if searching {
            results_list(&theme, &query, results, nav).into_element()
        } else {
            pane_list(&theme, self.shown, nav).into_element()
        };
        rect()
            .width(Size::px(t.size_sidebar))
            .height(Size::fill())
            .background(colour(t.colour_sidebar))
            .child(rect().width(Size::fill()).height(Size::px(t.size_toolbar)).toolbar())
            .child(
                rect()
                    .width(Size::fill())
                    .padding((0., t.space_s))
                    .child(
                        TextField::new(self.query, "Search")
                            .placeholder("Search")
                            .leading(Glyph::new("magnifying-glass", t.size_glyph, colour(t.colour_text_2))),
                    ),
            )
            .child(
                isle_ui::scroll_area(&theme)
                    .width(Size::fill())
                    .height(Size::fill())
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((t.space_m, t.space_s, t.space_m, t.space_s))
                            .spacing(t.space_m)
                            .maybe(!searching, |el| el.child(account_row(&theme, &self.account)))
                            .child(list),
                    ),
            )
    }
}

fn account_row(theme: &isle_ui::isle_tokens::Theme, account: &Account) -> impl IntoElement {
    let t = &theme.tokens;
    let nav = use_consume::<Nav>();
    let size = t.size_app_icon;
    rect()
        .width(Size::fill())
        .height(Size::px(48.))
        .padding(t.space_s)
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .corner_radius(t.radius_row)
        .a11y_role(AccessibilityRole::Button)
        .a11y_alt(format!("{}, {}", account.name, account.login))
        .on_press(move |_| nav.go(Location::pane("general")))
        .child(
            rect()
                .width(Size::px(size))
                .height(Size::px(size))
                .corner_radius(size / 2.)
                .background(colour(t.colour_fill_strong))
                .center()
                .child(text::line(theme, text::Style::Headline, colour(t.colour_text), account.initials())),
        )
        .child(
            rect()
                .child(text::line(
                    theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    if account.name.is_empty() { account.login.clone() } else { account.name.clone() },
                ))
                .maybe(!account.name.is_empty(), |el| {
                    el.child(text::line(
                        theme,
                        text::Style::Subheadline,
                        colour(t.colour_text_2),
                        account.login.clone(),
                    ))
                }),
        )
}

fn sidebar_row(
    theme: &isle_ui::isle_tokens::Theme,
    pane: &'static Pane,
    title: String,
    path: Option<String>,
    selected: bool,
    on_press: impl Fn() + 'static,
) -> Element {
    let t = &theme.tokens;
    let ink = if selected { t.colour_on_accent } else { t.colour_text };
    rect()
        .key(format!("{}:{title}", pane.id))
        .width(Size::fill())
        .min_height(Size::px(if path.is_some() { t.size_row_large } else { t.size_row }))
        .padding((0., t.space_s))
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .corner_radius(t.radius_row)
        .maybe(selected, |el| el.background(colour(t.colour_accent)))
        .a11y_role(AccessibilityRole::ListItem)
        .a11y_alt(title.clone())
        .on_press(move |_| on_press())
        .child(
            rect()
                .width(Size::px(t.size_pane_tile))
                .height(Size::px(t.size_pane_tile))
                .corner_radius(t.radius_row)
                .background(colour(pane.hue.colour(t)))
                .center()
                .child(Glyph::new(pane.glyph, t.size_glyph, Color::WHITE).fill(true)),
        )
        .child(
            rect()
                .child(text::line(theme, text::Style::Body, colour(ink), title))
                .map(path, |el, path| {
                    el.child(text::line(
                        theme,
                        text::Style::Subheadline,
                        colour(if selected { t.colour_on_accent } else { t.colour_text_2 }),
                        path,
                    ))
                }),
        )
        .into_element()
}

fn pane_list(theme: &isle_ui::isle_tokens::Theme, shown: &'static str, nav: Nav) -> impl IntoElement {
    let t = &theme.tokens;
    let mut groups: Vec<Vec<Element>> = Vec::new();
    let mut last = None;
    for pane in PANES {
        if last != Some(pane.group) {
            groups.push(Vec::new());
            last = Some(pane.group);
        }
        groups.last_mut().unwrap().push(sidebar_row(
            theme,
            pane,
            pane.name.to_owned(),
            None,
            pane.id == shown,
            move || nav.go(Location::pane(pane.id)),
        ));
    }
    rect()
        .width(Size::fill())
        .spacing(t.space_m)
        .a11y_role(AccessibilityRole::List)
        .on_key_down(move |e: Event<KeyboardEventData>| {
            let at = PANES.iter().position(|p| p.id == shown).unwrap_or(0);
            let to = match e.key {
                Key::Named(NamedKey::ArrowUp) => at.checked_sub(1),
                Key::Named(NamedKey::ArrowDown) => Some(at + 1).filter(|n| *n < PANES.len()),
                _ => None,
            };
            if let Some(to) = to {
                nav.go(Location::pane(PANES[to].id));
            }
        })
        .children(groups.into_iter().map(|rows| rect().width(Size::fill()).children(rows).into_element()))
}

fn results_list(
    theme: &isle_ui::isle_tokens::Theme,
    query: &str,
    results: Vec<nav::Found>,
    nav: Nav,
) -> impl IntoElement {
    let t = &theme.tokens;
    if results.is_empty() {
        return rect()
            .width(Size::fill())
            .padding((t.space_xxl, 0., 0., 0.))
            .cross_align(Alignment::Center)
            .child(text::line(
                theme,
                text::Style::Body,
                colour(t.colour_text_2),
                format!("No results for “{}”", query.trim()),
            ));
    }
    let shown = nav.location.read().clone();
    rect()
        .width(Size::fill())
        .a11y_role(AccessibilityRole::List)
        .children(results.into_iter().map(|found| {
            let to = found.location();
            let selected = to == shown;
            let path = Some(found.path()).filter(|p| !p.is_empty()).or(Some("Settings".into()));
            sidebar_row(theme, found.pane, found.label().to_owned(), path, selected, move || {
                nav.go(to.clone())
            })
        }))
}

#[derive(PartialEq)]
struct PaneContent {
    pane: &'static Pane,
    location: Location,
}

impl Component for PaneContent {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let nav = use_consume::<Nav>();
        let desktop = Desktop::get();
        let history = nav.history.read().clone();
        let status = desktop.status.read().clone().filter(|s| !s.ok);
        let tokens = desktop
            .tokens_status
            .read()
            .clone()
            .filter(|s| !s.ok && matches!(self.pane.id, "appearance" | "accessibility"));
        let notice = status.map(|s| (s, "settings.kdl", " Changes won't be saved until it's fixed."))
            .or(tokens.map(|s| (s, "tokens.kdl", " The last good look is in use.")))
            .map(|(s, file, consequence)| form::Notice {
                message: format!(
                    "{file} has an error on line {}, column {}.{consequence}",
                    s.line, s.column
                ),
                detail: s.message.clone(),
                actions: Vec::new(),
            });
        let arrow = |glyph: &'static str, alt: &'static str, enabled: bool, go: fn(&Nav)| {
            rect()
                .width(Size::px(t.size_control))
                .height(Size::px(t.size_control))
                .center()
                .corner_radius(t.radius_control)
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(alt)
                .on_press(move |e: Event<PressEventData>| {
                    e.stop_propagation();
                    if enabled {
                        go(&nav);
                    }
                })
                .child(Glyph::new(
                    glyph,
                    t.size_glyph,
                    colour(if enabled { t.colour_text } else { t.colour_text_disabled }),
                ))
        };
        rect()
            .width(Size::flex(1.))
            .height(Size::fill())
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_toolbar))
                    .padding((0., t.space_l))
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_xs)
                    .toolbar()
                    .child(arrow("caret-left", "Back", history.can_back(), Nav::back))
                    .child(arrow("caret-right", "Forward", history.can_forward(), Nav::forward))
                    .child(
                        rect()
                            .padding((0., 0., 0., t.space_s))
                            .child(text::line(&theme, text::Style::Headline, colour(t.colour_text), self.pane.name)),
                    ),
            )
            .child(
                isle_ui::scroll_area(&theme)
                    .width(Size::fill())
                    .height(Size::fill())
                    .child(
                        rect()
                            .width(Size::fill())
                            .cross_align(Alignment::Center)
                            .child(
                                rect()
                                    .width(Size::px(t.size_form))
                                    .padding((t.space_xl, 0., t.space_xxxl, 0.))
                                    .spacing(t.space_xl)
                                    .map(notice, |el, notice| el.child(notice))
                                    .child(form::PaneHeader {
                                        glyph: self.pane.glyph.into(),
                                        hue: self.pane.hue.colour(t),
                                        name: self.pane.name.into(),
                                        subtitle: self.pane.subtitle.into(),
                                    })
                                    .child(crate::pane_body(self.pane.id, &self.location)),
                            ),
                    ),
            )
    }
}
