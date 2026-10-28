//! The context menus: one builder per target, each row the menu bar's command of the same
//! name or an action on the clicked target (`p1/structure/files.md` § Pointer; the board
//! `Files context menus`).

use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use isle_ui::{MenuEntry, MenuItem, menu::Mark};

use crate::{
    places::Place,
    tab::Tab,
    view::{GroupBy, Shown},
};

/// What a row does.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    /// The command table's id, on the menu's items.
    Command(String),
    OpenWith(String),
    OpenInTab(PathBuf),
    OpenInWindow(PathBuf),
    ShowEnclosing(PathBuf),
    RemoveFromSidebar(Place, String),
    Eject(isle_disks::Volume),
    NewTab,
    CloseTab(usize),
    CloseOtherTabs(usize),
    MoveTab(usize),
    Column(Column),
}

/// List's columns that can be hidden; Name always shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Column {
    Modified,
    Size,
    Kind,
}

impl Column {
    pub const ALL: [Column; 3] = [Column::Modified, Column::Size, Column::Kind];

    pub fn name(self) -> &'static str {
        match self {
            Column::Modified => "Date Modified",
            Column::Size => "Size",
            Column::Kind => "Kind",
        }
    }
}

/// A context menu ready to show: its rows, what each does by id, and the items it acts on.
#[derive(Clone, Debug, PartialEq)]
pub struct Open {
    pub at: (f32, f32),
    pub entries: Rc<Vec<MenuEntry>>,
    pub acts: Vec<Act>,
    /// The items its commands act on in place of the selection; none for a target that
    /// has none (a tab, a sidebar row).
    pub items: Option<Vec<PathBuf>>,
    /// Opened from the keyboard, its first row selected.
    pub keyboard: bool,
}

/// Builds rows and their acts together, so a row's id is its act's place.
#[derive(Default)]
struct Rows {
    entries: Vec<MenuEntry>,
    acts: Vec<Act>,
}

impl Rows {
    fn item(&mut self, label: impl Into<String>, act: Act) -> &mut Self {
        self.entries.push(MenuEntry::Item(MenuItem::new(self.acts.len() as u64, label)));
        self.acts.push(act);
        self
    }

    fn command(&mut self, label: impl Into<String>, id: &str) -> &mut Self {
        self.item(label, Act::Command(id.into()))
    }

    fn checked(&mut self, label: impl Into<String>, act: Act, on: bool) -> &mut Self {
        let item = MenuItem::new(self.acts.len() as u64, label);
        self.entries.push(MenuEntry::Item(if on { item.mark(Mark::Check) } else { item }));
        self.acts.push(act);
        self
    }

    fn sub(&mut self, label: impl Into<String>, build: impl FnOnce(&mut Rows)) -> &mut Self {
        let mut inner = Rows { entries: Vec::new(), acts: std::mem::take(&mut self.acts) };
        build(&mut inner);
        self.acts = inner.acts;
        // A submenu's own id past every act, so choosing it runs none.
        let id = u64::MAX - self.entries.len() as u64;
        self.entries.push(MenuEntry::Item(MenuItem::new(id, label).submenu(inner.entries)));
        self
    }

    fn separator(&mut self) -> &mut Self {
        self.entries.push(MenuEntry::Separator);
        self
    }

    fn done(self, at: (f32, f32), items: Option<Vec<PathBuf>>, keyboard: bool) -> Open {
        Open { at, entries: Rc::new(self.entries), acts: self.acts, items, keyboard }
    }
}

fn name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// What Open With's submenu offers for one file: the apps for its type, the default first.
pub type Apps = Option<Vec<crate::open::Choice>>;

