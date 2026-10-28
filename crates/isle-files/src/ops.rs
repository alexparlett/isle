//! What the window's commands and handlers do to tabs, files and volumes.

use std::path::PathBuf;

use freya::prelude::*;
use isle_app::Command;
use isle_fs::jobs::Work;

use crate::{
    Shared,
    places::Place,
    search::{Criterion, Scope, Search},
    sheets::{Alert, Then},
    tab::{self, Tab},
    view::Shown,
    window::*,
};

/// A search of `term` from where the tab is: everything under the folder, or Home from a
/// gathering.
pub(crate) fn search_from(place: &Place, term: String, criteria: Vec<Criterion>) -> Search {
    let folder = place.folder().map(PathBuf::from);
    Search {
        term,
        scope: if folder.is_some() { Scope::Under } else { Scope::Home },
        folder,
        criteria,
        name: None,
    }
}

/// Leaves a search for the folder it began in, back through history when it came from there.
pub(crate) fn end_search(tab: State<Tab>, live: &tab::Live) {
    let Place::Search(search) = tab.peek().place.clone() else { return };
    let Some(folder) = search.folder.clone() else { return };
    let to = Place::Folder(folder);
    if tab.peek().back.last() == Some(&to) {
        tab::back(tab, live);
    } else {
        tab::replace(tab, live, to);
    }
}

/// Goes to a folder in the current tab, from any thread's callback on the window.
pub(crate) fn tab_go(tabs: State<Vec<TabHandle>>, current: State<usize>) -> impl Fn(PathBuf) + Clone + 'static {
    move |folder: PathBuf| {
        if let Some(handle) = tabs.peek().get(*current.peek()).cloned() {
            tab::go(handle.tab, &handle.live, Place::Folder(folder), None);
        }
    }
}

/// Ejects a network share the system mounted: through GIO for its own mounts, else
/// `umount`; a refusal is an alert.
pub(crate) fn unmount_share(share: PathBuf, alert: State<Option<Alert>>) {
    spawn(async move {
        let name = crate::volumes::share_name(&share);
        let done = off_thread(move || {
            let run = |cmd: &str, args: &[&std::ffi::OsStr]| std::process::Command::new(cmd).args(args).output();
            let gio = run("gio", &["mount".as_ref(), "-u".as_ref(), share.as_os_str()]);
            match gio {
                Ok(out) if out.status.success() => Ok(()),
                _ => match run("umount", &[share.as_os_str()]) {
                    Ok(out) if out.status.success() => Ok(()),
                    Ok(out) => Err(String::from_utf8_lossy(&out.stderr).trim().to_owned()),
                    Err(err) => Err(err.to_string()),
                },
            }
        })
        .await;
        if let Some(Err(why)) = done {
            let mut alert = alert;
            alert.set(Some(Alert::message(format!("The disk “{name}” couldn't be ejected."), why)));
        }
    });
}

/// A volume's row clicked: shown when mounted, mounted first otherwise, unlocked first when
/// encrypted, with the keyring's passphrase when it has one.
pub(crate) fn open_volume(
    shared: &Shared,
    volume: isle_disks::Volume,
    unlocking: State<Option<(isle_disks::Volume, Option<String>)>>,
    alert: State<Option<Alert>>,
    go: impl Fn(PathBuf) + 'static,
) {
    if let Some(point) = &volume.mount_point {
        go(PathBuf::from(point));
        return;
    }
    let worker = shared.app().worker();
    let mut mounting = shared.mounting;
    mounting.write().push(volume.path.clone());
    spawn(async move {
        let job = worker.spawn({
            let volume = volume.clone();
            async move {
                if !volume.locked {
                    return Some(isle_disks::mount(&volume.path).await.map_err(|e| e.to_string()));
                }
                let kept = isle_disks::keyring::lookup(&volume.uuid).await.ok().flatten()?;
                let cleartext = isle_disks::unlock(&volume.path, &kept).await.ok()?;
                Some(isle_disks::mount(&cleartext).await.map_err(|e| e.to_string()))
            }
        });
        let (mut unlocking, mut alert) = (unlocking, alert);
        let done = job.await.ok().flatten();
        mounting.write().retain(|p| *p != volume.path);
        match done {
            Some(Ok(point)) => go(PathBuf::from(point)),
            Some(Err(why)) => alert.set(Some(Alert::message(format!("The disk “{}” couldn't be opened.", volume.label), why))),
            None => unlocking.set(Some((volume, None))),
        }
    });
}

