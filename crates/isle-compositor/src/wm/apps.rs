//! The desktop in apps: the entries index, the close path with keep-running, quit, hide,
//! minimise, the front app's window cycle, and pings (`compositor.md` Apps 1–11, B20, B30).

use std::{error::Error, sync::Arc, time::Duration};

use isle_apps::Index;
use notify::{RecursiveMode, Watcher};
use smithay::{
    reexports::calloop::timer::{TimeoutAction, Timer},
    utils::SERIAL_COUNTER,
    wayland::shell::xdg::ShellClient,
};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::{
    state::{Isle, State},
    worker::{Event, ToLoop},
};

/// How long a client has to answer a ping before it is marked not responding.
const PING_TIMEOUT: Duration = Duration::from_secs(3);

/// Reads the desktop entries, and again whenever an applications directory changes; first
/// registers the AppImages in `~/Applications`, and again whenever that folder changes.
pub async fn watch(to_loop: ToLoop) -> Result<(), Box<dyn Error + Send + Sync>> {
    let dirs = Index::data_dirs();
    let images = isle_apps::appimage::apps_dir();
    let data_home = isle_apps::appimage::data_home();
    let scan = |dirs: Vec<std::path::PathBuf>| {
        let (images, data_home) = (images.clone(), data_home.clone());
        async move {
            tokio::task::spawn_blocking(move || {
                if let (Some(images), Some(home)) = (images, data_home) {
                    let synced = isle_apps::appimage::sync(
                        &images,
                        &home.join("applications"),
                        &home.join("icons/hicolor"),
                    );
                    if synced != Default::default() {
                        info!(?synced, "AppImages registered again");
                    }
                }
                Index::scan(&dirs)
            })
            .await
            .unwrap_or_default()
        }
    };
    if let Some(images) = &images {
        let _ = std::fs::create_dir_all(images);
    }
    let index = scan(dirs.clone()).await;
    info!(entries = index.entries().len(), "desktop entries read");
    to_loop.post(move |state| state.set_apps(Arc::new(index)));

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_ok() {
            let _ = tx.send(());
        }
    })?;
    for dir in &dirs {
        if dir.is_dir() {
            let _ = watcher.watch(dir, RecursiveMode::Recursive);
        }
    }
    if let Some(images) = images.as_ref().filter(|d| d.is_dir()) {
        let _ = watcher.watch(images, RecursiveMode::NonRecursive);
    }
    while rx.recv().await.is_some() {
        // A package install writes many files, and a download grows for a while; read them
        // once it settles.
        tokio::time::sleep(Duration::from_secs(1)).await;
        while rx.try_recv().is_ok() {}
        let index = scan(dirs.clone()).await;
        // Registering an AppImage writes an entry, which wakes this again; let that settle.
        while rx.try_recv().is_ok() {}
        debug!(
            entries = index.entries().len(),
            "desktop entries read again"
        );
        to_loop.post(move |state| state.set_apps(Arc::new(index)));
    }
    Ok(())
}

impl Isle {
    /// The app of the focused window.
    pub fn front_app(&self) -> Option<String> {
        let focus = self.seat.get_keyboard()?.current_focus()?;
        Some(self.wm.for_surface(focus.surface()?)?.app.clone())
    }

    /// The app a window counts under: its own, or for a dialog on another app's window, as
    /// Files' panels are, the app it hangs from (`p15-files.md` choice 23).
    pub fn owner_app<'a>(&'a self, record: &'a super::Record) -> &'a str {
        let mut top = record;
        for _ in 0..8 {
            match top.parent.and_then(|p| self.wm.get(p)) {
                Some(parent) => top = parent,
                None => break,
            }
        }
        &top.app
    }

    /// Apps with a window, most recently used first.
    pub fn apps_by_recency(&self) -> Vec<String> {
        let mut apps: Vec<String> = Vec::new();
        let recent = self.wm.recent.iter().filter_map(|id| self.wm.get(*id));
        let rest = self.wm.records.iter().rev();
        for record in recent.chain(rest) {
            let app = self.owner_app(record);
            if record.mapped && !apps.iter().any(|a| a == app) {
                apps.push(app.to_owned());
            }
        }
        apps
    }

