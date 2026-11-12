//! Whether the machine is running older code than it has installed (`platform.md` Updates 5).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::check::installed_version;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reason {
    Kernel,
    Graphics,
    Isle,
}

impl Reason {
    pub fn sentence(self) -> &'static str {
        match self {
            Reason::Kernel => "The kernel was updated",
            Reason::Graphics => "The graphics driver was updated",
            Reason::Isle => "Isle was updated",
        }
    }
}

/// Reads under `root` (`/` outside tests). `release` is the running kernel's, `pids` Isle's
/// long-running processes, whose executables read " (deleted)" once a package replaced them.
pub fn restart_owed(root: &Path, release: &str, pids: &[u32]) -> Vec<Reason> {
    let mut out = Vec::new();
    let modules = root.join("usr/lib/modules");
    if modules.is_dir() && !modules.join(release).is_dir() {
        out.push(Reason::Kernel);
    }
    if let Ok(loaded) = std::fs::read_to_string(root.join("sys/module/nvidia/version")) {
        // pacman's version carries a release after the last dash; the module's has none.
        let installed = installed_version(root, "nvidia-utils");
        let installed = installed
            .as_deref()
            .and_then(|v| v.rsplit_once('-'))
            .map(|(v, _)| v);
        let installed = installed.map(|v| v.split_once(':').map_or(v, |(_, v)| v));
        if installed.is_some_and(|v| v != loaded.trim()) {
            out.push(Reason::Graphics);
        }
    }
    let replaced = pids.iter().any(|pid| {
        std::fs::read_link(root.join(format!("proc/{pid}/exe")))
            .is_ok_and(|exe| exe.to_string_lossy().ends_with(" (deleted)"))
    });
    if replaced {
        out.push(Reason::Isle);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tree(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("isle-updates-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("usr/lib/modules/7.2.10-1-cachyos")).unwrap();
        std::fs::create_dir_all(root.join("sys/module/nvidia")).unwrap();
        std::fs::write(root.join("sys/module/nvidia/version"), "615.78.08\n").unwrap();
        std::fs::create_dir_all(root.join("var/lib/pacman/local/nvidia-utils-615.78.08-2"))
            .unwrap();
        std::fs::create_dir_all(root.join("proc/42")).unwrap();
        std::os::unix::fs::symlink("/usr/bin/isle-compositor", root.join("proc/42/exe")).unwrap();
        root
    }

    #[test]
    fn nothing_owed_on_the_host_today() {
        let root = tree("none");
        assert_eq!(restart_owed(&root, "7.2.10-1-cachyos", &[42]), vec![]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn each_reason() {
        let root = tree("all");
        std::fs::rename(
            root.join("usr/lib/modules/7.2.10-1-cachyos"),
            root.join("usr/lib/modules/7.2.15-1-cachyos"),
        )
        .unwrap();
        std::fs::rename(
            root.join("var/lib/pacman/local/nvidia-utils-615.78.08-2"),
            root.join("var/lib/pacman/local/nvidia-utils-615.95.04-1"),
        )
        .unwrap();
        std::fs::remove_file(root.join("proc/42/exe")).unwrap();
        std::os::unix::fs::symlink(
            "/usr/bin/isle-compositor (deleted)",
            root.join("proc/42/exe"),
        )
        .unwrap();
        assert_eq!(
            restart_owed(&root, "7.2.10-1-cachyos", &[42]),
            vec![Reason::Kernel, Reason::Graphics, Reason::Isle]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_driver_loaded_owes_nothing() {
        let root = tree("nodriver");
        std::fs::remove_dir_all(root.join("sys/module/nvidia")).unwrap();
        std::fs::rename(
            root.join("var/lib/pacman/local/nvidia-utils-615.78.08-2"),
            root.join("var/lib/pacman/local/nvidia-utils-615.95.04-1"),
        )
        .unwrap();
        assert_eq!(restart_owed(&root, "7.2.10-1-cachyos", &[42]), vec![]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