/// Unlocks with the passphrase given, keeps it if asked, mounts and shows the volume.
pub(crate) fn unlock(
    shared: &Shared,
    volume: isle_disks::Volume,
    secret: isle_ui::field::Secret,
    remember: bool,
    unlocking: State<Option<(isle_disks::Volume, Option<String>)>>,
    go: impl Fn(PathBuf) + 'static,
) {
    let worker = shared.app().worker();
    let passphrase = secret.expose().to_owned();
    spawn(async move {
        let job = worker.spawn({
            let volume = volume.clone();
            async move {
                let cleartext = isle_disks::unlock(&volume.path, &passphrase).await.map_err(|_| "The password is incorrect.".to_owned())?;
                if remember && let Err(err) = isle_disks::keyring::save(&volume.uuid, &volume.label, &passphrase).await {
                    tracing::warn!("the passphrase is not kept: {err}");
                }
                isle_disks::mount(&cleartext).await.map_err(|e| e.to_string())
            }
        });
        let mut unlocking = unlocking;
        match job.await {
            Ok(Ok(point)) => {
                unlocking.set(None);
                go(PathBuf::from(point));
            }
            Ok(Err(why)) => unlocking.set(Some((volume, Some(why)))),
            Err(_) => unlocking.set(None),
        }
    });
}

/// Ejects a volume; one in use asks, with Force Eject.
pub(crate) fn eject(shared: &Shared, volume: isle_disks::Volume, force: bool, alert: State<Option<Alert>>) {
    let worker = shared.app().worker();
    spawn(async move {
        let job = worker.spawn({
            let volume = volume.clone();
            async move { isle_disks::eject_with(&volume, force).await.map_err(|e| (isle_disks::is_busy(&e), e.to_string())) }
        });
        let mut alert = alert;
        match job.await {
            Ok(Ok(())) => {}
            Ok(Err((true, _))) => alert.set(Some(Alert::in_use(volume))),
            Ok(Err((false, why))) => alert.set(Some(Alert::message(format!("The disk “{}” couldn't be ejected.", volume.label), why))),
            Err(_) => {}
        }
    });
}

/// A command a field takes as its key while a name is being edited.
pub(crate) fn field_when(command: Command, editing: bool) -> Command {
    if editing { command.field() } else { command }
}

pub(crate) fn same_volume(a: &std::path::Path, b: &std::path::Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let dev = |p: &std::path::Path| p.ancestors().find_map(|a| std::fs::symlink_metadata(a).ok()).map(|m| m.dev());
    dev(a).is_some() && dev(a) == dev(b)
}

