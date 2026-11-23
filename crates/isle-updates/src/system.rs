//! What a system update holds, as Store's Updates names it: the kernel, the graphics driver,
//! Mesa, CachyOS's tools, and the apps it also updates (B149).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Pending, Source};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Named {
    pub package: String,
    pub from: String,
    pub to: String,
}

/// An app among the packages: one that installs a desktop entry.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppUpdate {
    pub package: String,
    /// The desktop entry's id, for its name and icon.
    pub desktop: String,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct System {
    pub packages: usize,
    pub kernel: Option<Named>,
    pub driver: Option<Named>,
    pub mesa: Option<Named>,
    /// CachyOS's own packages, by name.
    pub tools: Vec<String>,
    pub apps: Vec<AppUpdate>,
}

fn version(v: &str) -> String {
    // Drop the epoch and the release, as the card shows them.
    let v = v.split_once(':').map_or(v, |(_, v)| v);
    v.rsplit_once('-').map_or(v, |(v, _)| v).to_string()
}

fn named(p: &Pending) -> Named {
    Named {
        package: p.name.clone(),
        from: version(&p.from),
        to: version(&p.to),
    }
}

/// A kernel package itself: `linux` or `linux-<flavour>`, not its headers, firmware or
/// modules built for it.
fn is_kernel(name: &str) -> bool {
    let Some(flavour) = name.strip_prefix("linux") else {
        return false;
    };
    if flavour.is_empty() {
        return true;
    }
    let Some(flavour) = flavour.strip_prefix('-') else {
        return false;
    };
    ![
        "headers",
        "firmware",
        "api-headers",
        "docs",
        "nvidia",
        "zfs",
        "tools",
    ]
    .iter()
    .any(|part| flavour.split('-').any(|w| w == *part))
}

fn is_driver(name: &str) -> bool {
    name == "nvidia-utils" || name.starts_with("nvidia-open") || name == "nvidia"
}

fn is_tool(name: &str) -> bool {
    name.starts_with("cachyos-") || name.starts_with("cachy-")
}

/// The desktop entries a package installs, from pacman's local database.
fn desktop_entries(root: &Path, package: &str, version: &str) -> Vec<String> {
    let files = root.join(format!("var/lib/pacman/local/{package}-{version}/files"));
    let Ok(text) = std::fs::read_to_string(files) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| l.strip_prefix("usr/share/applications/"))
        .filter(|f| f.ends_with(".desktop") && !f.contains('/'))
        .map(|f| f.trim_end_matches(".desktop").to_string())
        .collect()
}

/// Names what the repositories' updates hold, reading under `root`.
pub fn summarise(root: &Path, pending: &[Pending]) -> System {
    let repo: Vec<&Pending> = pending
        .iter()
        .filter(|p| p.source == Source::Repositories)
        .collect();
    let mut system = System {
        packages: repo.len(),
        ..System::default()
    };
    for p in &repo {
        if system.kernel.is_none() && is_kernel(&p.name) {
            system.kernel = Some(named(p));
        } else if system.driver.is_none() && is_driver(&p.name) {
            system.driver = Some(named(p));
        } else if p.name == "mesa" {
            system.mesa = Some(named(p));
        } else if is_tool(&p.name) {
            system.tools.push(p.name.clone());
        }
        if let Some(desktop) = desktop_entries(root, &p.name, &p.from).into_iter().next() {
            system.apps.push(AppUpdate {
                package: p.name.clone(),
                desktop,
                from: version(&p.from),
                to: version(&p.to),
            });
        }
    }
    system
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(name: &str, from: &str, to: &str) -> Pending {
        Pending {
            name: name.into(),
            from: from.into(),
            to: to.into(),
            source: Source::Repositories,
        }
    }

    #[test]
    fn kernels_and_their_relatives() {
        for k in [
            "linux",
            "linux-cachyos",
            "linux-cachyos-lts",
            "linux-zen",
            "linux-cachyos-bore",
        ] {
            assert!(is_kernel(k), "{k}");
        }
        for k in [
            "linux-cachyos-headers",
            "linux-firmware",
            "linux-api-headers",
            "linux-cachyos-nvidia-open",
            "linux-firmware-nvidia",
            "linuxconsoletools",
        ] {
            assert!(!is_kernel(k), "{k}");
        }
    }

    #[test]
    fn names_the_parts_and_finds_the_apps() {
        let root = std::env::temp_dir().join(format!("isle-updates-system-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let local = root.join("var/lib/pacman/local/blender-17:4.5.3-1");
        std::fs::create_dir_all(&local).unwrap();
        std::fs::write(
            local.join("files"),
            "%FILES%\nusr/\nusr/bin/blender\nusr/share/applications/blender.desktop\n",
        )
        .unwrap();
        let system = summarise(
            &root,
            &[
                pending("linux-cachyos", "7.2.10-1", "7.3.4-1"),
                pending("linux-cachyos-headers", "7.2.10-1", "7.3.4-1"),
                pending("nvidia-utils", "615.78.08-2", "615.95.04-1"),
                pending("mesa", "1:26.2.6-1", "1:26.2.7-1"),
                pending("cachyos-settings", "1:1.3-1", "1:1.4-1"),
                pending("blender", "17:4.5.3-1", "17:4.5.4-1"),
                pending("glibc", "2.44-1", "2.44-2"),
            ],
        );
        assert_eq!(system.packages, 7);
        assert_eq!(system.kernel.as_ref().unwrap().to, "7.3.4");
        assert_eq!(system.driver.as_ref().unwrap().from, "615.78.08");
        assert_eq!(system.mesa.as_ref().unwrap().to, "26.2.7");
        assert_eq!(system.tools, ["cachyos-settings"]);
        assert_eq!(
            system.apps,
            [AppUpdate {
                package: "blender".into(),
                desktop: "blender".into(),
                from: "4.5.3".into(),
                to: "4.5.4".into()
            }]
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
