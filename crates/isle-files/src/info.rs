//! Get Info: a window per item, one summary of several, and the Inspector that follows the
//! front window's selection (`p1/structure/files.md` § Get Info).

use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    rc::Rc,
};

use freya::prelude::*;
use isle_fs::Entry;
use isle_ui::{Glyph, colour, field::CheckRow, text, use_theme};

use crate::{Shared, format, list_view, window::off_thread};

const COMMENT: &str = "user.xdg.comment";

/// What a Get Info window shows.
#[derive(Clone, Debug, PartialEq)]
pub enum InfoOf {
    One(PathBuf),
    Summary(Vec<PathBuf>),
    /// The front window's selection, as it changes.
    Inspector,
}

impl InfoOf {
    pub fn title(&self) -> String {
        match self {
            InfoOf::One(path) => format!("{} Info", crate::places::folder_name(path)),
            InfoOf::Summary(items) => format!("{} Info", format::items(items.len())),
            InfoOf::Inspector => "Inspector".into(),
        }
    }
}

#[derive(Clone)]
pub struct InfoWindow {
    pub shared: Rc<Shared>,
    pub of: InfoOf,
}

/// Read, write and none, as Sharing & Permissions offers them per class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    ReadWrite,
    ReadOnly,
    NoAccess,
}

pub const ACCESS: [(Access, &str); 3] =
    [(Access::ReadWrite, "Read & Write"), (Access::ReadOnly, "Read only"), (Access::NoAccess, "No access")];

/// A class's access from its three bits.
pub fn access_of(bits: u32) -> Access {
    match (bits & 4 != 0, bits & 2 != 0) {
        (true, true) => Access::ReadWrite,
        (true, false) => Access::ReadOnly,
        _ => Access::NoAccess,
    }
}

/// `mode` with the class at `shift` (6 owner, 3 group, 0 others) given `access`: a folder's
/// search bit goes with read, a file's run bit is kept while it can be read.
pub fn with_access(mode: u32, shift: u32, access: Access, dir: bool) -> u32 {
    let old = (mode >> shift) & 7;
    let run = if dir { 1 } else { old & 1 };
    let bits = match access {
        Access::ReadWrite => 6 | run,
        Access::ReadOnly => 4 | run,
        Access::NoAccess => 0,
    };
    (mode & !(7 << shift)) | (bits << shift)
}

/// `mode` allowed or not to run as a program: the run bit for every class that can read.
pub fn with_running(mode: u32, on: bool) -> u32 {
    let mut next = mode & !0o111;
    if on {
        for shift in [6, 3, 0] {
            if (mode >> shift) & 4 != 0 {
                next |= 1 << shift;
            }
        }
    }
    next
}

/// A mounted volume's facts, when the item is its mount point.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumeFacts {
    pub format: String,
    pub capacity: u64,
    pub available: u64,
}

/// What Get Info reads of one item, off the window's thread.
#[derive(Clone, Debug, PartialEq)]
pub struct Facts {
    pub entry: Entry,
    pub on_disk: u64,
    pub owner: String,
    pub group: String,
    pub mine: bool,
    pub comment: String,
    pub dimensions: Option<(u32, u32)>,
    pub volume: Option<VolumeFacts>,
    /// Open With's apps for its type, the default first, and the type.
    pub apps: Vec<crate::open::Choice>,
    pub kind: String,
}

fn name_of(file: &str, id: u32) -> String {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                let mut parts = line.split(':');
                let name = parts.next()?;
                (parts.nth(1)?.parse::<u32>().ok()? == id).then(|| name.to_owned())
            })
        })
        .unwrap_or_else(|| id.to_string())
}

pub fn comment(path: &Path) -> String {
    let mut buf = vec![0u8; 4096];
    match rustix::fs::getxattr(path, COMMENT, &mut buf) {
        Ok(n) => String::from_utf8_lossy(&buf[..n]).into_owned(),
        Err(_) => String::new(),
    }
}

/// Writes the item's comment; an empty one is removed.
pub fn set_comment(path: &Path, text: &str) -> rustix::io::Result<()> {
    if text.is_empty() {
        return match rustix::fs::removexattr(path, COMMENT) {
            Err(rustix::io::Errno::NODATA) => Ok(()),
            other => other,
        };
    }
    rustix::fs::setxattr(path, COMMENT, text.as_bytes(), rustix::fs::XattrFlags::empty())
}

