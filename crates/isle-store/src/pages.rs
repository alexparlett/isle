//! Store's pages to pick new apps (`p14e-store.md` choices 4–11): Discover, Browse and its
//! categories, a page per catalogue, search, an app's page, and the Get and Remove sheets.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::PathBuf,
    rc::Rc,
};

use freya::prelude::*;
use isle_catalog::catalogue::{Kind, Offer};
use isle_tokens::Theme;
use isle_ui::{AppIcon, Button, Glyph, button::Kind as ButtonKind, colour, text};
use serde_json::Value;

use crate::{
    data::{Ask, Data},
    installed::{App, Catalogue},
    plan::{self, Plan},
    window::{Page, badge, hue, note, section},
};

/// Browse's categories: a key, the name, the glyph and the AppStream categories in it.
pub const CATEGORIES: &[(&str, &str, &str, &[&str])] = &[
    ("games", "Games", "game-controller", &["Game"]),
    ("create", "Creativity", "shapes", &["Graphics"]),
    (
        "media",
        "Music & Video",
        "film-strip",
        &["AudioVideo", "Audio", "Video"],
    ),
    ("develop", "Development", "code", &["Development"]),
    ("work", "Productivity", "briefcase", &["Office"]),
    ("internet", "Internet", "globe", &["Network"]),
    (
        "learn",
        "Education & Science",
        "book-open",
        &["Education", "Science"],
    ),
    ("utilities", "Utilities", "gear-six", &["Utility"]),
    ("system", "System", "desktop-tower", &["System", "Settings"]),
];

/// Discover's features, a list Isle ships: a catalogue and what installs the app.
const FEATURED: &[(Kind, &str)] = &[
    (Kind::Cachyos, "steam"),
    (Kind::Flathub, "com.heroicgameslauncher.hgl"),
    (Kind::Cachyos, "krita"),
    (Kind::Cachyos, "blender"),
    (Kind::Flathub, "com.obsproject.Studio"),
    (Kind::Flathub, "com.discordapp.Discord"),
    (Kind::Cachyos, "lutris"),
    (Kind::Flathub, "com.usebottles.bottles"),
    (Kind::Flathub, "com.spotify.Client"),
    (Kind::Flathub, "md.obsidian.Obsidian"),
];

/// A page shows this many apps before Show More.
pub const PAGE: usize = 60;

pub fn catalogue_of(kind: Kind) -> Catalogue {
    match kind {
        Kind::Cachyos => Catalogue::Cachyos,
        Kind::Flathub => Catalogue::Flathub,
        Kind::Aur => Catalogue::Aur,
    }
}

pub fn word(kind: Kind) -> &'static str {
    match kind {
        Kind::Cachyos => "cachyos",
        Kind::Flathub => "flathub",
        Kind::Aur => "aur",
    }
}

pub fn key(o: &Offer) -> String {
    format!("{}:{}", word(o.kind), o.install)
}

pub fn get_key(o: &Offer) -> String {
    format!("get:{}", key(o))
}

pub fn remove_key(o: &Offer) -> String {
    format!("remove:{}", key(o))
}

/// The same app in another catalogue: AppStream ids without `.desktop`, else the name.
fn same_app(a: &Offer, b: &Offer) -> bool {
    let id = |o: &Offer| o.id.trim_end_matches(".desktop").to_lowercase();
    (a.kind != Kind::Aur && b.kind != Kind::Aur && id(a) == id(b))
        || a.name.to_lowercase() == b.name.to_lowercase()
}

/// Get goes straight to its run, with no sheet, when the app brings nothing else and nothing
/// needs saying first.
pub fn straight(o: &Offer, plan: &Plan, system_waiting: bool) -> bool {
    plan.refused.is_none()
        && plan.note.is_none()
        && plan.others(&o.install) == 0
        && !(o.kind == Kind::Cachyos && system_waiting)
}

pub fn install_part(o: &Offer) -> String {
    format!("install:{}:{}", word(o.kind), o.install)
}

#[derive(Clone, PartialEq)]
pub enum Sheet {
    Get(Offer),
    Remove(Offer),
}

/// An offer for an app installed with no catalogue entry, so its page can still be shown.
pub fn offer_for(app: &App) -> Offer {
    let kind = match app.catalogue {
        Catalogue::Flathub => Kind::Flathub,
        Catalogue::Aur => Kind::Aur,
        _ => Kind::Cachyos,
    };
    let mut o = bare(kind, &app.source, &app.summary);
    o.id = app.id.clone();
    o.name = app.name.clone();
    o
}

/// An offer known only by its package or app id.
pub fn bare(kind: Kind, install: &str, summary: &str) -> Offer {
    Offer {
        kind,
        id: install.to_string(),
        install: install.to_string(),
        name: install.to_string(),
        summary: summary.to_string(),
        description: Vec::new(),
        icon: None,
        categories: Vec::new(),
        screenshots: Vec::new(),
        licence: None,
        homepage: None,
        developer: None,
        version: None,
        aur: None,
    }
}

