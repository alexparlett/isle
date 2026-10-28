//! Widget sources: git repositories with `providers.json` at their root, fetched shallow
//! through gitoxide and installed from the fetched tree through staging (`p17-widgets.md`
//! choices 22 and 23).

use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use serde::{Deserialize, Serialize};

use crate::manifest::Manifest;

/// One provider a source offers.
#[derive(Clone, Debug, PartialEq)]
pub struct Listing {
    pub id: String,
    /// Its folder in the source's tree.
    pub path: String,
    pub manifest: Manifest,
}

/// What a source offers at a commit.
#[derive(Clone, Debug, PartialEq)]
pub struct Index {
    pub name: String,
    pub commit: String,
    pub widgets: Vec<Listing>,
    /// Entries left out, and why.
    pub problems: Vec<String>,
}

#[derive(Deserialize)]
struct IndexFile {
    name: String,
    #[serde(default)]
    format: u32,
    widgets: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    id: String,
    path: String,
}

/// What an install leaves beside a provider: where it came from.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstallNote {
    pub source: String,
    pub version: String,
    pub commit: String,
}

/// The source's clone under `cache`, named from its address.
pub fn clone_dir(cache: &Path, url: &str) -> PathBuf {
    let name: String = url
        .trim_end_matches('/')
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' })
        .collect();
    cache.join(name)
}

fn shallow() -> gix::remote::fetch::Shallow {
    gix::remote::fetch::Shallow::DepthAtRemote(1.try_into().unwrap())
}

/// Fetches `url` into its clone (made shallow and bare the first time) and reads its index.
/// `stop` ends a fetch that runs too long.
pub fn fetch(url: &str, cache: &Path, stop: &AtomicBool) -> Result<Index, String> {
    let dir = clone_dir(cache, url);
    if dir.exists() {
        let repo = gix::open(&dir).map_err(|e| e.to_string())?;
        let remote = repo
            .find_remote("origin")
            .map_err(|e| e.to_string())?
            .with_refspecs(Some("+refs/heads/*:refs/heads/*"), gix::remote::Direction::Fetch)
            .map_err(|e| e.to_string())?;
        remote
            .connect(gix::remote::Direction::Fetch)
            .map_err(|e| e.to_string())?
            .prepare_fetch(gix::progress::Discard, Default::default())
            .map_err(|e| e.to_string())?
            .with_shallow(shallow())
            .receive(gix::progress::Discard, stop)
            .map_err(|e| e.to_string())?;
        read(&dir)
    } else {
        std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
        let staging = cache.join(format!(".{}", clone_dir(cache, url).file_name().unwrap().to_string_lossy()));
        let _ = std::fs::remove_dir_all(&staging);
        let mut prepare = gix::prepare_clone_bare(url, &staging)
            .map_err(|e| e.to_string())?
            .with_shallow(shallow());
        prepare
            .fetch_only(gix::progress::Discard, stop)
            .map_err(|e| e.to_string())?;
        std::fs::rename(&staging, &dir).map_err(|e| e.to_string())?;
        read(&dir)
    }
}

/// The index of the clone as last fetched, with no network.
pub fn read(dir: &Path) -> Result<Index, String> {
    let repo = gix::open(dir).map_err(|e| e.to_string())?;
    let commit = repo.head_commit().map_err(|e| e.to_string())?;
    let tree = commit.tree().map_err(|e| e.to_string())?;
    let file = |path: &str| -> Option<Vec<u8>> {
        let entry = tree.lookup_entry_by_path(path).ok()??;
        if !entry.mode().is_blob() {
            return None;
        }
        Some(entry.object().ok()?.data.clone())
    };
    let index = file("providers.json").ok_or("It has no providers.json")?;
    let index: IndexFile =
        serde_json::from_slice(&index).map_err(|e| format!("providers.json: {e}"))?;
    if index.format > 2 {
        return Err(format!("providers.json is format {}, newer than Isle reads", index.format));
    }
    let mut widgets = Vec::new();
    let mut problems = Vec::new();
    for entry in index.widgets {
        let folder = entry.path.trim_matches('/').to_owned();
        if folder.split('/').any(|p| p == ".." || p.is_empty()) {
            problems.push(format!("{}: its path leaves the source", entry.id));
            continue;
        }
        let Some(text) = file(&format!("{folder}/widget.json")) else {
            problems.push(format!("{}: no widget.json at {folder}", entry.id));
            continue;
        };
        match Manifest::read(&String::from_utf8_lossy(&text), |f| file(&format!("{folder}/{f}")).is_some()) {
            Ok(manifest) if manifest.id == entry.id => widgets.push(Listing {
                id: entry.id,
                path: folder,
                manifest,
            }),
            Ok(manifest) => problems.push(format!("{}: its widget.json names {}", entry.id, manifest.id)),
            Err(p) => problems.push(format!("{}: {}", entry.id, p.join("; "))),
        }
    }
    Ok(Index {
        name: index.name,
        commit: commit.id.to_string(),
        widgets,
        problems,
    })
}