/// The menu for `items` in `tab`: one, several, or ones in the Trash.
pub fn items(tab: &Tab, items: Vec<PathBuf>, apps: &Apps, at: (f32, f32), keyboard: bool) -> Open {
    let mut rows = Rows::default();
    let entries: Vec<_> = items.iter().filter_map(|p| tab.folder.get(p)).collect();
    let count = items.len();
    let one = (count == 1).then(|| name(&items[0]));
    if entries.iter().any(|e| e.trashed.is_some()) {
        rows.command("Put Back", "put-back").command("Delete Immediately…", "delete-now").separator();
        rows.command("Get Info", "get-info").command("Quick Look", "quick-look").separator();
        rows.command("Copy", "copy");
        return rows.done(at, Some(items), keyboard);
    }
    let folder = (count == 1).then(|| entries.first().filter(|e| e.is_dir()).map(|e| e.path.clone())).flatten();
    rows.command("Open", "open");
    match &folder {
        Some(path) => {
            rows.item("Open in New Tab", Act::OpenInTab(path.clone()));
            rows.item("Open in New Window", Act::OpenInWindow(path.clone()));
        }
        None if count == 1 => {
            rows.sub("Open With", |sub| {
                if let Some(apps) = apps {
                    for (i, app) in apps.iter().enumerate() {
                        let label = if i == 0 { format!("{} (default)", app.name) } else { app.name.clone() };
                        sub.item(label, Act::OpenWith(app.id.clone()));
                        if i == 0 && apps.len() > 1 {
                            sub.separator();
                        }
                    }
                    sub.separator();
                }
                sub.command("Other…", "open-with");
            });
        }
        None => {}
    }
    rows.separator().command("Move to Trash", "move-to-trash").separator();
    rows.command("Get Info", "get-info");
    match &one {
        Some(name) => {
            rows.command("Rename", "rename");
            rows.command(format!("Compress “{name}”"), "compress");
        }
        None => {
            rows.command(format!("Rename {count} Items…"), "rename");
            rows.command(format!("Compress {count} Items"), "compress");
        }
    }
    rows.command("Duplicate", "duplicate").command("Make Alias", "make-alias");
    rows.command(if one.is_some() { "Quick Look".to_owned() } else { format!("Quick Look {count} Items") }, "quick-look");
    rows.separator();
    match &one {
        Some(name) => rows.command("Copy", "copy").command(format!("Copy “{name}” as Pathname"), "copy-path"),
        None => rows.command(format!("Copy {count} Items"), "copy").command("Copy as Pathnames", "copy-path"),
    };
    if count > 1 {
        rows.separator().command(format!("New Folder with Selection ({count} Items)"), "new-folder-with-items");
    }
    // Isle's extras, where they apply.
    rows.separator();
    if folder.is_some() {
        rows.command("Open in Terminal", "open-terminal");
    }
    let mime = |e: &&isle_fs::Entry| e.mime.clone().unwrap_or_default();
    if count == 1 && entries.first().map(mime).is_some_and(|m| m.starts_with("image/")) {
        rows.command("Set as Wallpaper", "set-wallpaper");
    }
    if count == 1 && one.as_deref().is_some_and(is_archive) {
        rows.command("Expand", "expand");
    }
    rows.done(at, Some(items), keyboard)
}

/// The menu for the folder's background.
pub fn background(tab: &Tab, templates: &[PathBuf], at: (f32, f32), keyboard: bool) -> Open {
    let mut rows = Rows::default();
    let shown = tab.shown;
    if tab.place == Place::Trash {
        rows.command("Empty Trash…", "empty-trash").separator();
        view_rows(&mut rows, &shown, false);
        return rows.done(at, Some(Vec::new()), keyboard);
    }
    if tab.place.folder().is_some() {
        rows.command("New Folder", "new-folder");
        if templates.is_empty() {
            rows.command("New File", "new-file");
        } else {
            rows.sub("New File", |sub| {
                sub.command("Empty File", "new-file");
                for (i, template) in templates.iter().enumerate() {
                    let label = template.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    sub.command(label, &format!("new-file:{i}"));
                }
            });
        }
        rows.separator().command("Get Info", "get-info").separator().command("Paste Item", "paste").separator();
    }
    view_rows(&mut rows, &shown, true);
    if tab.place.folder().is_some() {
        rows.separator().command("Open in Terminal", "open-terminal");
    }
    rows.done(at, Some(Vec::new()), keyboard)
}

