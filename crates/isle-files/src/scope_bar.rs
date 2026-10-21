//! The scope bar and its criteria rows while a search stands (`p1/surfaces/files.md` § Search).

use freya::prelude::*;
use isle_ui::{Glyph, colour, text, use_theme};

use crate::search::{Criterion, KINDS, KindIs, Scope, Search, WITHIN, Within};

/// Where a search looks, as the scope bar offers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Where {
    /// The folder shown, filtered as typed.
    ThisFolder,
    Search(Scope),
}

#[derive(PartialEq)]
pub struct ScopeBar {
    /// The walk standing, if one is.
    pub search: Option<Search>,
    /// A place to filter: the folder, or the gathering, shown or searched from.
    pub this_folder: bool,
    /// The folder "Everything under" names; none from a gathering.
    pub folder: Option<String>,
    pub on_where: EventHandler<Where>,
    pub on_criteria: EventHandler<Vec<Criterion>>,
    pub on_save: EventHandler<()>,
}

const CRITERIA: [&str; 3] = ["Kind", "Last modified", "Contents contain"];

fn default_of(i: usize) -> Criterion {
    match i {
        0 => Criterion::Kind(KindIs::Any),
        1 => Criterion::Modified(Within::Week),
        _ => Criterion::Contents(String::new()),
    }
}

fn index_of(c: &Criterion) -> usize {
    match c {
        Criterion::Kind(_) => 0,
        Criterion::Modified(_) => 1,
        Criterion::Contents(_) => 2,
    }
}

impl Component for ScopeBar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut options = Vec::new();
        let mut wheres = Vec::new();
        if self.this_folder {
            options.push("This folder".to_owned());
            wheres.push(Where::ThisFolder);
        }
        if let Some(folder) = &self.folder {
            options.push(format!("Everything under “{folder}”"));
            wheres.push(Where::Search(Scope::Under));
        }
        options.push("Home".to_owned());
        wheres.push(Where::Search(Scope::Home));
        let now = match &self.search {
            None => Where::ThisFolder,
            Some(s) => Where::Search(s.scope),
        };
        let selected = wheres.iter().position(|w| *w == now).unwrap_or(0);
        let criteria = self.search.as_ref().map(|s| s.criteria.clone()).unwrap_or_default();
        let on_where = self.on_where.clone();
        let picks = wheres.clone();
        let adding = {
            let (on_criteria, criteria) = (self.on_criteria.clone(), criteria.clone());
            move || {
                let mut next = criteria.clone();
                next.push(default_of(0));
                on_criteria.call(next);
            }
        };
        let bar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_row + t.space_s))
            .padding((0., t.space_l))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .background(colour(t.colour_window))
            .border(hairline_under(t))
            .a11y_alt("Search scope")
            .child(text::line(&theme, text::Style::Body, colour(t.colour_text_2), "Search:"))
            .child(isle_ui::form::Segmented::new(options, selected, "Search scope", move |i: usize| {
                on_where.call(picks[i])
            }))
            .child(rect().width(Size::flex(1.)))
            .child(
                isle_ui::Button::icon("plus", "Add a criterion")
                    .small()
                    .on_press(move |_| adding())
                    .into_element(),
            )
            .child({
                let on_save = self.on_save.clone();
                isle_ui::Button::new("Save")
                    .small()
                    .enabled(self.search.is_some())
                    .on_press(move |_| on_save.call(()))
                    .into_element()
            });
        let rows = criteria.iter().enumerate().map(|(n, c)| {
            criterion_row(&theme, n, c, &criteria, self.on_criteria.clone())
        });
        rect().width(Size::fill()).child(bar).children(rows)
    }
}

fn hairline_under(t: &isle_tokens::Tokens) -> Border {
    Border::new()
        .fill(colour(t.colour_hairline))
        .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
        .alignment(BorderAlignment::Inner)
}

/// A criterion: what it asks, its value, and − and + at its end.
fn criterion_row(
    theme: &isle_tokens::Theme,
    n: usize,
    criterion: &Criterion,
    all: &[Criterion],
    on_criteria: EventHandler<Vec<Criterion>>,
) -> Element {
    let t = &theme.tokens;
    let change = {
        let (all, on_criteria) = (all.to_vec(), on_criteria.clone());
        move |c: Criterion| {
            let mut next = all.clone();
            next[n] = c;
            on_criteria.call(next);
        }
    };
    let what = {
        let change = change.clone();
        isle_ui::form::PopUp::new(
            CRITERIA.iter().map(|s| s.to_string()).collect(),
            index_of(criterion),
            "Criterion",
            move |i: usize| change(default_of(i)),
        )
        .into_element()
    };
    let value = match criterion {
        Criterion::Kind(kind) => {
            let change = change.clone();
            isle_ui::form::PopUp::new(
                KINDS.iter().map(|(_, s)| s.to_string()).collect(),
                KINDS.iter().position(|(k, _)| k == kind).unwrap_or(0),
                "Kind",
                move |i: usize| change(Criterion::Kind(KINDS[i].0)),
            )
            .into_element()
        }
        Criterion::Modified(within) => {
            let change = change.clone();
            isle_ui::form::PopUp::new(
                WITHIN.iter().map(|(_, s)| s.to_string()).collect(),
                WITHIN.iter().position(|(w, _)| w == within).unwrap_or(0),
                "Last modified",
                move |i: usize| change(Criterion::Modified(WITHIN[i].0)),
            )
            .into_element()
        }
        Criterion::Contents(text) => ContentsField {
            text: text.clone(),
            on_change: EventHandler::new({
                let change = change.clone();
                move |text: String| change(Criterion::Contents(text))
            }),
        }
        .into_element(),
    };
    let remove = {
        let (all, on_criteria) = (all.to_vec(), on_criteria.clone());
        move || {
            let mut next = all.clone();
            next.remove(n);
            on_criteria.call(next);
        }
    };
    let add = {
        let (all, on_criteria) = (all.to_vec(), on_criteria);
        move || {
            let mut next = all.clone();
            next.insert(n + 1, default_of(0));
            on_criteria.call(next);
        }
    };
    let mark = |glyph: &'static str, alt: &'static str, on_press: Box<dyn Fn()>| {
        rect()
            .width(Size::px(t.size_control_small))
            .height(Size::px(t.size_control_small))
            .center()
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(alt)
            .on_press(move |_| on_press())
            .child(Glyph::new(glyph, 16., colour(t.colour_text_2)))
            .into_element()
    };
    rect()
        .key(n)
        .width(Size::fill())
        .height(Size::px(t.size_row_large))
        .padding((0., t.space_l))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .background(colour(t.colour_window))
        .border(hairline_under(t))
        .child(what)
        .child(value)
        .child(rect().width(Size::flex(1.)))
        .child(mark("minus-circle", "Remove the criterion", Box::new(remove)))
        .child(mark("plus-circle", "Add a criterion", Box::new(add)))
        .into_element()
}

/// Contents contain's field: the search runs again on Return.
#[derive(PartialEq)]
struct ContentsField {
    text: String,
    on_change: EventHandler<String>,
}

impl Component for ContentsField {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let typed = use_state({
            let text = self.text.clone();
            move || text
        });
        let on_change = self.on_change.clone();
        isle_ui::TextField::new(typed, "Contents contain")
            .width(Size::px(theme.tokens.size_row_field))
            .on_submit(move |text: String| on_change.call(text))
    }
}