/// Reads an item's facts; blocks on the filesystem.
pub fn read_facts(path: &Path) -> Option<Facts> {
    let entry = Entry::at(path).ok()?;
    let meta = std::fs::symlink_metadata(path).ok()?;
    let table = isle_fs::mounts::read();
    let volume = isle_fs::mounts::of(&table, path).filter(|m| m.point == path).and_then(|m| {
        let stat = rustix::fs::statvfs(path).ok()?;
        Some(VolumeFacts {
            format: m.fstype.clone(),
            capacity: stat.f_blocks * stat.f_frsize,
            available: stat.f_bavail * stat.f_frsize,
        })
    });
    let dimensions = entry
        .mime
        .as_deref()
        .filter(|m| m.starts_with("image/"))
        .and_then(|_| imagesize::size(path).ok())
        .map(|d| (d.width as u32, d.height as u32));
    let (apps, kind) = if entry.is_dir() {
        (Vec::new(), String::new())
    } else {
        let offer = crate::open::offer(path);
        (offer.apps, offer.kind)
    };
    Some(Facts {
        on_disk: meta.blocks() * 512,
        owner: name_of("/etc/passwd", meta.uid()),
        group: name_of("/etc/group", meta.gid()),
        mine: meta.uid() == rustix::process::getuid().as_raw(),
        comment: comment(path),
        dimensions,
        volume,
        apps,
        kind,
        entry,
    })
}

impl App for InfoWindow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let shared = self.shared.clone();
        let paths: Vec<PathBuf> = match &self.of {
            InfoOf::One(path) => vec![path.clone()],
            InfoOf::Summary(items) => items.clone(),
            InfoOf::Inspector => shared.selection.read().clone(),
        };
        let mut facts = use_state(Vec::<Facts>::new);
        // Counts each change made here, so the facts are read again after it.
        let changed = use_state(|| 0u64);
        let reading = (paths.clone(), *changed.read());
        use_side_effect_with_deps(&reading, move |(paths, _)| {
            let paths = paths.clone();
            spawn(async move {
                if let Some(read) = off_thread(move || paths.iter().filter_map(|p| read_facts(p)).collect()).await {
                    facts.set(read);
                }
            });
        });
        let collapsed = shared.collapsed_info();
        let body: Element = match (facts.read().as_slice(), &self.of) {
            ([], InfoOf::Inspector) => rect()
                .expanded()
                .center()
                .child(text::line(&theme, text::Style::Body, colour(t.colour_text_2), "No selection"))
                .into_element(),
            ([], _) => rect().into_element(),
            ([one], _) => one_item(&theme, &shared, one.clone(), collapsed, move || {
                let mut changed = changed;
                let n = *changed.peek();
                changed.set(n + 1);
            }),
            (many, _) => summary(&theme, many),
        };
        rect()
            .expanded()
            .background(colour(t.colour_window))
            .child(isle_ui::scroll_area(&theme).width(Size::fill()).height(Size::fill()).child(body))
    }
}

/// A section: its title with a disclosure triangle, its rows while open.
fn section(
    theme: &isle_tokens::Theme,
    shared: &Rc<Shared>,
    title: &'static str,
    collapsed: &[String],
    rows: Vec<Element>,
) -> Element {
    let t = &theme.tokens;
    let open = !collapsed.iter().any(|c| c == title);
    let toggle = shared.clone();
    rect()
        .width(Size::fill())
        .padding((t.space_s, t.space_l))
        .spacing(t.space_xs)
        .border(
            Border::new()
                .fill(colour(t.colour_hairline))
                .width(BorderWidth { top: 1., right: 0., bottom: 0., left: 0. })
                .alignment(BorderAlignment::Inner),
        )
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(title)
                .a11y_builder(move |node| node.set_expanded(open))
                .on_press(move |_| toggle.toggle_info_section(title))
                .child(Glyph::new(if open { "caret-down" } else { "caret-right" }, t.size_glyph_small, colour(t.colour_text_2)))
                .child(text::line(theme, text::Style::Subheadline, colour(t.colour_text), title).font_weight(600)),
        )
        .maybe_child(open.then(|| rect().width(Size::fill()).padding((0., 0., 0., t.space_l)).spacing(t.space_xs).children(rows)))
        .into_element()
}