/// What every page reads.
#[derive(Clone)]
pub struct Ctx {
    pub theme: Rc<Theme>,
    pub offers: Rc<Vec<Offer>>,
    pub icons: Rc<HashMap<PathBuf, Option<PathBuf>>>,
    /// Catalogue icons a page drew without a drawable copy yet.
    pub wanted: Rc<RefCell<Vec<PathBuf>>>,
    pub shots_wanted: Rc<RefCell<Vec<String>>>,
    pub apps: Rc<Vec<App>>,
    pub packages: Rc<HashSet<String>>,
    pub status: Rc<Value>,
    pub plans: Rc<HashMap<String, Plan>>,
    pub shots: Rc<HashMap<String, PathBuf>>,
    pub aur: Rc<Option<(String, Result<Vec<Offer>, String>)>>,
    pub data: Data,
    pub page: State<Page>,
    pub back: State<Vec<Page>>,
    pub sheet: State<Option<Sheet>>,
    pub limit: State<usize>,
    pub failed: State<Option<String>>,
    /// The Get whose plan, when it comes, may start the run with no sheet.
    pub pending_get: Rc<RefCell<Option<String>>>,
    pub narrow: State<Option<Kind>>,
    /// The search an app's page was opened from: a new one shows results again.
    pub opened_from: State<Option<String>>,
    pub query: String,
    pub narrowing: State<bool>,
    pub system_waiting: bool,
}

impl Ctx {
    pub fn go(&self, to: Page) {
        let (mut page, mut back, mut limit) = (self.page, self.back, self.limit);
        let from = page.read().clone();
        if from != to {
            if matches!(to, Page::App(_)) {
                let mut opened = self.opened_from;
                opened.set(Some(self.query.clone()));
            }
            back.write().push(from);
            page.set(to);
            limit.set(PAGE);
        }
    }

    pub fn ask(&self, ask: Ask) {
        let mut failed = self.failed;
        failed.set(None);
        self.data.ask(ask);
    }

    pub fn find(&self, kind: Kind, install: &str) -> Option<&Offer> {
        self.offers
            .iter()
            .find(|o| o.kind == kind && o.install == install)
    }

    pub fn installed_app(&self, o: &Offer) -> Option<&App> {
        let catalogue = catalogue_of(o.kind);
        self.apps.iter().find(|a| match o.kind {
            Kind::Flathub => a.catalogue == catalogue && (a.id == o.install || a.source == o.install),
            _ => a.source == o.install,
        })
    }

    pub fn is_installed(&self, o: &Offer) -> bool {
        match o.kind {
            Kind::Flathub => self.installed_app(o).is_some(),
            _ => self.packages.contains(&o.install),
        }
    }

    /// The run working on `o`, Get's or Remove's: its fraction done.
    fn busy(&self, o: &Offer) -> Option<(bool, f32)> {
        let run = &self.status["progress"];
        if run["state"] != "Running" {
            return None;
        }
        let part = run["part"].as_str().unwrap_or_default();
        let w = word(o.kind);
        let installing = part == format!("install:{w}:{}", o.install)
            || part == format!("install:{w}+updates:{}", o.install);
        let removing = part == format!("remove:{w}:{}", o.install);
        if !installing && !removing {
            return None;
        }
        let (done, total) = (
            run["done"].as_u64().unwrap_or(0),
            run["total"].as_u64().unwrap_or(0),
        );
        Some((
            installing,
            if total > 0 {
                done as f32 / total as f32
            } else {
                0.
            },
        ))
    }

    fn running(&self) -> bool {
        self.status["progress"]["state"] == "Running"
    }

    pub fn icon(&self, o: &Offer, size: f32) -> Element {
        if let Some(path) = self.installed_app(o).and_then(|a| a.icon.clone()) {
            return AppIcon::new(path, size).into_element();
        }
        if let Some(src) = &o.icon {
            match self.icons.get(src) {
                Some(Some(path)) => return AppIcon::new(path.clone(), size).into_element(),
                Some(None) => {}
                None => self.wanted.borrow_mut().push(src.clone()),
            }
        }
        placeholder(&self.theme, o.kind, size)
    }

    /// Get, Open, or the run working on it.
    pub fn get_button(&self, o: &Offer, small: bool) -> Element {
        let t = &self.theme.tokens;
        if let Some((installing, level)) = self.busy(o) {
            return rect()
                .width(Size::px(if small { 72. } else { 120. }))
                .spacing(t.space_xxs)
                .child(note(
                    &self.theme,
                    if installing { "Installing…" } else { "Removing…" },
                ))
                .child(isle_ui::LevelBar {
                    level,
                    track: colour(t.colour_fill_strong),
                    fill: colour(t.colour_accent),
                })
                .into_element();
        }
        let b = if self.is_installed(o) {
            match self.installed_app(o) {
                Some(app) => {
                    let (ctx, id) = (self.clone(), app.id.clone());
                    Button::new("Open").on_press(move |_| ctx.ask(Ask::Launch(id.clone())))
                }
                None => Button::new("Installed").enabled(false),
            }
        } else {
            let (ctx, offer) = (self.clone(), o.clone());
            Button::new("Get")
                .kind(ButtonKind::Default)
                .enabled(!self.running())
                .on_press(move |_| ctx.get(&offer))
        };
        if small { b.small() } else { b }.into_element()
    }

    /// Get: straight to the run when it brings nothing else, else the sheet listing it.
    pub fn get(&self, o: &Offer) {
        let k = get_key(o);
        match self.plans.get(&k) {
            Some(p) if straight(o, p, self.system_waiting) => {
                self.ask(Ask::Install(install_part(o)));
                return;
            }
            Some(_) => {}
            None => {
                *self.pending_get.borrow_mut() = Some(k.clone());
                self.ask(Ask::PlanGet(k, o.clone()));
            }
        }
        let mut sheet = self.sheet;
        sheet.set(Some(Sheet::Get(o.clone())));
    }

    /// Asks for `o`'s plan once.
    pub fn want_plan(&self, o: &Offer, asked: &RefCell<HashSet<String>>) {
        let k = get_key(o);
        if !self.plans.contains_key(&k) && asked.borrow_mut().insert(k.clone()) {
            self.data.ask(Ask::PlanGet(k, o.clone()));
        }
    }

