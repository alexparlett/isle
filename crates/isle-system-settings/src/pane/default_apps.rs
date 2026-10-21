//! Apps › Default apps: the web browser, mail, text editor and terminal, each a pop-up of the
//! apps that handle it (`p15-files.md` choice 34).

use freya::prelude::*;
use isle_apps::{Index, mimeapps};
use isle_settings::Value;
use isle_ui::form;

use crate::{nav::Location, ui::Rows};

/// A row: its id, label, and the types its app becomes the default for.
const KINDS: [(&str, &str, &[&str]); 3] = [
    ("default-browser", "Web browser", &["x-scheme-handler/https", "x-scheme-handler/http", "text/html"]),
    ("default-mail", "Mail", &["x-scheme-handler/mailto"]),
    ("default-editor", "Text editor", &["text/plain"]),
];

/// What each row offers: its apps by id and name, the default first when there is one.
#[derive(Clone, Debug, Default, PartialEq)]
struct Offers {
    rows: Vec<Vec<(String, String)>>,
    /// Terminals, with the command each runs.
    terminals: Vec<(String, String)>,
}

/// An entry's Exec without its field codes, as a command line to run in a folder.
fn command(exec: &str) -> String {
    exec.split_whitespace().filter(|w| !w.starts_with('%')).collect::<Vec<_>>().join(" ")
}

fn read() -> Offers {
    let index = Index::scan(&Index::data_dirs());
    let lists = mimeapps::read_lists();
    let rows = KINDS
        .iter()
        .map(|(_, _, kinds)| {
            mimeapps::apps_for(kinds[0], &lists, &index).into_iter().map(|id| (index.name(&id), id)).collect::<Vec<_>>()
        })
        .map(|apps| apps.into_iter().map(|(name, id)| (id, name)).collect())
        .collect();
    let mut terminals: Vec<(String, String)> = index
        .entries()
        .iter()
        .filter(|e| e.categories.iter().any(|c| c == "TerminalEmulator") && !e.no_display)
        .filter_map(|e| Some((e.name.clone(), command(e.exec.as_deref()?))))
        .collect();
    terminals.sort();
    terminals.dedup_by(|a, b| a.1 == b.1);
    Offers { rows, terminals }
}

#[derive(PartialEq)]
pub struct DefaultApps {
    pub at: Location,
}

impl Component for DefaultApps {
    fn render(&self) -> impl IntoElement {
        let r = Rows::new(&self.at);
        let mut offers = use_state(Offers::default);
        // Counts the defaults set here, so the lists are read again.
        let mut set = use_state(|| 0u64);
        use_side_effect_with_deps(&*set.read(), move |_| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            std::thread::spawn(move || {
                let _ = tx.send(read());
            });
            spawn(async move {
                if let Ok(read) = rx.await {
                    offers.set(read);
                }
            });
        });
        let current = offers.read().clone();
        let mut rows: Vec<Element> = KINDS
            .iter()
            .enumerate()
            .map(|(n, (id, label, kinds))| {
                let apps = current.rows.get(n).cloned().unwrap_or_default();
                let control = if apps.is_empty() {
                    form::PopUp::new(vec!["None".into()], 0, *label, |_| {}).enabled(false)
                } else {
                    let names = apps.iter().map(|(_, name)| name.clone()).collect();
                    form::PopUp::new(names, 0, *label, move |i: usize| {
                        let Some((app, _)) = apps.get(i) else { return };
                        if let Err(err) = mimeapps::set_default(kinds, app, false) {
                            tracing::warn!("the default is not kept: {err}");
                        }
                        let n = *set.peek();
                        set.set(n + 1);
                    })
                };
                r.row(id, *label, None).control(control).into_element()
            })
            .collect();
        let terminal_now = r.str("shortcuts.terminal-command").unwrap_or_else(|| "kitty".into());
        let terminals = current.terminals.clone();
        let at = terminals.iter().position(|(_, cmd)| *cmd == terminal_now).unwrap_or(0);
        let desktop = r.desktop;
        rows.push(
            r.row("default-terminal", "Terminal", Some("shortcuts.terminal-command"))
                .control(
                    form::PopUp::new(
                        if terminals.is_empty() { vec![terminal_now.clone()] } else { terminals.iter().map(|(n, _)| n.clone()).collect() },
                        at,
                        "Terminal",
                        move |i: usize| {
                            if let Some((_, cmd)) = terminals.get(i) {
                                desktop.set("shortcuts.terminal-command", Value::Str(cmd.clone()));
                            }
                        },
                    )
                    .enabled(!current.terminals.is_empty()),
                )
                .into_element(),
        );
        form::group(Some("Default apps".into()), rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_terminals_command_drops_its_field_codes() {
        assert_eq!(command("kitty --single-instance %U"), "kitty --single-instance");
        assert_eq!(command("foot"), "foot");
    }
}