fn fact(theme: &isle_tokens::Theme, name: &str, value: String) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .horizontal()
        .spacing(t.space_s)
        .child(rect().width(Size::px(96.)).main_align(Alignment::End).horizontal().child(text::line(
            theme,
            text::Style::Callout,
            colour(t.colour_text_2),
            format!("{name}:"),
        )))
        .child(
            rect().width(Size::flex(1.)).child(
                label()
                    .width(Size::fill())
                    .font_size(text::metrics(theme, text::Style::Callout).0)
                    .color(colour(t.colour_text))
                    .max_lines(3)
                    .text_overflow(TextOverflow::Ellipsis)
                    .text(value),
            ),
        )
        .into_element()
}

fn size_line(f: &Facts, counted: Option<&crate::preview::Counted>) -> String {
    if f.entry.is_dir() {
        return match counted {
            Some(c) => format!("{} on disk for {}", format::size(c.bytes), format::items(c.items)),
            None => "Calculating…".into(),
        };
    }
    format!("{} ({} on disk)", format::size(f.entry.size), format::size(f.on_disk))
}

fn one_item(
    theme: &isle_tokens::Theme,
    shared: &Rc<Shared>,
    f: Facts,
    collapsed: Vec<String>,
    changed: impl Fn() + Clone + 'static,
) -> Element {
    OneItem {
        theme_key: theme.appearance.mode == isle_tokens::Mode::Dark,
        shared: shared.clone(),
        facts: f,
        collapsed,
        changed: EventHandler::new(move |_: ()| changed()),
    }
    .into_element()
}

#[derive(Clone)]
struct OneItem {
    theme_key: bool,
    shared: Rc<Shared>,
    facts: Facts,
    collapsed: Vec<String>,
    changed: EventHandler<()>,
}

impl PartialEq for OneItem {
    fn eq(&self, other: &Self) -> bool {
        self.theme_key == other.theme_key && self.facts == other.facts && self.collapsed == other.collapsed
    }
}

impl Component for OneItem {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let f = &self.facts;
        let entry = &f.entry;
        let dark = theme.appearance.mode == isle_tokens::Mode::Dark;
        let now = format::now();
        let mut counted = use_state(|| None::<crate::preview::Counted>);
        let counting = entry.is_dir().then(|| entry.path.clone());
        use_side_effect_with_deps(&counting, move |counting| {
            let Some(path) = counting.clone() else { return };
            counted.set(None);
            spawn(async move {
                if let Some(found) = off_thread(move || crate::preview::count(path)).await {
                    counted.set(Some(found));
                }
            });
        });
        let name = use_state({
            let n = entry.name.clone();
            move || n
        });
        let note = use_state({
            let c = f.comment.clone();
            move || c
        });
        let mut chosen_app = use_state(|| 0usize);
        let mut asking = use_state(|| false);
        let counted_now = counted.read().clone().filter(|c| c.path == entry.path);
        let kind = list_view::kind(entry);
        let shared = &self.shared;