    /// `app`'s icon at `size` points, in Papirus for the appearance in force; a game's cover.
    pub fn app_icon(&self, app: &str, size: u32) -> Option<std::path::PathBuf> {
        if let Some(cover) = self.games.for_app(app).and_then(|g| g.cover.clone()) {
            return Some(cover);
        }
        let name = self.apps.get(app)?.icon.clone()?;
        let dark = self.shell.theme().appearance.mode == isle_tokens::Mode::Dark;
        let mut icons = self.icons.borrow_mut();
        if icons.as_ref().is_none_or(|(d, _)| *d != dark) {
            *icons = Some((dark, isle_apps::icons::Icons::papirus(dark)));
        }
        icons.as_mut().unwrap().1.lookup(&name, size)
    }

    /// Apps with no window whose process still runs, most recently windowless first.
    pub fn background_apps(&self) -> Vec<String> {
        self.processes
            .seen()
            .filter(|app| !self.wm.records.iter().any(|r| r.mapped && self.owner_app(r) == app.as_str()))
            .cloned()
            .collect()
    }

    /// An app's toplevels in the one order Cmd+` and the switcher's window row share: the
    /// order they opened, dialogs aside.
    pub fn app_windows(&self, app: &str) -> Vec<u64> {
        let mut ids: Vec<u64> = self
            .wm
            .records
            .iter()
            .filter(|r| r.mapped && r.app == app && r.parent.is_none() && !r.parked)
            .map(|r| r.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    pub fn app_infos(&self) -> Vec<isle_ipc::AppInfo> {
        let background = self.background_apps();
        let apps = self.apps_by_recency();
        apps.into_iter()
            .map(|app| (app, false))
            .chain(background.into_iter().map(|app| (app, true)))
            .map(|(app, background)| {
                let windows: Vec<&super::Record> = self
                    .wm
                    .records
                    .iter()
                    .filter(|r| r.mapped && self.owner_app(r) == app && !r.parked)
                    .collect();
                let toplevels = windows.iter().filter(|r| r.parent.is_none());
                let entry = self.badges.get(&app);
                isle_ipc::AppInfo {
                    name: self.app_name(&app),
                    hidden: windows.iter().any(|r| r.hidden),
                    windows: toplevels.clone().count() as u32,
                    attention: windows.iter().any(|r| r.attention)
                        || entry.is_some_and(|e| e.urgent),
                    urgent: windows.iter().any(|r| r.urgent) || entry.is_some_and(|e| e.urgent),
                    not_responding: windows.iter().any(|r| r.not_responding),
                    minimised: toplevels.clone().next().is_some()
                        && toplevels.clone().all(|r| r.minimised),
                    background,
                    badge: entry.and_then(|e| e.badge()).unwrap_or(-1),
                    progress: entry.and_then(|e| e.progress()).unwrap_or(-1.0),
                    icon: self
                        .app_icon(&app, 128)
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    app,
                }
            })
            .collect()
    }

    fn keeps_running(&self, app: &str) -> bool {
        self.settings
            .windows
            .keep_running
            .iter()
            .any(|k| k == app || self.apps.resolve(k) == app)
    }
}

impl State {
    pub fn set_apps(&mut self, index: Arc<Index>) {
        self.isle.apps = index;
        let ids: Vec<u64> = self.isle.wm.records.iter().map(|r| r.id).collect();
        for id in ids {
            self.isle.refresh_identity(id);
        }
        self.isle.notify(Event::WindowsChanged);
    }

    /// The close path of Cmd+W, the close light, the switcher and Mission Control: a
    /// keep-running app's last window hides the app; any other window is asked to close.
    pub fn close_window(&mut self, id: u64) -> bool {
        let Some(record) = self.isle.wm.get(id) else {
            return false;
        };
        let app = record.app.clone();
        let last = !self
            .isle
            .wm
            .records
            .iter()
            .any(|r| r.id != id && r.mapped && r.app == app);
        if last && record.parent.is_none() && self.isle.keeps_running(&app) {
            info!(app, "last window of a keep-running app: hidden");
            return self.set_app_hidden(&app, true);
        }
        if record.not_responding {
            self.show_alert(id);
            return true;
        }
        record.send_close();
        self.ping(id);
        true
    }

    /// Cmd+Q: every window of the app is asked to close, and once they are gone its processes
    /// still running get the polite signal, keep-running apps included (B30).
    pub fn quit_app(&mut self, app: &str) -> bool {
        if self.quit_steam(app) {
            return true;
        }
        let ids: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| r.app == app)
            .map(|r| r.id)
            .collect();
        if ids.is_empty() {
            return false;
        }
        let pids: Vec<i32> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| r.app == app)
            .filter_map(|r| r.pid)
            .filter(|pid| *pid > 1 && *pid != std::process::id() as i32)
            .collect();
        info!(app, ?pids, "quit");
        self.isle.quitting.retain(|(a, _)| a != app);
        self.isle.quitting.push((app.to_owned(), pids));
        for id in ids {
            if let Some(record) = self.isle.wm.get(id) {
                record.send_close();
            }
            self.ping(id);
        }
        true
    }

