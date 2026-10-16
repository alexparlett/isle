//! Starting apps and opening links: each in a systemd scope of its own, with an activation
//! token so its first window takes focus (`p6-island-and-menus.md` choice 24).

use smithay::wayland::xdg_activation::XdgActivationTokenData;
use tracing::{info, warn};

use crate::state::State;

/// What to start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Launch {
    /// An app by desktop id, or one of its entry's actions, opening `uris`.
    App {
        app: String,
        action: Option<String>,
        uris: Vec<String>,
    },
    /// A link or file in the app that handles it.
    Open(String),
    /// A shell command line, as Spotlight's Run gives it.
    Command(String),
}

/// A unit name's part from `text`: what systemd allows, the rest as `_`.
fn unit_part(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "_.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect()
}

impl State {
    /// Starts `what`; false when there is nothing to start. An entry that is
    /// `DBusActivatable` is asked over the bus first and started by `Exec` only if that fails.
    pub fn launch(&mut self, what: Launch) -> bool {
        let mut activate = None;
        let (argv, name) = match &what {
            Launch::App { app, action, uris } => {
                let Some(entry) = self.isle.apps.get(app).cloned() else {
                    warn!(app, "no desktop entry to start");
                    return false;
                };
                if entry.dbus_activatable {
                    activate = Some(Activation {
                        app: app.clone(),
                        action: action.clone(),
                        uris: uris.clone(),
                    });
                }
                let exec = match action {
                    Some(id) => entry
                        .actions
                        .iter()
                        .find(|a| &a.id == id)
                        .and_then(|a| a.exec.clone()),
                    None => entry.exec.clone(),
                };
                let argv = exec.and_then(|exec| {
                    let argv = isle_apps::exec::command(
                        &exec,
                        uris,
                        &entry.name,
                        entry.icon.as_deref(),
                        &entry.path.to_string_lossy(),
                    );
                    if argv.is_none() {
                        warn!(app, exec, "the entry's Exec line does not parse");
                    }
                    argv
                });
                let argv = match argv {
                    Some(mut argv) => {
                        if entry.terminal {
                            argv.splice(0..0, ["xdg-terminal-exec".to_owned()]);
                        }
                        argv
                    }
                    None if activate.is_some() => Vec::new(),
                    None => {
                        warn!(app, ?action, "the entry has nothing to run");
                        return false;
                    }
                };
                (argv, app.clone())
            }
            Launch::Open(target) => (vec!["xdg-open".into(), target.clone()], "xdg-open".into()),
            Launch::Command(line) => (vec!["sh".into(), "-c".into(), line.clone()], "run".into()),
        };

        let (token, _) = self
            .isle
            .xdg_activation_state
            .create_external_token(XdgActivationTokenData::default());
        let token = token.to_string();
        let Some(runtime) = self.isle.worker.clone() else {
            return false;
        };
        let to_loop = self.isle.to_loop.clone();
        let scope_app = matches!(what, Launch::App { .. }).then(|| name.clone());
        let unit = format!(
            "app-isle-{}-{:08x}.scope",
            unit_part(&name),
            std::process::id() ^ rand_part()
        );
        info!(?argv, unit, "launching");
        // Started in the home folder, as from a terminal, whatever Isle's own folder is.
        let home = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .filter(|h| h.is_dir())
            .unwrap_or_else(|| "/".into());
        runtime.spawn(async move {
            if let Some(activation) = activate {
                match activation.call(&token).await {
                    Ok(()) => return,
                    Err(err) if argv.is_empty() => {
                        warn!(app = activation.app, "could not activate over D-Bus: {err}");
                        return;
                    }
                    Err(err) => warn!(
                        app = activation.app,
                        "could not activate over D-Bus, starting by Exec: {err}"
                    ),
                }
            }
            let envs = [
                ("XDG_ACTIVATION_TOKEN", token.clone()),
                ("DESKTOP_STARTUP_ID", token),
            ];
            let scoped = tokio::process::Command::new("systemd-run")
                .args([
                    "--user",
                    "--scope",
                    "--collect",
                    "--quiet",
                    "--unit",
                    &unit,
                    "--",
                ])
                .args(&argv)
                .envs(crate::child_env::vars())
                .envs(envs.clone())
                .current_dir(&home)
                .stdin(std::process::Stdio::null())
                .spawn();
            // systemd-run becomes the app once its scope exists; one that exits at once with
            // a failure found no user manager to ask, and the app runs as Isle's child.
            let failed = match scoped {
                Err(err) => Some(err.to_string()),
                Ok(mut child) => {
                    let pid = child.id();
                    match tokio::time::timeout(std::time::Duration::from_millis(300), child.wait())
                        .await
                    {
                        Ok(Ok(status)) if !status.success() => {
                            Some(format!("it exited with {status}"))
                        }
                        _ => {
                            // The scope is the app's from here, whatever it forks or execs.
                            let dir = pid.and_then(crate::processes::cgroup_of);
                            if let (Some(dir), Some(app), Some(to_loop)) = (dir, scope_app, to_loop)
                            {
                                if dir.ends_with(&unit) {
                                    let watch = dir.clone();
                                    to_loop.post(move |state| state.track_scope(app, dir));
                                    tokio::spawn(crate::processes::watch_scope(to_loop, watch));
                                }
                            }
                            None
                        }
                    }
                }
            };
            if let Some(err) = failed {
                warn!("systemd-run failed, starting directly: {err}");
                let direct = tokio::process::Command::new(&argv[0])
                    .args(&argv[1..])
                    .envs(crate::child_env::vars())
                    .envs(envs)
                    .current_dir(&home)
                    .stdin(std::process::Stdio::null())
                    .spawn();
                if let Err(err) = direct {
                    warn!(?argv, "could not start: {err}");
                }
            }
        });
        true
    }
}

