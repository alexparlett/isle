//! Providers on disk: installed from sources, and the user's own.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::manifest::Manifest;

/// A provider found on disk.
#[derive(Clone, Debug)]
pub struct Installed {
    pub manifest: Manifest,
    pub folder: PathBuf,
    /// The source it came from; none for the user's own.
    pub source: Option<String>,
    pub version_commit: Option<String>,
}

/// Where installs from sources go: `~/.local/share/isle/widgets/<id>/`.
pub fn installed_folder() -> PathBuf {
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
    });
    data.join("isle/widgets")
}

#[derive(serde::Deserialize)]
struct InstallNote {
    source: String,
    #[serde(default)]
    commit: Option<String>,
}

/// Every provider in the two folders, the user's own winning a clash of ids; a folder whose
/// manifest has problems is left out.
pub fn scan() -> BTreeMap<String, Installed> {
    let mut out = BTreeMap::new();
    let read = |dir: &Path, own: bool, out: &mut BTreeMap<String, Installed>| {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let folder = entry.path();
            let Ok(manifest) = Manifest::load(&folder) else {
                continue;
            };
            let note: Option<InstallNote> = std::fs::read_to_string(folder.join(".isle-install.json"))
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok());
            let (source, version_commit) = match (own, note) {
                (false, Some(n)) => (Some(n.source), n.commit),
                _ => (None, None),
            };
            out.insert(
                manifest.id.clone(),
                Installed {
                    manifest,
                    folder,
                    source,
                    version_commit,
                },
            );
        }
    };
    read(&installed_folder(), false, &mut out);
    read(&crate::manifest::own_folder(), true, &mut out);
    out
}