    fn open(&self, o: &Offer) -> impl Fn(Event<PressEventData>) + 'static {
        let (ctx, to) = (self.clone(), Page::App(Box::new(o.clone())));
        move |_| ctx.go(to.clone())
    }

    /// A list row: icon, name, catalogue and summary, then Get.
    pub fn row(&self, o: &Offer) -> Element {
        let theme = &self.theme;
        let t = &theme.tokens;
        rect()
            .width(Size::fill())
            .padding((t.space_s, 0.))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_m)
            .border(hairline_below(theme))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .spacing(t.space_m)
                    .a11y_role(AccessibilityRole::Link)
                    .a11y_alt(o.name.clone())
                    .on_press(self.open(o))
                    .child(self.icon(o, 48.))
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .spacing(t.space_xxs)
                            .child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(t.space_s)
                                    .child(text::line(
                                        theme,
                                        text::Style::Headline,
                                        colour(t.colour_text),
                                        o.name.clone(),
                                    ))
                                    .child(badge(theme, catalogue_of(o.kind))),
                            )
                            .child(text::line(
                                theme,
                                text::Style::Callout,
                                colour(t.colour_text_2),
                                o.summary.clone(),
                            )),
                    ),
            )
            .child(self.get_button(o, true))
            .into_element()
    }

    /// A tile in Discover's rows.
    fn tile(&self, o: &Offer) -> Element {
        let theme = &self.theme;
        let t = &theme.tokens;
        rect()
            .width(Size::flex(1.))
            .padding(t.space_m)
            .spacing(t.space_xs)
            .corner_radius(t.radius_card)
            .background(colour(t.colour_window_raised))
            .a11y_role(AccessibilityRole::Link)
            .a11y_alt(o.name.clone())
            .on_press(self.open(o))
            .child(self.icon(o, 64.))
            .child(text::line(
                theme,
                text::Style::Headline,
                colour(t.colour_text),
                o.name.clone(),
            ))
            .child(
                text::line(
                    theme,
                    text::Style::Footnote,
                    colour(t.colour_text_2),
                    o.summary.clone(),
                )
                .max_lines(2)
                .height(Size::px(32.)),
            )
            .child(badge(theme, catalogue_of(o.kind)))
            .into_element()
    }

    /// A featured app: large, with its summary and Get.
    fn feature(&self, o: &Offer) -> Element {
        let theme = &self.theme;
        let t = &theme.tokens;
        rect()
            .width(Size::flex(1.))
            .padding(t.space_l)
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_l)
            .corner_radius(t.radius_card)
            .background(colour(t.colour_window_raised))
            .child(
                rect()
                    .on_press(self.open(o))
                    .a11y_role(AccessibilityRole::Link)
                    .a11y_alt(o.name.clone())
                    .child(self.icon(o, 80.)),
            )
            .child(
                rect()
                    .width(Size::flex(1.))
                    .spacing(t.space_xs)
                    .on_press(self.open(o))
                    .child(text::line(
                        theme,
                        text::Style::Title3,
                        colour(t.colour_text),
                        o.name.clone(),
                    ))
                    .child(
                        text::line(
                            theme,
                            text::Style::Callout,
                            colour(t.colour_text_2),
                            o.summary.clone(),
                        )
                        .max_lines(2),
                    )
                    .child(badge(theme, catalogue_of(o.kind))),
            )
            .child(self.get_button(o, false))
            .into_element()
    }

    fn in_category(&self, cats: &[&str]) -> Vec<&Offer> {
        self.offers
            .iter()
            .filter(|o| o.categories.iter().any(|c| cats.contains(&c.as_str())))
            .collect()
    }

    pub fn discover(&self) -> Vec<Element> {
        let theme = &self.theme;
        let t = &theme.tokens;
        let mut out = vec![title(theme, "Discover")];
        if self.offers.is_empty() {
            out.push(note(theme, "Reading the catalogues…"));
            return out;
        }
        let featured: Vec<&Offer> = FEATURED
            .iter()
            .filter_map(|(k, i)| self.find(*k, i))
            .filter(|o| !self.is_installed(o))
            .take(6)
            .collect();
        if !featured.is_empty() {
            out.push(section(theme, "Featured", None));
            for pair in featured.chunks(2) {
                out.push(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .content(Content::flex())
                        .spacing(t.space_m)
                        .children(pair.iter().map(|o| self.feature(o)))
                        .maybe_child((pair.len() == 1).then(|| rect().width(Size::flex(1.)).into_element()))
                        .into_element(),
                );
            }
        }
        let day = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() / 86_400);
        for (key, name, _, cats) in CATEGORIES {
            let picks = picks(self.in_category(cats), day, 4);
            if picks.is_empty() {
                continue;
            }
            let ctx = self.clone();
            out.push(section(
                theme,
                name,
                Some(
                    Button::new("See All")
                        .kind(ButtonKind::Plain)
                        .on_press(move |_| ctx.go(Page::Category(key)))
                        .into_element(),
                ),
            ));
            out.push(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::flex())
                    .spacing(t.space_m)
                    .children(picks.iter().map(|o| self.tile(o)))
                    .children((picks.len()..4).map(|_| rect().width(Size::flex(1.)).into_element()))
                    .into_element(),
            );
        }
        out
    }

    pub fn browse(&self) -> Vec<Element> {
        let theme = &self.theme;
        let t = &theme.tokens;
        let mut out = vec![
            title(theme, "Browse"),
            note(theme, "Apps from CachyOS and Flathub by what they are for. The AUR has no categories: search it."),
        ];
        let tiles: Vec<Element> = CATEGORIES
            .iter()
            .map(|(key, name, glyph, cats)| {
                let count = self.in_category(cats).len();
                let ctx = self.clone();
                rect()
                    .width(Size::flex(1.))
                    .padding(t.space_l)
                    .spacing(t.space_s)
                    .corner_radius(t.radius_card)
                    .background(colour(t.colour_window_raised))
                    .a11y_role(AccessibilityRole::Link)
                    .a11y_alt(*name)
                    .on_press(move |_| ctx.go(Page::Category(key)))
                    .child(Glyph::new(*glyph, 28., colour(t.colour_accent)))
                    .child(text::line(
                        theme,
                        text::Style::Headline,
                        colour(t.colour_text),
                        *name,
                    ))
                    .child(note(theme, format!("{count} apps")))
                    .into_element()
            })
            .collect();
        let mut tiles = tiles.into_iter();
        loop {
            let row: Vec<Element> = tiles.by_ref().take(3).collect();
            if row.is_empty() {
                break;
            }
            let fill = 3 - row.len();
            out.push(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::flex())
                    .spacing(t.space_m)
                    .children(row)
                    .children((0..fill).map(|_| rect().width(Size::flex(1.)).into_element()))
                    .into_element(),
            );
        }
        out
    }

    fn listed(&self, mut list: Vec<&Offer>, out: &mut Vec<Element>) {
        list.sort_by_key(|o| o.name.to_lowercase());
        let limit = *self.limit.read();
        let more = list.len() > limit;
        for o in list.iter().take(limit) {
            out.push(self.row(o));
        }
        if more {
            let mut l = self.limit;
            out.push(
                rect()
                    .width(Size::fill())
                    .padding((t_space(&self.theme), 0.))
                    .center()
                    .child(
                        Button::new(format!("Show More ({} left)", list.len() - limit))
                            .on_press(move |_| {
                                let next = *l.read() + PAGE;
                                l.set(next);
                            }),
                    )
                    .into_element(),
            );
        }
    }

    pub fn category(&self, key: &str) -> Vec<Element> {
        let theme = &self.theme;
        let Some((_, name, _, cats)) = CATEGORIES.iter().find(|c| c.0 == key) else {
            return Vec::new();
        };
        let list = self.in_category(cats);
        let mut out = vec![
            title(theme, name),
            note(theme, format!("{} apps from CachyOS and Flathub", list.len())),
        ];
        self.listed(list, &mut out);
        out
    }

    pub fn catalogue(&self, c: Catalogue) -> Vec<Element> {
        let theme = &self.theme;
        let t = &theme.tokens;
        let (what, glyph) = match c {
            Catalogue::Cachyos => (
                "CachyOS's own repositories: Arch Linux's packages rebuilt for this computer's processor and signed by CachyOS. Apps from here update with the system, in Software Update.",
                "package",
            ),
            Catalogue::Flathub => (
                "Apps packaged by their developers and the Flathub community, each in its own sandbox with its own libraries. They update here, in Store.",
                "stack",
            ),
            Catalogue::Aur => (
                "The Arch User Repository: build scripts anyone can publish, built on this computer. Neither CachyOS nor Arch checks them; read what a package does before you install it.",
                "users",
            ),
            Catalogue::AppImages => (
                "Single-file apps from their developers' websites. Put one in your Applications folder and Isle adds it to Launchpad; it updates from its developer.",
                "folder",
            ),
        };
        let ink = colour(hue(theme, c.hue()));
        let mut out = vec![
            rect()
                .width(Size::fill())
                .padding(t.space_l)
                .horizontal()
                .content(Content::flex())
                .spacing(t.space_l)
                .corner_radius(t.radius_card)
                .background(colour(t.colour_window_raised))
                .child(
                    rect()
                        .width(Size::px(56.))
                        .height(Size::px(56.))
                        .corner_radius(14.)
                        .border(Border::new().fill(ink).width(2.).alignment(BorderAlignment::Inner))
                        .center()
                        .child(Glyph::new(glyph, 28., ink)),
                )
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .spacing(t.space_xs)
                        .child(text::line(
                            theme,
                            text::Style::Title,
                            colour(t.colour_text),
                            c.name(),
                        ))
                        .child(para(theme, what, t.colour_text_2)),
                )
                .into_element(),
        ];
        let kind = match c {
            Catalogue::Cachyos => Some(Kind::Cachyos),
            Catalogue::Flathub => Some(Kind::Flathub),
            _ => None,
        };
        match c {
            Catalogue::Cachyos | Catalogue::Flathub => {
                let list: Vec<&Offer> = self
                    .offers
                    .iter()
                    .filter(|o| Some(o.kind) == kind)
                    .collect();
                if list.is_empty() && c == Catalogue::Flathub {
                    out.push(section(theme, "Flathub isn't set up", None));
                    out.push(para(theme, "Install flatpak from CachyOS, then add Flathub in Terminal: flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo", t.colour_text_2));
                } else if list.is_empty() {
                    out.push(note(theme, "Reading the catalogue…"));
                } else {
                    out.push(section(theme, &format!("{} apps", list.len()), None));
                    self.listed(list, &mut out);
                }
            }
            Catalogue::Aur => {
                out.push(note(theme, "The AUR has thousands of packages and no pictures: search for one by name."));
                self.installed_from(c, &mut out);
            }
            Catalogue::AppImages => self.installed_from(c, &mut out),
        }
        out
    }

    /// The catalogue's entry for an installed app, or one made from what is installed.
    pub fn offer_of(&self, app: &App) -> Offer {
        let found = match app.catalogue {
            Catalogue::Cachyos => self.find(Kind::Cachyos, &app.source),
            Catalogue::Flathub => self.find(Kind::Flathub, &app.source),
            _ => None,
        };
        found.cloned().unwrap_or_else(|| offer_for(app))
    }

    pub fn installed(&self) -> Vec<Element> {
        let theme = &self.theme;
        let mut out = vec![
            title(theme, "Installed"),
            note(
                theme,
                format!("{} apps · the system's packages are not listed here", self.apps.len()),
            ),
        ];
        for a in self.apps.iter() {
            out.push(self.row(&self.offer_of(a)));
        }
        out
    }

    fn installed_from(&self, c: Catalogue, out: &mut Vec<Element>) {
        let theme = &self.theme;
        let mine: Vec<&App> = self.apps.iter().filter(|a| a.catalogue == c).collect();
        out.push(section(theme, "Installed", None));
        if mine.is_empty() {
            out.push(note(theme, "None installed."));
        }
        for a in mine {
            out.push(self.row(&self.offer_of(a)));
        }
    }

    pub fn search(&self, query: &str) -> Vec<Element> {
        let theme = &self.theme;
        let t = &theme.tokens;
        let q = query.trim().to_lowercase();
        let narrow = *self.narrow.read();
        let mut narrowing = self.narrowing;
        let open = *narrowing.read();
        let label = narrow.map_or("All catalogues", |k| catalogue_of(k).name());
        let options: Vec<Element> = [None, Some(Kind::Cachyos), Some(Kind::Flathub), Some(Kind::Aur)]
            .into_iter()
            .map(|k| {
                let mut n = self.narrow;
                Button::new(k.map_or("All catalogues", |k| catalogue_of(k).name()))
                    .kind(if k == narrow { ButtonKind::Default } else { ButtonKind::Plain })
                    .wide()
                    .on_press(move |_| {
                        n.set(k);
                        narrowing.set(false);
                    })
                    .into_element()
            })
            .collect();
        let picker = isle_ui::Popover::new(
            Button::new(format!("{label} ⌄")).on_press(move |_| narrowing.set(!open)),
            open,
            move |_| narrowing.set(false),
        )
        .alt("Catalogues")
        .child(rect().spacing(t.space_xxs).children(options));
        let mut out = vec![
            rect()
                .width(Size::fill())
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .child(rect().width(Size::flex(1.)).child(text::line(
                    theme,
                    text::Style::Title,
                    colour(t.colour_text),
                    format!("Results for “{}”", query.trim()),
                )))
                .child(picker)
                .into_element(),
        ];
        let wants = |k: Kind| narrow.is_none_or(|n| n == k);
        let mut any = false;
        for kind in [Kind::Cachyos, Kind::Flathub] {
            if !wants(kind) {
                continue;
            }
            let mut found: Vec<&Offer> = self
                .offers
                .iter()
                .filter(|o| o.kind == kind && isle_catalog::catalogue::matches(o, &q))
                .collect();
            found.sort_by_key(|o| {
                let n = o.name.to_lowercase();
                (!n.starts_with(&q), !n.contains(&q), n)
            });
            if found.is_empty() {
                continue;
            }
            any = true;
            out.push(section(
                theme,
                &format!("{} · {}", catalogue_of(kind).name(), found.len()),
                None,
            ));
            let cap = if narrow.is_some() { 100 } else { 8 };
            for o in found.iter().take(cap) {
                out.push(self.row(o));
            }
            if found.len() > cap {
                let mut n = self.narrow;
                out.push(
                    Button::new(format!("All {} in {}", found.len(), catalogue_of(kind).name()))
                        .kind(ButtonKind::Plain)
                        .on_press(move |_| n.set(Some(kind)))
                        .into_element(),
                );
            }
        }
        if wants(Kind::Aur) && q.len() >= 2 {
            match self.aur.as_ref() {
                Some((term, Ok(list))) if term.to_lowercase() == q => {
                    if !list.is_empty() {
                        any = true;
                        out.push(section(theme, &format!("AUR · {}", list.len()), None));
                        out.push(note(theme, "Built on this computer from community scripts nobody checks."));
                        let cap = if narrow.is_some() { 100 } else { 8 };
                        for o in list.iter().take(cap) {
                            out.push(self.row(o));
                        }
                        if list.len() > cap {
                            let mut n = self.narrow;
                            out.push(
                                Button::new(format!("All {} in the AUR", list.len()))
                                    .kind(ButtonKind::Plain)
                                    .on_press(move |_| n.set(Some(Kind::Aur)))
                                    .into_element(),
                            );
                        }
                    }
                }
                Some((term, Err(why))) if term.to_lowercase() == q => {
                    out.push(section(theme, "AUR", None));
                    out.push(note(theme, format!("The AUR could not be searched: {why}")));
                }
                _ => {
                    out.push(section(theme, "AUR", None));
                    out.push(note(theme, "Searching the AUR…"));
                }
            }
        }
        let aur_answered = self
            .aur
            .as_ref()
            .as_ref()
            .is_some_and(|(t, got)| t.to_lowercase() == q && got.is_ok());
        if !any && (narrow.is_some_and(|k| k != Kind::Aur) || q.len() < 2 || aur_answered) {
            out.push(note(theme, "Nothing found."));
        }
        out
    }

    /// An app's page.
    pub fn app(&self, o: &Offer, asked: &RefCell<HashSet<String>>) -> Vec<Element> {
        let theme = &self.theme;
        let t = &theme.tokens;
        self.want_plan(o, asked);
        let plan = self.plans.get(&get_key(o));
        let installed = self.is_installed(o);
        let size_line = match plan {
            _ if installed => "Installed".to_owned(),
            None => "Working out its size…".to_owned(),
            Some(p) if p.refused.is_some() => p.refused.clone().unwrap_or_default(),
            Some(p) => {
                let total = p.total();
                let others = p.others(&o.install);
                let mut words = Vec::new();
                if total > 0 {
                    words.push(plan::size(total));
                }
                if others > 0 {
                    words.push(format!(
                        "also installs {others} package{}",
                        if others == 1 { "" } else { "s" }
                    ));
                }
                if o.kind == Kind::Aur {
                    words.push("built on this computer".into());
                }
                words.join(" · ")
            }
        };
        let ctx = self.clone();
        let offer = o.clone();
        let remove = installed.then(|| {
            Button::new("Remove…")
                .kind(ButtonKind::Plain)
                .small()
                .enabled(!self.running())
                .on_press(move |_| {
                    let mut sheet = ctx.sheet;
                    let k = remove_key(&offer);
                    if !ctx.plans.contains_key(&k) {
                        ctx.ask(Ask::PlanRemove(k, word(offer.kind).into(), offer.install.clone()));
                    }
                    sheet.set(Some(Sheet::Remove(offer.clone())));
                })
                .into_element()
        });
        let mut out = vec![
            rect()
                .width(Size::fill())
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_xl)
                .child(self.icon(o, 128.))
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .spacing(t.space_xs)
                        .child(text::line(
                            theme,
                            text::Style::Title,
                            colour(t.colour_text),
                            o.name.clone(),
                        ))
                        .child(para(theme, &o.summary, t.colour_text_2))
                        .maybe_child(o.developer.clone().map(|d| note(theme, d)))
                        .child(badge(theme, catalogue_of(o.kind))),
                )
                .child(
                    rect()
                        .cross_align(Alignment::End)
                        .spacing(t.space_xs)
                        .child(self.get_button(o, false))
                        .child(note(theme, size_line))
                        .maybe_child(remove),
                )
                .into_element(),
        ];

        let elsewhere: Vec<&Offer> = self
            .offers
            .iter()
            .filter(|x| x.kind != o.kind && same_app(x, o))
            .collect();
        if !elsewhere.is_empty() {
            let mut row = rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(note(theme, "Also in"));
            for x in elsewhere.iter().take(3) {
                let (ctx, to) = (self.clone(), Page::App(Box::new((*x).clone())));
                row = row.child(
                    Button::new(catalogue_of(x.kind).name())
                        .small()
                        .on_press(move |_| ctx.go(to.clone())),
                );
            }
            out.push(row.into_element());
        }

        if o.kind == Kind::Aur {
            out.push(warning(theme, "From the AUR: built on this computer from a script anyone can publish. Neither CachyOS nor Arch checks it."));
            if self.system_waiting {
                out.push(note(theme, "Update CachyOS first, in Software Update: the AUR builds against the system as it is."));
            }
        }

        if !o.screenshots.is_empty() {
            let mut shots = rect().horizontal().spacing(t.space_m);
            for url in o.screenshots.iter().take(4) {
                let frame = rect()
                    .width(Size::px(420.))
                    .height(Size::px(260.))
                    .corner_radius(t.radius_card)
                    .background(colour(t.colour_window_sunken))
                    .overflow(Overflow::Clip)
                    .center();
                shots = shots.child(match self.shots.get(url) {
                    Some(path) => frame
                        .child(
                            ImageViewer::new(path.clone())
                                .expanded()
                                .aspect_ratio(AspectRatio::Min)
                                .image_cover(ImageCover::Center),
                        )
                        .into_element(),
                    None => {
                        self.shots_wanted.borrow_mut().push(url.clone());
                        frame.child(isle_ui::Spinner::new(24., "Loading")).into_element()
                    }
                });
            }
            out.push(
                isle_ui::scroll_area(theme)
                    .direction(Direction::Horizontal)
                    .width(Size::fill())
                    .height(Size::px(276.))
                    .child(shots)
                    .into_element(),
            );
        }

        if !o.description.is_empty() {
            out.push(section(theme, "About", None));
            for p in o.description.iter().take(8) {
                out.push(para(theme, p, t.colour_text));
            }
        }

        if let Some(p) = plan.filter(|p| !p.extras.is_empty()) {
            out.push(section(theme, "Works better with", None));
            out.push(note(theme, "Optional extras; none is installed unless you get it."));
            for (name, why) in &p.extras {
                let kind = if o.kind == Kind::Aur && self.find(Kind::Cachyos, name).is_none() {
                    Kind::Aur
                } else {
                    Kind::Cachyos
                };
                let extra = bare(kind, name, why);
                let shown = self.find(Kind::Cachyos, name).cloned().map(|mut f| {
                    f.summary = why.clone();
                    f
                });
                out.push(self.row(shown.as_ref().unwrap_or(&extra)));
            }
        }

        out.push(section(theme, "Information", None));
        let mut facts: Vec<(&str, String)> = vec![("Catalogue", catalogue_of(o.kind).name().into())];
        facts.push((
            match o.kind {
                Kind::Flathub => "App ID",
                _ => "Package",
            },
            o.install.clone(),
        ));
        // The package's own version over AppStream's newest release, which can lag it.
        let packaged = plan
            .and_then(|p| p.packages.iter().find(|x| x.name == o.install))
            .map(|x| x.version.clone())
            .filter(|v| !v.is_empty());
        if let Some(v) = packaged.or_else(|| o.version.clone()) {
            facts.push(("Version", v));
        }
        if let Some(p) = plan.filter(|p| p.total() > 0) {
            facts.push(("Download", plan::size(p.total())));
        }
        if let Some(d) = &o.developer {
            facts.push(("Developer", d.clone()));
        }
        if let Some(l) = &o.licence {
            facts.push(("Licence", l.clone()));
        }
        if let Some(h) = &o.homepage {
            facts.push(("Website", h.clone()));
        }
        if let Some(f) = &o.aur {
            facts.push(("Votes", f.votes.to_string()));
            facts.push(("Popularity", format!("{:.2}", f.popularity)));
            facts.push(("Maintainer", f.maintainer.clone().unwrap_or_else(|| "None (orphaned)".into())));
            facts.push(("Last updated", days_ago(f.last_modified)));
            if f.out_of_date {
                facts.push(("Out of date", "Flagged by its users".into()));
            }
        }
        for (k, v) in facts {
            out.push(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::flex())
                    .padding((t.space_xs, 0.))
                    .border(hairline_below(theme))
                    .child(
                        rect().width(Size::px(160.)).child(text::line(
                            theme,
                            text::Style::Callout,
                            colour(t.colour_text_2),
                            k,
                        )),
                    )
                    .child(rect().width(Size::flex(1.)).child(text::line(
                        theme,
                        text::Style::Callout,
                        colour(t.colour_text),
                        v,
                    )))
                    .into_element(),
            );
        }
        out
    }

    /// The Get or Remove sheet over the window.
    pub fn sheet(&self, sheet: &Sheet) -> Element {
        let theme = &self.theme;
        let t = &theme.tokens;
        let mut close = self.sheet;
        let cancel = (
            "Cancel".to_string(),
            EventHandler::new(move |_| close.set(None)),
        );
        let (o, removing) = match sheet {
            Sheet::Get(o) => (o, false),
            Sheet::Remove(o) => (o, true),
        };
        let k = if removing { remove_key(o) } else { get_key(o) };
        let Some(plan) = self.plans.get(&k) else {
            return isle_ui::sheet(
                460.,
                vec![
                    isle_ui::alert_text(
                        &o.name,
                        if removing {
                            "Working out what goes with it…"
                        } else {
                            "Working out what it needs…"
                        },
                    ),
                    rect().width(Size::fill()).center().child(isle_ui::Spinner::new(24., "Loading")).into_element(),
                ],
                vec![cancel],
            );
        };
        if let Some(why) = &plan.refused {
            let words = if removing {
                format!("{} can't be removed. {why}", o.name)
            } else {
                why.clone()
            };
            return isle_ui::sheet(
                460.,
                vec![isle_ui::alert_text(&o.name, &words)],
                vec![("OK".to_string(), EventHandler::new(move |_| close.set(None)))],
            );
        }
        let aur_blocked = !removing && o.kind == Kind::Aur && self.system_waiting;
        let with_updates = !removing && o.kind == Kind::Cachyos && self.system_waiting;
        let others = plan.others(&o.install);
        let (title, message) = if removing {
            (
                format!("Remove {}?", o.name),
                if plan.packages.len() > 1 {
                    format!(
                        "{} packages go with it, which nothing else needs.",
                        plan.packages.len() - 1
                    )
                } else {
                    "Nothing else goes with it.".into()
                },
            )
        } else {
            let total = plan.total();
            (
                format!("Install {}?", o.name),
                format!(
                    "From {}. {}{}",
                    match o.kind {
                        Kind::Aur => "the AUR",
                        k => catalogue_of(k).name(),
                    },
                    if others > 0 {
                        format!(
                            "It also installs {others} package{} it needs",
                            if others == 1 { "" } else { "s" }
                        )
                    } else {
                        "It needs nothing else".into()
                    },
                    if total > 0 {
                        format!(", {} in all.", plan::size(total))
                    } else {
                        ".".into()
                    }
                ),
            )
        };
        let mut body = vec![isle_ui::alert_text(&title, &message)];
        let list: Vec<Element> = plan
            .packages
            .iter()
            .map(|p| {
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::flex())
                    .padding((t.space_xxs, 0.))
                    .child(rect().width(Size::flex(1.)).child(text::line(
                        theme,
                        text::Style::Callout,
                        colour(t.colour_text),
                        p.name.clone(),
                    )))
                    .child(note(
                        theme,
                        [p.version.clone(), p.size.map(plan::size).unwrap_or_default()]
                            .into_iter()
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join(" · "),
                    ))
                    .into_element()
            })
            .collect();
        if !list.is_empty() {
            body.push(
                isle_ui::scroll_area(theme)
                    .width(Size::fill())
                    .max_height(Size::px(220.))
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((0., t.space_l, 0., 0.))
                            .children(list),
                    )
                    .into_element(),
            );
        }
        if let Some(n) = &plan.note {
            body.push(if o.kind == Kind::Aur && !removing {
                warning(theme, n)
            } else {
                note(theme, n.clone())
            });
        }
        if with_updates {
            body.push(note(theme, "CachyOS has updates waiting. They are installed with it, so nothing is left half-updated."));
        }
        if aur_blocked {
            body.push(note(theme, "Update CachyOS first, in Software Update: the AUR builds against the system as it is."));
            let ctx = self.clone();
            return isle_ui::sheet(
                500.,
                body,
                vec![
                    cancel,
                    (
                        "Software Update…".into(),
                        EventHandler::new(move |_| {
                            let mut s = ctx.sheet;
                            s.set(None);
                            ctx.ask(Ask::SoftwareUpdate);
                        }),
                    ),
                ],
            );
        }
        let ctx = self.clone();
        let part = if removing {
            format!("remove:{}:{}", word(o.kind), o.install)
        } else {
            install_part(o)
        };
        let label = match (removing, with_updates) {
            (true, _) => "Remove",
            (false, true) => "Update and Install",
            (false, false) => "Install",
        };
        isle_ui::sheet(
            500.,
            body,
            vec![
                cancel,
                (
                    label.into(),
                    EventHandler::new(move |_| {
                        let mut s = ctx.sheet;
                        s.set(None);
                        ctx.ask(Ask::Install(part.clone()));
                    }),
                ),
            ],
        )
    }
}