    /// Force Quit: every process of the app is killed, its windows' and its launch scope's.
    pub fn force_quit_app(&mut self, app: &str) -> bool {
        let mut pids = self.isle.processes.pids(app);
        pids.extend(
            self.isle
                .wm
                .records
                .iter()
                .filter(|r| r.app == app)
                .filter_map(|r| r.pid),
        );
        pids.sort_unstable();
        pids.dedup();
        pids.retain(|pid| *pid > 1 && *pid != std::process::id() as i32);
        info!(app, ?pids, "force quit");
        let mut all = true;
        for pid in pids {
            // Safety: a signal to a process this app's windows or scope belong to.
            if unsafe { libc::kill(pid, libc::SIGKILL) } != 0 {
                warn!(app, pid, "couldn't force quit: {}", std::io::Error::last_os_error());
                all = false;
            }
        }
        all
    }

    /// After a window went: an app being quit whose windows are all gone gets its signal.
    pub fn finish_quits(&mut self) {
        let records = &self.isle.wm.records;
        let (done, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut self.isle.quitting)
            .into_iter()
            .partition(|(app, _)| !records.iter().any(|r| &r.app == app));
        self.isle.quitting = waiting;
        for (app, mut pids) in done {
            pids.sort_unstable();
            pids.dedup();
            for pid in pids {
                // Safety: kill with signal 0 only asks whether the process exists.
                if unsafe { libc::kill(pid, 0) } == 0 {
                    debug!(app, pid, "still running after its windows closed: SIGTERM");
                    unsafe { libc::kill(pid, libc::SIGTERM) };
                }
            }
        }
    }

    /// Cmd+Option+H: hides every app but the front one.
    pub fn hide_others(&mut self) {
        let front = self.isle.front_app();
        let others: Vec<String> = self
            .isle
            .apps_by_recency()
            .into_iter()
            .filter(|a| Some(a) != front.as_ref())
            .collect();
        for app in others {
            self.set_app_hidden(&app, true);
        }
    }

    /// Cmd+M on the focused window; refused for a dialog or a full-screen window.
    pub fn minimise_focused(&mut self) {
        let Some(id) = self.focused_window() else {
            return;
        };
        if self.isle.wm.get(id).is_some_and(|r| r.parent.is_none()) {
            self.minimise(id);
        }
    }

