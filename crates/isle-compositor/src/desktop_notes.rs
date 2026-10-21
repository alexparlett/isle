//! The desktop's own notifications, posted through the same decision as any app's: a drive
//! connected, a rejected hand edit, Isle's last crash (`p8-services.md` choice 20).

use isle_bus::Snapshot;
use isle_disks::{Disks, Volume};
use isle_notifications::Notification;

use crate::{
    launch::Launch,
    state::{Isle, State},
};

/// Ids for the desktop's own, apart from the server's.
const FIRST: u32 = 0x8000_0000;

/// What a desktop notification's actions act on.
#[derive(Clone, Debug)]
pub enum Note {
    Drive(String),
    SettingsError,
    Crash(std::path::PathBuf),
    Capture,
}

impl Isle {
    fn volume(&self, path: &str) -> Option<Volume> {
        self.services
            .disks
            .state
            .volumes
            .iter()
            .find(|v| v.path == path)
            .cloned()
    }
}

impl State {
    /// Posts a notification of Isle's own.
    pub fn post_desktop(
        &mut self,
        app: &str,
        app_name: &str,
        summary: String,
        body: String,
        actions: Vec<(&str, &str)>,
        note: Note,
    ) {
        let id = FIRST + self.isle.notifications.desktop_next;
        self.isle.notifications.desktop_next += 1;
        self.isle.notifications.desktop.insert(id, note);
        self.notification_incoming(isle_notifications::Incoming::Notify(Box::new(
            Notification {
                id,
                app_name: app_name.into(),
                summary,
                body,
                actions: actions
                    .into_iter()
                    .map(|(k, l)| (k.to_owned(), l.to_owned()))
                    .collect(),
                urgency: 1,
                desktop_entry: Some(app.to_owned()).filter(|a| !a.is_empty()),
                expire: -1,
                at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs()),
                ..Default::default()
            },
        )));
    }

    /// One of the desktop's own notifications was acted on.
    pub fn desktop_action(&mut self, id: u32, key: &str) {
        let Some(note) = self.isle.notifications.desktop.get(&id).cloned() else {
            return;
        };
        match (note, key) {
            (Note::Drive(path), "eject") => {
                if let Some(volume) = self.isle.volume(&path) {
                    self.isle
                        .run_service(async move { isle_disks::eject(&volume).await }, |_, _| {});
                }
            }
            (Note::Drive(path), "mount") => {
                self.isle.run_service(
                    async move { isle_disks::mount(&path).await.map(drop) },
                    |_, _| {},
                );
            }
            (Note::Drive(path), _) => {
                let mounted = self.isle.volume(&path).and_then(|v| v.mount_point);
                self.isle.run_service(
                    async move {
                        match mounted {
                            Some(point) => Ok(point),
                            None => isle_disks::mount(&path).await,
                        }
                    },
                    |state, point| {
                        if let Ok(point) = point {
                            state.launch(Launch::Open(point));
                        }
                    },
                );
            }
            (Note::SettingsError, _) => self.notice_action("editor"),
            (Note::Crash(record), _) => {
                self.launch(Launch::Open(record.to_string_lossy().into_owned()));
            }
            (Note::Capture, _) => {}
        }
    }

    pub fn disks_changed(&mut self, snapshot: Snapshot<Disks>) {
        let first = !self.isle.services.disks.available;
        let before: Vec<String> = self
            .isle
            .services
            .disks
            .state
            .volumes
            .iter()
            .map(|v| v.path.clone())
            .collect();
        self.isle.services.disks = snapshot;
        if first {
            return;
        }
        let new: Vec<Volume> = self
            .isle
            .services
            .disks
            .state
            .volumes
            .iter()
            .filter(|v| !before.contains(&v.path) && (v.removable || v.loop_device))
            .cloned()
            .collect();
        for volume in new {
            let mut actions = vec![("default", "Open")];
            if volume.mount_point.is_none() {
                actions.push(("mount", "Mount"));
            }
            actions.push(("eject", "Eject"));
            self.post_desktop(
                "org.gnome.Nautilus",
                "Files",
                format!("{} connected", volume.label),
                String::new(),
                actions,
                Note::Drive(volume.path.clone()),
            );
        }
    }

    /// A hand edit of `settings.kdl` was rejected: said once per broken state of the file.
    pub fn settings_error_note(&mut self) {
        let Some((file, line, column, _)) = self.isle.settings_error.clone() else {
            self.isle.notifications.settings_noted = None;
            return;
        };
        let key = format!("{file}:{line}:{column}");
        if self.isle.notifications.settings_noted.as_ref() == Some(&key) {
            return;
        }
        self.isle.notifications.settings_noted = Some(key);
        self.post_desktop(
            "",
            "Settings",
            format!(
                "settings.kdl has an error on line {line}, column {column}. Changes won't be saved until it's fixed."
            ),
            String::new(),
            vec![("default", "Open in editor")],
            Note::SettingsError,
        );
    }

    /// Isle's last crash, said once at the next start.
    pub fn crash_note(&mut self) {
        let Ok(entries) = std::fs::read_dir(crate::crash::crashes_dir()) else {
            return;
        };
        let newest = entries
            .flatten()
            .filter(|e| e.path().is_file())
            .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
        let Some(newest) = newest else { return };
        let name = newest.file_name().to_string_lossy().into_owned();
        if self.isle.notifications.last_crash.as_ref() == Some(&name) {
            return;
        }
        self.isle.notifications.last_crash = Some(name);
        self.post_desktop(
            "",
            "Isle",
            "Isle quit unexpectedly.".into(),
            String::new(),
            vec![("default", "Report…")],
            Note::Crash(newest.path()),
        );
    }
}