fn view_rows(rows: &mut Rows, shown: &Shown, group_by: bool) {
    rows.checked("Use Groups", Act::Command("use-groups".into()), shown.group.is_some());
    if group_by {
        rows.sub("Group By", |sub| {
            for (label, id, by) in [("Kind", "group-kind", GroupBy::Kind), ("Date Modified", "group-modified", GroupBy::Modified), ("Size", "group-size", GroupBy::Size)] {
                sub.checked(label, Act::Command(id.into()), shown.group == Some(by));
            }
        });
    }
    rows.sub("Sort By", |sub| {
        use isle_fs::SortBy;
        for (label, id, by) in [("Name", "sort-name", SortBy::Name), ("Kind", "sort-kind", SortBy::Kind), ("Date Modified", "sort-modified", SortBy::Modified), ("Size", "sort-size", SortBy::Size)] {
            sub.checked(label, Act::Command(id.into()), shown.sort == by);
        }
    });
    rows.command("Show View Options", "view-options");
}

/// The menu for the tab at `index` of `count`.
pub fn tab(index: usize, count: usize, at: (f32, f32)) -> Open {
    let mut rows = Rows::default();
    rows.item("New Tab", Act::NewTab).separator();
    rows.item("Close Tab", Act::CloseTab(index));
    if count > 1 {
        rows.item("Close Other Tabs", Act::CloseOtherTabs(index));
        rows.item("Move Tab to New Window", Act::MoveTab(index));
    }
    rows.done(at, None, false)
}

/// What a sidebar row stands for.
#[derive(Clone, Debug, PartialEq)]
pub enum Side {
    Place(Place, String),
    Volume(isle_disks::Volume),
}

/// The menu for a sidebar row.
pub fn sidebar(side: &Side, at: (f32, f32)) -> Open {
    let mut rows = Rows::default();
    match side {
        Side::Place(Place::Trash, _) => {
            let trash = PathBuf::from(crate::interact::TRASH);
            rows.item("Open in New Tab", Act::OpenInTab(trash.clone()));
            rows.item("Open in New Window", Act::OpenInWindow(trash));
            rows.separator().command("Empty Trash…", "empty-trash");
            return rows.done(at, Some(Vec::new()), false);
        }
        Side::Place(place, label) => {
            if let Some(folder) = place.folder().map(PathBuf::from) {
                rows.item("Open in New Tab", Act::OpenInTab(folder.clone()));
                rows.item("Open in New Window", Act::OpenInWindow(folder.clone()));
                rows.item("Show in Enclosing Folder", Act::ShowEnclosing(folder.clone()));
                rows.separator().command("Get Info", "get-info").separator();
                rows.item("Remove from Sidebar", Act::RemoveFromSidebar(place.clone(), label.clone()));
                return rows.done(at, Some(vec![folder]), false);
            }
            rows.item("Remove from Sidebar", Act::RemoveFromSidebar(place.clone(), label.clone()));
        }
        Side::Volume(volume) => {
            if let Some(mount) = volume.mount_point.as_ref().map(PathBuf::from) {
                rows.item("Open in New Tab", Act::OpenInTab(mount.clone()));
                rows.item("Open in New Window", Act::OpenInWindow(mount.clone()));
                rows.separator().command("Get Info", "get-info");
                if volume.removable || volume.loop_device {
                    rows.separator().item(format!("Eject “{}”", volume.label), Act::Eject(volume.clone()));
                }
                return rows.done(at, Some(vec![mount]), false);
            }
        }
    }
    rows.done(at, None, false)
}

/// The menu for List's column header: which columns show.
pub fn header(hidden: &[Column], at: (f32, f32)) -> Open {
    let mut rows = Rows::default();
    rows.entries.push(MenuEntry::Section("Columns".into()));
    for column in Column::ALL {
        rows.checked(column.name(), Act::Column(column), !hidden.contains(&column));
    }
    rows.done(at, None, false)
}