        let header = rect()
            .width(Size::fill())
            .padding(t.space_l)
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_m)
            .child(crate::thumbnail::Thumbnail { entry: entry.clone(), size: 48., dark, remote: false })
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(text::line(&theme, text::Style::Headline, colour(t.colour_text), entry.name.clone()))
                    .child(text::line(
                        &theme,
                        text::Style::Callout,
                        colour(t.colour_text_2),
                        format!("Modified: {}", format::date(entry.modified, &now)),
                    )),
            )
            .child(text::line(
                &theme,
                text::Style::Callout,
                colour(t.colour_text_2),
                if entry.is_dir() {
                    counted_now.as_ref().map(|c| format::size(c.bytes)).unwrap_or_else(|| "--".into())
                } else {
                    format::size(entry.size)
                },
            ));

        let mut general = vec![
            fact(&theme, "Kind", kind),
            fact(&theme, "Size", size_line(f, counted_now.as_ref())),
            fact(&theme, "Where", entry.path.parent().map(|p| p.display().to_string()).unwrap_or_default()),
        ];
        general.extend(entry.created.map(|c| fact(&theme, "Created", format::date(c, &now))));
        general.push(fact(&theme, "Modified", format::date(entry.modified, &now)));
        general.push(fact(&theme, "Last opened", format::date(entry.accessed, &now)));

        let mut sections = vec![header.into_element(), section(&theme, shared, "General", &self.collapsed, general)];
        if let Some(v) = &f.volume {
            sections.push(section(
                &theme,
                shared,
                "Volume",
                &self.collapsed,
                vec![
                    fact(&theme, "Format", v.format.clone()),
                    fact(&theme, "Capacity", format::size(v.capacity)),
                    fact(&theme, "Available", format::size(v.available)),
                    fact(&theme, "Used", format::size(v.capacity.saturating_sub(v.available))),
                    fact(&theme, "Mount point", entry.path.display().to_string()),
                ],
            ));
        }
        if let Some((w, h)) = f.dimensions {
            sections.push(section(&theme, shared, "More Info", &self.collapsed, vec![fact(&theme, "Dimensions", format!("{w} × {h}"))]));
        }

        let rename = {
            let (shared, path, changed) = (shared.clone(), entry.path.clone(), self.changed.clone());
            move |typed: String| {
                let typed = typed.trim().to_owned();
                let current = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if typed.is_empty() || typed == current || typed.contains('/') {
                    return;
                }
                if let Some(key) = shared.front_window() {
                    shared.jobs.start(
                        &shared.app(),
                        key,
                        isle_fs::jobs::Work::Rename { item: path.clone(), to: typed },
                        "Renaming".into(),
                        "Rename".into(),
                    );
                }
                changed.call(());
            }
        };
        sections.push(section(
            &theme,
            shared,
            "Name & Extension",
            &self.collapsed,
            vec![isle_ui::TextField::new(name, "Name").on_submit(rename).into_element()],
        ));
        let save_note = {
            let (path, changed) = (entry.path.clone(), self.changed.clone());
            move |typed: String| {
                if let Err(err) = set_comment(&path, typed.trim()) {
                    tracing::warn!("the comment is not kept: {err}");
                }
                changed.call(());
            }
        };
        sections.push(section(
            &theme,
            shared,
            "Comments",
            &self.collapsed,
            vec![isle_ui::TextField::new(note, "Comments").on_submit(save_note).into_element()],
        ));

        if !entry.is_dir() && !f.apps.is_empty() {
            let at = (*chosen_app.read()).min(f.apps.len() - 1);
            let apps = f.apps.clone();
            sections.push(section(
                &theme,
                shared,
                "Open with",
                &self.collapsed,
                vec![
                    isle_ui::form::PopUp::new(
                        apps.iter().map(|a| a.name.clone()).collect(),
                        at,
                        "Open with",
                        move |i: usize| chosen_app.set(i),
                    )
                    .into_element(),
                    text::line(
                        &theme,
                        text::Style::Callout,
                        colour(t.colour_text_2),
                        "Change All… opens every document like this one in it.".to_owned(),
                    )
                    .into_element(),
                    isle_ui::Button::new("Change All…")
                        .small()
                        .enabled(at != 0)
                        .on_press(move |_| asking.set(true))
                        .into_element(),
                ],
            ));
        }
        sections.push(section(
            &theme,
            shared,
            "Preview",
            &self.collapsed,
            vec![
                rect()
                    .width(Size::fill())
                    .center()
                    .child(crate::thumbnail::Thumbnail { entry: entry.clone(), size: 200., dark, remote: false })
                    .into_element(),
            ],
        ));
        sections.push(section(&theme, shared, "Sharing & Permissions", &self.collapsed, permissions(&theme, f, self.changed.clone())));

        let ask = (*asking.read()).then(|| {
            let app = f.apps.get(*chosen_app.read()).cloned();
            let (kind, changed) = (f.kind.clone(), self.changed.clone());
            let name = app.as_ref().map(|a| a.name.clone()).unwrap_or_default();
            isle_ui::sheet(
                t.size_form_narrow,
                vec![isle_ui::alert_text(
                    &format!("Change all similar documents to open with {name}?"),
                    "This change applies to every document of this kind.",
                )],
                vec![
                    ("Cancel".into(), EventHandler::new(move |_| asking.set(false))),
                    (
                        "Continue".into(),
                        EventHandler::new(move |_| {
                            asking.set(false);
                            let Some(app) = app.clone() else { return };
                            if let Err(err) = isle_apps::mimeapps::set_default(&[&kind], &app.id, false) {
                                tracing::warn!("the default is not kept: {err}");
                            }
                            chosen_app.set(0);
                            changed.call(());
                        }),
                    ),
                ],
            )
        });
        rect().width(Size::fill()).children(sections).maybe_child(ask)
    }
}