/// Return on a selection: one item's name opens for editing, several open Rename Many.
pub(crate) fn start_rename(
    selection: Vec<PathBuf>,
    mut renaming: State<Option<PathBuf>>,
    mut text: State<String>,
    mut error: State<Option<String>>,
    mut many: State<Option<Vec<PathBuf>>>,
) {
    match selection.as_slice() {
        [] => {}
        [one] => {
            text.set(one.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
            error.set(None);
            renaming.set(Some(one.clone()));
        }
        _ => many.set(Some(selection)),
    }
}

pub(crate) enum RenameCheck {
    Cancel,
    Refused(String),
    Ask(Alert),
    Go(Work),
}

/// What a typed name does (`files.md` § Acting on files, Rename).
pub(crate) fn check_rename(tab: &Tab, path: &std::path::Path, typed: &str, warn_extension: bool) -> RenameCheck {
    let is_dir = tab.folder.get(path).is_some_and(|e| e.is_dir());
    check_rename_with(path, typed, is_dir, |p| tab.folder.get(p).is_some(), warn_extension)
}

/// As `check_rename`, `taken` saying whether a name is in use: Get Info asks the disk.
pub(crate) fn check_rename_with(
    path: &std::path::Path,
    typed: &str,
    is_dir: bool,
    taken: impl Fn(&std::path::Path) -> bool,
    warn_extension: bool,
) -> RenameCheck {
    let name = typed.trim();
    let old = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if name.is_empty() || name == old {
        return RenameCheck::Cancel;
    }
    if name.contains('/') {
        return RenameCheck::Refused("A name can't contain “/”.".into());
    }
    if taken(&path.with_file_name(name)) {
        return RenameCheck::Refused(format!("The name “{name}” is already taken. Please choose a different name."));
    }
    let to = |n: &str| Work::Rename { item: path.to_owned(), to: n.to_owned() };
    if name.starts_with('.') && !old.starts_with('.') {
        return RenameCheck::Ask(Alert {
            title: "Are you sure you want to use a name that begins with a dot?".into(),
            message: "Names that begin with a dot are hidden.".into(),
            items: Vec::new(),
            buttons: vec![("Cancel".into(), None), (format!("Use “{name}”"), Some(Then::Job(to(name))))],
        });
    }
    let (_, old_ext) = isle_fs::jobs::split_ext(&old);
    let (_, new_ext) = isle_fs::jobs::split_ext(name);
    if warn_extension && !is_dir && !old_ext.is_empty() && new_ext != old_ext {
        let kept = format!("{}{old_ext}", isle_fs::jobs::split_ext(name).0);
        return RenameCheck::Ask(Alert {
            title: format!("Are you sure you want to change the extension from “{old_ext}” to “{new_ext}”?"),
            message: "If you make this change, your document may open in a different application.".into(),
            items: Vec::new(),
            buttons: vec![(format!("Use {new_ext}"), Some(Then::Job(to(name)))), (format!("Keep {old_ext}"), Some(Then::Job(to(&kept))))],
        });
    }
    RenameCheck::Go(to(name))
}

/// Starts a job, or says first that it will not fit.
pub(crate) fn start_job(shared: &Shared, key: isle_app::WindowKey, work: Work, alert: &mut State<Option<Alert>>) {
    let probe = work.clone();
    let short = isle_fs::jobs::short_of_space(&probe);
    if let Some((needed, free, _)) = short {
        let items: &[PathBuf] = match &probe {
            Work::Copy { items, .. } | Work::Move { items, .. } | Work::Duplicate { items } => items,
            _ => &[],
        };
        let what = match items {
            [one] => format!("“{}”", crate::sheets::name(one)),
            _ => "The items".into(),
        };
        alert.set(Some(Alert::no_space(&what, crate::jobs::verb(&work), needed.saturating_sub(free))));
        return;
    }
    let (title, undo_label) = crate::jobs::title(&work);
    shared.jobs.start(&shared.app(), key, work, title, undo_label);
}

/// Everything in every trash; reads them, so call it off the window's thread.
pub(crate) fn trash_items() -> Vec<isle_fs::trash::Trashed> {
    isle_fs::trash::all_trashes().iter().flat_map(isle_fs::trash::list).collect()
}

/// Asks before emptying, once the Trash is counted.
pub(crate) fn ask_empty_trash(alert: State<Option<Alert>>) {
    spawn(async move {
        if let Some(items) = off_thread(trash_items).await {
            let mut alert = alert;
            alert.set(Some(Alert::empty_trash(items)));
        }
    });
}

/// A link to an item beside it, "{name} alias" (`files.md` § Acting on files).
pub(crate) fn make_alias(item: &std::path::Path) {
    let (Some(dir), Some(name)) = (item.parent(), item.file_name()) else { return };
    let link = isle_fs::jobs::free_name(dir, &format!("{} alias", name.to_string_lossy()), false);
    if let Err(err) = std::os::unix::fs::symlink(item, &link) {
        tracing::warn!("no alias of {}: {err}", item.display());
    }
}

/// A view changed in a tab is its folder's from now on.
pub(crate) fn set_shown(shared: &Shared, mut tab: State<Tab>, next: Shown) {
    let place = {
        let mut t = tab.write();
        t.shown = next;
        t.folder.set_order(next.order());
        t.rev += 1;
        t.place.clone()
    };
    shared.keep_shown(&place, next);
}

pub(crate) fn open_new_tab(
    shared: &Shared,
    mut tabs: State<Vec<TabHandle>>,
    mut current: State<usize>,
    key: isle_app::WindowKey,
    place: Place,
) {
    let Some(scope) = shared.window_scope(key) else { return };
    let handle = new_tab(shared, place, None, scope);
    tab::load(handle.tab, &handle.live, false);
    let n = {
        let mut list = tabs.write();
        list.push(handle);
        list.len() - 1
    };
    current.set(n);
    shared.remember(key, &tabs.peek(), n);
}

/// Opens items of a tab: one folder in place, several (or ⌘) in tabs.
pub(crate) fn open_items(
    shared: &Shared,
    tabs: State<Vec<TabHandle>>,
    current: State<usize>,
    key: isle_app::WindowKey,
    handle: &TabHandle,
    paths: Vec<PathBuf>,
    in_tabs: bool,
    open_with: State<Option<crate::open::Offer>>,
) {
    // An item not in the tab's listing yet, as one opened from a column before the last.
    let entries: Vec<_> = {
        let t = handle.tab.peek();
        paths.iter().filter_map(|p| t.folder.get(p).cloned().or_else(|| isle_fs::Entry::at(p).ok())).collect()
    };
    let dirs: Vec<_> = entries.iter().filter(|e| e.is_dir()).collect();
    match (dirs.as_slice(), in_tabs) {
        ([one], false) => tab::go(handle.tab, &handle.live, Place::Folder(one.path.clone()), None),
        // ⌘-double-click opens a window instead while General's tabs setting is off.
        (many, true) if !shared.prefs().open_folders_in_tabs => {
            for dir in many {
                shared.open_window(vec![(Place::Folder(dir.path.clone()), None)], 0, None);
            }
        }
        (many, _) => {
            for dir in many {
                open_new_tab(shared, tabs, current, key, Place::Folder(dir.path.clone()));
            }
        }
    }
    // An item in the Trash is put back or dragged out to be opened.
    let files: Vec<PathBuf> = entries.iter().filter(|e| !e.is_dir() && e.trashed.is_none()).map(|e| e.path.clone()).collect();
    if !files.is_empty() {
        open_files(shared, key, files, open_with);
    }
}

/// Opens files in their apps, expanding an archive with no default app and asking with Open
/// With for the first other file nothing opens.
pub(crate) fn open_files(shared: &Shared, key: isle_app::WindowKey, files: Vec<PathBuf>, open_with: State<Option<crate::open::Offer>>) {
    let (app, jobs) = (shared.app(), shared.jobs);
    let bus = app.bus();
    spawn(async move {
        let Some((launches, ask)) = off_thread(move || crate::open::decide(&files)).await else { return };
        if let Some(bus) = bus {
            for (app, uris) in launches {
                crate::open::launch(bus.clone(), app, uris).await;
            }
        }
        let (archives, ask): (Vec<_>, Vec<_>) = ask.into_iter().partition(|p| crate::menus::is_archive(&p.to_string_lossy()));
        for archive in archives {
            let work = Work::Expand { archive };
            let (title, undo_label) = crate::jobs::title(&work);
            jobs.start(&app, key, work, title, undo_label);
        }
        if let Some(first) = ask.into_iter().next() {
            ask_open_with(first, open_with);
        }
    });
}

/// Shows Open With for one file, once its apps are read.
pub(crate) fn ask_open_with(path: PathBuf, open_with: State<Option<crate::open::Offer>>) {
    spawn(async move {
        if let Some(offer) = off_thread(move || crate::open::offer(&path)).await {
            tracing::info!(path = %offer.path.display(), kind = offer.kind, apps = offer.apps.len(), "open with");
            let mut open_with = open_with;
            open_with.set(Some(offer));
        }
    });
}

pub(crate) fn close_tab(mut tabs: State<Vec<TabHandle>>, mut current: State<usize>, i: usize) {
    let len = tabs.peek().len();
    if len <= 1 || i >= len {
        return;
    }
    tabs.write().remove(i);
    let now = *current.peek();
    if now >= i && now > 0 {
        current.set(now - 1);
    }
}