/// A name Expand takes apart.
fn is_archive(name: &str) -> bool {
    let name = name.to_lowercase();
    [".zip", ".tar", ".tar.gz", ".tgz", ".tar.bz2", ".tar.xz", ".tar.zst", ".7z", ".rar", ".gz", ".xz", ".bz2"]
        .iter()
        .any(|ext| name.ends_with(ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(open: &Open) -> Vec<String> {
        open.entries
            .iter()
            .map(|e| match e {
                MenuEntry::Item(i) => i.label.clone(),
                MenuEntry::Separator => "-".into(),
                MenuEntry::Section(s) => format!("[{s}]"),
            })
            .collect()
    }

    #[test]
    fn a_tab_menu_offers_the_others_only_with_others() {
        assert_eq!(labels(&tab(0, 1, (0., 0.))), ["New Tab", "-", "Close Tab"]);
        let open = tab(1, 3, (0., 0.));
        assert_eq!(labels(&open), ["New Tab", "-", "Close Tab", "Close Other Tabs", "Move Tab to New Window"]);
        let MenuEntry::Item(other) = &open.entries[3] else { panic!() };
        assert_eq!(open.acts[other.id as usize], Act::CloseOtherTabs(1));
    }

    #[test]
    fn a_submenu_row_runs_nothing_and_its_rows_their_acts() {
        let mut rows = Rows::default();
        rows.command("Open", "open").sub("Open With", |sub| {
            sub.item("Editor (default)", Act::OpenWith("editor".into()));
            sub.command("Other…", "open-with");
        });
        let open = rows.done((0., 0.), None, false);
        let MenuEntry::Item(with) = &open.entries[1] else { panic!() };
        assert!(with.id as usize >= open.acts.len());
        let Some(sub) = &with.submenu else { panic!() };
        let ids: Vec<u64> = sub.iter().filter_map(|e| e.item().map(|i| i.id)).collect();
        assert_eq!(ids, [1, 2]);
        assert_eq!(open.acts[2], Act::Command("open-with".into()));
    }

    #[test]
    fn the_header_lists_the_columns_shown_checked() {
        let open = header(&[Column::Size], (0., 0.));
        assert_eq!(labels(&open), ["[Columns]", "Date Modified", "Size", "Kind"]);
        let marks: Vec<bool> = open.entries.iter().filter_map(|e| e.item()).map(|i| i.mark == Mark::Check).collect();
        assert_eq!(marks, [true, false, true]);
    }
}

/// Opens the menu for `target`, an item or the background with none: an item in the
/// selection takes the selection with it, one outside it is acted on alone. A file's Open
/// With fills in once its apps are read.
pub fn open_for(tab: freya::prelude::State<Tab>, target: Option<PathBuf>, at: (f32, f32), keyboard: bool, mut context: freya::prelude::State<Option<Open>>) {
    use freya::prelude::*;
    let t = tab.peek();
    let Some(path) = target else {
        let open = background(&t, &crate::places::templates(), at, keyboard);
        drop(t);
        context.set(Some(open));
        return;
    };
    let chosen = if t.is_selected(&path) { t.selection.clone() } else { vec![path] };
    let file = match chosen.as_slice() {
        [one] => t.folder.get(one).filter(|e| !e.is_dir() && e.trashed.is_none()).map(|e| e.path.clone()),
        _ => None,
    };
    let open = items(&t, chosen.clone(), &None, at, keyboard);
    drop(t);
    context.set(Some(open));
    let Some(file) = file else { return };
    spawn(async move {
        let Some(offer) = crate::window::off_thread(move || crate::open::offer(&file)).await else { return };
        let same = context.peek().as_ref().is_some_and(|o| o.items.as_ref() == Some(&chosen));
        if same {
            let open = items(&tab.peek(), chosen, &Some(offer.apps), at, keyboard);
            context.set(Some(open));
        }
    });
}