/// Writes `listing`'s folder from the clone into `root/<id>`, through staging, so it swaps
/// in whole; symbolic links are left out.
pub fn install(url: &str, cache: &Path, listing: &Listing, root: &Path) -> Result<(), String> {
    let repo = gix::open(clone_dir(cache, url)).map_err(|e| e.to_string())?;
    let commit = repo.head_commit().map_err(|e| e.to_string())?;
    let tree = commit.tree().map_err(|e| e.to_string())?;
    let entry = tree
        .lookup_entry_by_path(&listing.path)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("{} is not in the source any more", listing.path))?;
    let folder = entry.object().map_err(|e| e.to_string())?.into_tree();
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let staging = root.join(format!(".staging-{}", listing.id));
    let _ = std::fs::remove_dir_all(&staging);
    write_tree(&folder, &staging)?;
    let note = InstallNote {
        source: url.to_owned(),
        version: listing.manifest.version.clone(),
        commit: commit.id.to_string(),
    };
    std::fs::write(
        staging.join(".isle-install.json"),
        serde_json::to_vec_pretty(&note).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;
    let dest = root.join(&listing.id);
    let old = root.join(format!(".old-{}", listing.id));
    let _ = std::fs::remove_dir_all(&old);
    if dest.exists() {
        std::fs::rename(&dest, &old).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&staging, &dest).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&old);
    Ok(())
}

fn write_tree(tree: &gix::Tree<'_>, to: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in tree.iter() {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.filename().to_string();
        if name == ".." || name == "." || name.contains('/') {
            continue;
        }
        let path = to.join(&name);
        let mode = entry.mode();
        let object = entry.object().map_err(|e| e.to_string())?;
        if mode.is_tree() {
            write_tree(&object.into_tree(), &path)?;
        } else if mode.is_blob() {
            std::fs::write(&path, &object.data).map_err(|e| e.to_string())?;
            let bits = if mode.is_executable() { 0o755 } else { 0o644 };
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(bits)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Takes an installed provider out.
pub fn remove(root: &Path, id: &str) -> Result<(), String> {
    let dest = root.join(id);
    if dest.starts_with(root) && dest != root {
        std::fs::remove_dir_all(dest).map_err(|e| e.to_string())
    } else {
        Err(format!("{id} is not an installed widget"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "init.defaultBranch=main"])
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stderr));
    }

    fn source(dir: &Path, version: &str) {
        let w = dir.join("widgets/moon");
        std::fs::create_dir_all(&w).unwrap();
        std::fs::write(
            w.join("widget.json"),
            format!(r#"{{"id":"moon","name":"Moon","version":"{version}","sizes":["2x1"],"default":"2x1","template":"text","script":"moon.py","interval":60}}"#),
        )
        .unwrap();
        std::fs::write(w.join("moon.py"), "print('{\"text\":\"moon\"}')\n").unwrap();
        std::fs::write(
            dir.join("providers.json"),
            r#"{"name":"Test widgets","format":2,"widgets":[{"id":"moon","path":"widgets/moon"},{"id":"bad","path":"widgets/missing"},{"id":"out","path":"../x"}]}"#,
        )
        .unwrap();
    }

    #[test]
    fn a_source_is_fetched_read_installed_and_updated() {
        let root = std::env::temp_dir().join(format!("isle-sources-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (repo, cache, installed) = (root.join("repo"), root.join("cache"), root.join("installed"));
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q"]);
        source(&repo, "1.0.0");
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-qm", "one"]);
        let url = format!("file://{}", repo.display());
        let stop = AtomicBool::new(false);

        let index = fetch(&url, &cache, &stop).unwrap();
        assert_eq!(index.name, "Test widgets");
        assert_eq!(index.widgets.len(), 1);
        assert_eq!(index.problems.len(), 2, "{:?}", index.problems);
        install(&url, &cache, &index.widgets[0], &installed).unwrap();
        let moon = installed.join("moon");
        assert!(moon.join("moon.py").is_file());
        let note: InstallNote = serde_json::from_slice(&std::fs::read(moon.join(".isle-install.json")).unwrap()).unwrap();
        assert_eq!((note.version.as_str(), note.commit.as_str()), ("1.0.0", index.commit.as_str()));
        assert!(Manifest::load(&moon).is_ok());

        // A new commit is fetched over the old clone and installs over the old copy.
        source(&repo, "1.1.0");
        git(&repo, &["commit", "-qam", "two"]);
        let index = fetch(&url, &cache, &stop).unwrap();
        assert_eq!(index.widgets[0].manifest.version, "1.1.0");
        assert!(Manifest::load(&moon).unwrap().older_than(&index.widgets[0].manifest));
        install(&url, &cache, &index.widgets[0], &installed).unwrap();
        assert_eq!(Manifest::load(&moon).unwrap().version, "1.1.0");
        assert_eq!(read(&clone_dir(&cache, &url)).unwrap().commit, index.commit, "read again without a network");
        remove(&installed, "moon").unwrap();
        assert!(!moon.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
