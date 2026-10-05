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
    /// Starts `what`; false when there is nothing to start.
    pub fn launch(&mut self, what: Launch) -> bool {
        let (argv, name) = match &what {
            Launch::App { app, action, uris } => {
                let Some(entry) = self.isle.apps.get(app).cloned() else {
                    warn!(app, "no desktop entry to start");
                    return false;
                };
                let exec = match action {
                    Some(id) => entry
                        .actions
                        .iter()
                        .find(|a| &a.id == id)
                        .and_then(|a| a.exec.clone()),
                    None => entry.exec.clone(),
                };
                let Some(exec) = exec else {
                    warn!(app, ?action, "the entry has nothing to run");
                    return false;
                };
                let Some(mut argv) = isle_apps::exec::command(
                    &exec,
                    uris,
                    &entry.name,
                    entry.icon.as_deref(),
                    &entry.path.to_string_lossy(),
                ) else {
                    warn!(app, exec, "the entry's Exec line does not parse");
                    return false;
                };
                if entry.terminal {
                    argv.splice(0..0, ["xdg-terminal-exec".to_owned()]);
                }
                (argv, app.clone())
            }
            Launch::Open(target) => (vec!["xdg-open".into(), target.clone()], "xdg-open".into()),
        };

        let (token, _) = self
            .isle
            .xdg_activation_state
            .create_external_token(XdgActivationTokenData::default());
        let token = token.to_string();
        let Some(runtime) = self.isle.worker.clone() else {
            return false;
        };
        let unit = format!(
            "app-isle-{}-{:08x}.scope",
            unit_part(&name),
            std::process::id() ^ rand_part()
        );
        info!(?argv, unit, "launching");
        runtime.spawn(async move {
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
                .envs(envs.clone())
                .stdin(std::process::Stdio::null())
                .spawn();
            // systemd-run becomes the app once its scope exists; one that exits at once with
            // a failure found no user manager to ask, and the app runs as Isle's child.
            let failed = match scoped {
                Err(err) => Some(err.to_string()),
                Ok(mut child) => {
                    match tokio::time::timeout(std::time::Duration::from_millis(300), child.wait())
                        .await
                    {
                        Ok(Ok(status)) if !status.success() => {
                            Some(format!("it exited with {status}"))
                        }
                        _ => None,
                    }
                }
            };
            if let Some(err) = failed {
                warn!("systemd-run failed, starting directly: {err}");
                let direct = tokio::process::Command::new(&argv[0])
                    .args(&argv[1..])
                    .envs(envs)
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

fn rand_part() -> u32 {
    use std::hash::{BuildHasher, RandomState};
    RandomState::new().hash_one(std::time::Instant::now()) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_names_keep_only_what_systemd_allows() {
        assert_eq!(unit_part("org.mozilla.firefox"), "org.mozilla.firefox");
        assert_eq!(unit_part("kde-dolphin app"), "kde_dolphin_app");
    }
}