/// `n` of `list` chosen afresh each day among those with an icon and a screenshot, one per name.
fn picks(list: Vec<&Offer>, day: u64, n: usize) -> Vec<&Offer> {
    use std::hash::{Hash, Hasher};
    let mut good: Vec<(u64, &Offer)> = list
        .into_iter()
        .filter(|o| o.icon.is_some() && !o.screenshots.is_empty())
        .map(|o| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (day, &o.id).hash(&mut h);
            (h.finish(), o)
        })
        .collect();
    good.sort_by_key(|(h, _)| *h);
    let mut seen = HashSet::new();
    good.into_iter()
        .map(|(_, o)| o)
        .filter(|o| seen.insert(o.name.to_lowercase()))
        .take(n)
        .collect()
}

fn days_ago(secs: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    match now.saturating_sub(secs) / 86_400 {
        0 => "Today".into(),
        1 => "Yesterday".into(),
        d if d < 60 => format!("{d} days ago"),
        d if d < 730 => format!("{} months ago", d / 30),
        d => format!("{} years ago", d / 365),
    }
}

fn t_space(theme: &Theme) -> f32 {
    theme.tokens.space_m
}

pub fn hairline_below(theme: &Theme) -> Border {
    let t = &theme.tokens;
    Border::new().fill(colour(t.colour_hairline)).width(BorderWidth {
        top: 0.,
        right: 0.,
        bottom: t.stroke_hairline,
        left: 0.,
    })
}