/// An `org.freedesktop.Application` call for a `DBusActivatable` entry.
struct Activation {
    app: String,
    action: Option<String>,
    uris: Vec<String>,
}

impl Activation {
    /// The object path the spec derives from the desktop id.
    fn path(&self) -> String {
        format!("/{}", self.app.replace('.', "/").replace('-', "_"))
    }

    async fn call(&self, token: &str) -> zbus::Result<()> {
        use std::collections::HashMap;
        use zvariant::Value;
        let conn = isle_bus::session().await?;
        let platform: HashMap<&str, Value> = HashMap::from([
            ("activation-token", Value::from(token)),
            ("desktop-startup-id", Value::from(token)),
        ]);
        let (dest, path, iface) = (
            self.app.as_str(),
            self.path(),
            "org.freedesktop.Application",
        );
        let call = async {
            match (&self.action, self.uris.is_empty()) {
                (Some(action), _) => {
                    let none: Vec<Value> = Vec::new();
                    conn.call_method(
                        Some(dest),
                        path.as_str(),
                        Some(iface),
                        "ActivateAction",
                        &(action, none, &platform),
                    )
                    .await
                }
                (None, false) => {
                    conn.call_method(
                        Some(dest),
                        path.as_str(),
                        Some(iface),
                        "Open",
                        &(&self.uris, &platform),
                    )
                    .await
                }
                (None, true) => {
                    conn.call_method(
                        Some(dest),
                        path.as_str(),
                        Some(iface),
                        "Activate",
                        &(&platform,),
                    )
                    .await
                }
            }
        };
        match tokio::time::timeout(std::time::Duration::from_secs(10), call).await {
            Ok(reply) => reply.map(|_| ()),
            Err(_) => Err(zbus::Error::Failure("no answer in ten seconds".into())),
        }
    }
}

fn rand_part() -> u32 {
    use std::hash::{BuildHasher, RandomState};
    RandomState::new().hash_one(std::time::Instant::now()) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_paths_follow_the_desktop_id() {
        let a = Activation {
            app: "org.gnome.Nautilus".into(),
            action: None,
            uris: Vec::new(),
        };
        assert_eq!(a.path(), "/org/gnome/Nautilus");
        let b = Activation {
            app: "org.example.my-app".into(),
            ..a
        };
        assert_eq!(b.path(), "/org/example/my_app");
    }

    #[test]
    fn unit_names_keep_only_what_systemd_allows() {
        assert_eq!(unit_part("org.mozilla.firefox"), "org.mozilla.firefox");
        assert_eq!(unit_part("kde-dolphin app"), "kde_dolphin_app");
    }
}
