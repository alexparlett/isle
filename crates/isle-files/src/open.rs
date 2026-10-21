//! Opening items: each in the app Isle's lists name, through the compositor's launch path;
//! one nothing opens by itself in the Open With sheet (`p15-files.md` choice 12;
//! `files.md` § Acting on files, Open With).

use std::path::{Path, PathBuf};

use isle_apps::{
    Index,
    mimeapps::{self, Opening},
};

/// An app Open With offers.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub id: String,
    pub name: String,
}

/// What Open With shows for a file.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub path: PathBuf,
    pub kind: String,
    /// The apps for its type, the default first.
    pub apps: Vec<Choice>,
    /// Every app, for Other….
    pub all: Vec<Choice>,
}

fn index() -> Index {
    Index::scan(&Index::data_dirs())
}

fn choice(index: &Index, id: &str) -> Choice {
    Choice { id: id.to_owned(), name: index.name(id) }
}

/// Decides how each path opens; reads the lists, the entries and the files, so call it off the
/// window's thread. The apps to launch with their URIs, and the paths to ask about.
pub fn decide(paths: &[PathBuf]) -> (Vec<(String, Vec<String>)>, Vec<PathBuf>) {
    let index = index();
    let lists = mimeapps::read_lists();
    let mut launches: Vec<(String, Vec<String>)> = Vec::new();
    let mut ask = Vec::new();
    for path in paths {
        match mimeapps::opening(path, &lists, &index) {
            Opening::App(app) => {
                let uri = isle_thumbnail::uri(path);
                match launches.iter_mut().find(|(a, _)| *a == app) {
                    Some((_, uris)) => uris.push(uri),
                    None => launches.push((app, vec![uri])),
                }
            }
            Opening::Ask => ask.push(path.clone()),
        }
    }
    (launches, ask)
}

/// What Open With offers for `path`; off the window's thread.
pub fn offer(path: &Path) -> Offer {
    let index = index();
    let lists = mimeapps::read_lists();
    let kind = mimeapps::kind_of(path);
    let apps = mimeapps::apps_for(&kind, &lists, &index).iter().map(|id| choice(&index, id)).collect();
    let mut all: Vec<Choice> = index
        .entries()
        .iter()
        .filter(|e| !e.no_display && e.exec.is_some())
        .map(|e| Choice { id: e.id.clone(), name: e.name.clone() })
        .collect();
    all.sort_by_key(|c| c.name.to_lowercase());
    Offer { path: path.to_owned(), kind, apps, all }
}

/// Starts `app` with `uris` through the compositor, as every launch goes.
pub async fn launch(bus: zbus::Connection, app: String, uris: Vec<String>) {
    let Ok(windows) = isle_ipc::windows::WindowsProxy::new(&bus).await else { return };
    let uris: Vec<&str> = uris.iter().map(String::as_str).collect();
    match windows.launch(&app, "", &uris).await {
        Ok(true) => {}
        Ok(false) => tracing::warn!(app, "nothing to start"),
        Err(err) => tracing::warn!(app, "not started: {err}"),
    }
}

/// Starts the user's terminal in `folder`, in a scope of its own as the compositor's launches
/// are.
pub fn terminal(folder: &Path, command: &str) {
    let started = std::process::Command::new("systemd-run")
        .args(["--user", "--scope", "--collect", "--quiet", "--", "sh", "-c", command])
        .current_dir(folder)
        .stdin(std::process::Stdio::null())
        .spawn();
    if let Err(err) = started {
        tracing::warn!("no terminal: {err}");
    }
}