fn permissions(theme: &isle_tokens::Theme, f: &Facts, changed: EventHandler<()>) -> Vec<Element> {
    let t = &theme.tokens;
    let mode = f.entry.mode & 0o7777;
    let dir = f.entry.is_dir();
    let path = f.entry.path.clone();
    let apply = {
        let (path, changed) = (path.clone(), changed.clone());
        move |next: u32| {
            if let Err(err) = rustix::fs::chmod(&path, rustix::fs::Mode::from_raw_mode(next)) {
                tracing::warn!("permissions are not changed: {err}");
            }
            changed.call(());
        }
    };
    let me = format!("{} (Me)", f.owner);
    let classes = [(if f.mine { me } else { f.owner.clone() }, 6u32), (f.group.clone(), 3), ("everyone".to_owned(), 0)];
    let mut rows: Vec<Element> = classes
        .into_iter()
        .map(|(who, shift)| {
            let now = access_of((mode >> shift) & 7);
            let apply = apply.clone();
            rect()
                .width(Size::fill())
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(rect().width(Size::flex(1.)).child(text::line(theme, text::Style::Callout, colour(t.colour_text), who)))
                .child(
                    isle_ui::form::PopUp::new(
                        ACCESS.iter().map(|(_, s)| s.to_string()).collect(),
                        ACCESS.iter().position(|(a, _)| *a == now).unwrap_or(2),
                        "Privilege",
                        move |i: usize| apply(with_access(mode, shift, ACCESS[i].0, dir)),
                    )
                    .enabled(f.mine),
                )
                .into_element()
        })
        .collect();
    if !dir {
        let apply = apply.clone();
        rows.push(
            CheckRow::new("Allow running as a program", mode & 0o100 != 0)
                .on_toggle(move |on: bool| apply(with_running(mode, on)))
                .into_element(),
        );
    }
    if !f.mine {
        rows.push(
            text::line(theme, text::Style::Callout, colour(t.colour_text_2), "You can only read these settings: the item is not yours.")
                .into_element(),
        );
    }
    rows
}

fn summary(theme: &isle_tokens::Theme, many: &[Facts]) -> Element {
    let t = &theme.tokens;
    let kinds: Vec<String> = many.iter().map(|f| list_view::kind(&f.entry)).collect();
    let kind = if kinds.iter().all(|k| *k == kinds[0]) { kinds[0].clone() } else { "Multiple kinds".into() };
    let total: u64 = many.iter().filter(|f| !f.entry.is_dir()).map(|f| f.entry.size).sum();
    let folders = many.iter().filter(|f| f.entry.is_dir()).count();
    let parent = many[0].entry.path.parent().map(PathBuf::from);
    let place = if many.iter().all(|f| f.entry.path.parent().map(PathBuf::from) == parent) {
        parent.map(|p| p.display().to_string()).unwrap_or_default()
    } else {
        "Multiple folders".into()
    };
    let size = if folders > 0 {
        format!("{} for the files; folders are not counted", format::size(total))
    } else {
        format::size(total)
    };
    rect()
        .width(Size::fill())
        .padding(t.space_l)
        .spacing(t.space_xs)
        .child(text::line(theme, text::Style::Headline, colour(t.colour_text), format::items(many.len())))
        .child(rect().height(Size::px(t.space_s)))
        .child(fact(theme, "Kind", kind))
        .child(fact(theme, "Size", size))
        .child(fact(theme, "Where", place))
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_maps_each_class_and_keeps_the_run_bit() {
        assert_eq!(access_of(0o6), Access::ReadWrite);
        assert_eq!(access_of(0o5), Access::ReadOnly);
        assert_eq!(access_of(0o2), Access::NoAccess);
        assert_eq!(with_access(0o755, 3, Access::ReadWrite, false), 0o775);
        assert_eq!(with_access(0o644, 0, Access::NoAccess, false), 0o640);
        assert_eq!(with_access(0o700, 3, Access::ReadOnly, true), 0o750);
        assert_eq!(with_running(0o644, true), 0o755);
        assert_eq!(with_running(0o750, false), 0o640);
        assert_eq!(with_running(0o600, true), 0o700);
    }

    #[test]
    fn a_comment_is_kept_in_the_items_xattr() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "x").unwrap();
        if set_comment(&file, "from the trip").is_err() {
            return; // no user xattrs on this filesystem
        }
        assert_eq!(comment(&file), "from the trip");
        set_comment(&file, "").unwrap();
        assert_eq!(comment(&file), "");
        let facts = read_facts(&file).unwrap();
        assert!(facts.mine);
        assert!(facts.on_disk >= facts.entry.size);
    }
}