pub fn title(theme: &Theme, words: &str) -> Element {
    text::line(
        theme,
        text::Style::Title,
        colour(theme.tokens.colour_text),
        words,
    )
    .into_element()
}

pub fn para(theme: &Theme, words: &str, ink: isle_tokens::Colour) -> Element {
    text::line(theme, text::Style::Body, colour(ink), words)
        .width(Size::fill())
        .max_lines(None)
        .text_overflow(TextOverflow::Clip)
        .into_element()
}

fn warning(theme: &Theme, words: &str) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .padding(t.space_m)
        .horizontal()
        .content(Content::flex())
        .spacing(t.space_s)
        .corner_radius(t.radius_card)
        .background(colour(t.colour_window_sunken))
        .child(Glyph::new("warning", t.size_glyph, colour(t.colour_warn)))
        .child(rect().width(Size::flex(1.)).child(para(theme, words, t.colour_text)))
        .into_element()
}

fn placeholder(theme: &Theme, kind: Kind, size: f32) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::px(size))
        .height(Size::px(size))
        .corner_radius(size / 4.)
        .background(colour(t.colour_fill_strong))
        .center()
        .child(Glyph::new(
            if kind == Kind::Aur { "package" } else { "app-window" },
            size * 0.5,
            colour(t.colour_text_2),
        ))
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(kind: Kind, id: &str, name: &str) -> Offer {
        Offer {
            kind,
            id: id.into(),
            install: id.into(),
            name: name.into(),
            summary: String::new(),
            description: Vec::new(),
            icon: Some("/i.png".into()),
            categories: vec!["Graphics".into()],
            screenshots: vec!["https://x/1.png".into()],
            licence: None,
            homepage: None,
            developer: None,
            version: None,
            aur: None,
        }
    }

    #[test]
    fn the_same_app_across_catalogues() {
        let a = offer(Kind::Cachyos, "org.kde.krita.desktop", "Krita");
        let b = offer(Kind::Flathub, "org.kde.krita", "krita");
        let c = offer(Kind::Flathub, "org.gimp.GIMP", "GNU Image Manipulation Program");
        assert!(same_app(&a, &b));
        assert!(!same_app(&a, &c));
    }

    #[test]
    fn get_goes_straight_only_when_nothing_needs_saying() {
        let o = offer(Kind::Cachyos, "krita", "Krita");
        let alone = Plan {
            packages: vec![plan::Package {
                name: "krita".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(straight(&o, &alone, false));
        assert!(!straight(&o, &alone, true));
        let mut more = alone.clone();
        more.packages.push(plan::Package {
            name: "kimageformats".into(),
            ..Default::default()
        });
        assert!(!straight(&o, &more, false));
    }

    #[test]
    fn discover_picks_one_per_name_each_day() {
        let a = offer(Kind::Cachyos, "a", "Same");
        let b = offer(Kind::Flathub, "b", "same");
        let mut c = offer(Kind::Flathub, "c", "Bare");
        c.screenshots.clear();
        let got = picks(vec![&a, &b, &c], 1, 4);
        assert_eq!(got.len(), 1);
        assert_eq!(picks(vec![&a, &b], 1, 4), picks(vec![&a, &b], 1, 4));
    }
}