    /// Cmd+Option+M: every window of the front app.
    pub fn minimise_app(&mut self) {
        let Some(app) = self.isle.front_app() else {
            return;
        };
        let ids: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| r.app == app && r.parent.is_none() && self.isle.wm.is_shown(r.id))
            .map(|r| r.id)
            .collect();
        for id in ids {
            self.minimise(id);
        }
    }

    /// Shows `app`'s windows, raises them in their order and focuses its most recent one.
    pub fn activate_app(&mut self, app: &str) -> bool {
        if !self
            .isle
            .wm
            .records
            .iter()
            .any(|r| r.mapped && r.app == app)
        {
            return false;
        }
        self.set_app_hidden(app, false);
        let shown: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| r.app == app && r.shown())
            .map(|r| r.id)
            .collect();
        for id in &shown {
            self.isle.wm.raise(*id);
        }
        let target = self
            .isle
            .wm
            .recent
            .iter()
            .copied()
            .find(|id| shown.contains(id))
            .or_else(|| shown.last().copied())
            .or_else(|| {
                self.isle
                    .wm
                    .records
                    .iter()
                    .find(|r| r.mapped && r.app == app)
                    .map(|r| r.id)
            });
        match target {
            Some(id) => self.activate(id),
            None => false,
        }
    }

    /// Cmd+`: the front app's next shown window, in the order they opened, not by recency.
    pub fn cycle_app_windows(&mut self, forward: bool) {
        let Some(app) = self.isle.front_app() else {
            return;
        };
        let focused = self.focused_window();
        let ids: Vec<u64> = self
            .isle
            .app_windows(&app)
            .into_iter()
            .filter(|id| self.isle.wm.is_shown(*id))
            .collect();
        if ids.len() < 2 {
            return;
        }
        let at = focused
            .and_then(|f| ids.iter().position(|i| *i == f))
            .unwrap_or(0);
        let next = if forward {
            ids[(at + 1) % ids.len()]
        } else {
            ids[(at + ids.len() - 1) % ids.len()]
        };
        self.activate(next);
    }

    /// Pings the client of `id`; one that does not answer in time is marked not responding.
    pub fn ping(&mut self, id: u64) {
        let Some(client) = self
            .isle
            .wm
            .get(id)
            .and_then(|r| r.window.toplevel())
            .map(|t| t.client())
        else {
            return;
        };
        if let Err(err) = client.send_ping(SERIAL_COUNTER.next_serial()) {
            debug!(id, "no ping: {err:?}");
            return;
        }
        let token = self.isle.loop_handle.insert_source(
            Timer::from_duration(PING_TIMEOUT),
            move |_, _, state| {
                state.ping_timed_out(id);
                TimeoutAction::Drop
            },
        );
        if let Err(err) = token {
            warn!("no ping timer: {err}");
        }
        self.isle.pings.push((client, id));
    }

    fn ping_timed_out(&mut self, id: u64) {
        let Some(index) = self.isle.pings.iter().position(|(_, i)| *i == id) else {
            return;
        };
        let (client, _) = self.isle.pings.remove(index);
        if !client.alive() {
            return;
        }
        info!(id, "not responding");
        self.set_not_responding(&client, true);
    }

    pub fn pong(&mut self, client: ShellClient) {
        self.isle.pings.retain(|(c, _)| *c != client);
        self.set_not_responding(&client, false);
    }

    fn set_not_responding(&mut self, client: &ShellClient, on: bool) {
        let mut changed = Vec::new();
        for record in &mut self.isle.wm.records {
            if record
                .window
                .toplevel()
                .is_some_and(|t| t.client() == *client)
                && record.not_responding != on
            {
                record.not_responding = on;
                changed.push(record.id);
            }
        }
        if !on {
            self.alerts_answered(&changed);
        }
        if !changed.is_empty() {
            self.isle.notify(Event::WindowsChanged);
            self.refresh_pointer_cursor();
        }
    }

    /// The busy pointer over a window that is not responding; the client's own otherwise.
    pub fn refresh_pointer_cursor(&mut self) {
        let pointer = self.isle.seat.get_pointer().unwrap();
        let over = pointer
            .current_focus()
            .and_then(|f| f.surface().cloned())
            .and_then(|s| {
                let mut root = s;
                while let Some(parent) = smithay::wayland::compositor::get_parent(&root) {
                    root = parent;
                }
                self.isle.wm.for_surface(&root).map(|r| r.not_responding)
            })
            .unwrap_or(false);
        use smithay::input::pointer::{CursorIcon, CursorImageStatus};
        let busy = CursorImageStatus::Named(CursorIcon::Wait);
        if over && self.isle.cursor_status != busy {
            self.isle.cursor_before_busy = Some(self.isle.cursor_status.clone());
            self.isle.cursor_status = busy;
        } else if !over && let Some(before) = self.isle.cursor_before_busy.take() {
            self.isle.cursor_status = before;
        } else {
            return;
        }
        self.backend.update_cursor(&self.isle);
        self.isle.queue_redraw_all();
    }
}
