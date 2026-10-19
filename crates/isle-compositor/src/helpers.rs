//! Isle's helper programs, held open from the compositor's start so a package installed
//! while it runs cannot hand it a helper from another build.

use std::{
    fs::File,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub struct Helper {
    /// Where it was found, for logs.
    pub found: PathBuf,
    file: Option<File>,
}

impl Helper {
    fn open(name: &str, test_override: Option<&str>) -> Self {
        let found = locate(name, test_override);
        let file = File::open(&found).ok();
        Self { found, file }
    }

    /// What to execute: the descriptor's path while it is open, which the kernel resolves
    /// to the file the compositor started with even after it is replaced on disk.
    pub fn program(&self) -> PathBuf {
        match &self.file {
            Some(file) => PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd())),
            None => self.found.clone(),
        }
    }

    pub fn available(&self) -> bool {
        self.file.is_some()
    }
}

static AUTH: OnceLock<Helper> = OnceLock::new();
static RECORD: OnceLock<Helper> = OnceLock::new();

/// Opens both; called once at start, before any update could replace them.
pub fn pin() {
    auth();
    record();
}

pub fn auth() -> &'static Helper {
    AUTH.get_or_init(|| Helper::open("isle-auth", Some("ISLE_AUTH_HELPER")))
}

pub fn record() -> &'static Helper {
    RECORD.get_or_init(|| Helper::open("isle-record", Some("ISLE_RECORD_HELPER")))
}

/// Beside the compositor, else on `PATH`; a test build may name its own.
fn locate(name: &str, test_override: Option<&str>) -> PathBuf {
    #[cfg(feature = "test-hooks")]
    if let Some(path) = test_override.and_then(std::env::var_os) {
        return PathBuf::from(path);
    }
    #[cfg(not(feature = "test-hooks"))]
    let _ = test_override;
    let beside = std::env::current_exe()
        .ok()
        .map(|exe| exe.with_file_name(name))
        .filter(|path| path.is_file());
    beside
        .or_else(|| on_path(name))
        .unwrap_or_else(|| PathBuf::from(name))
}

fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| Path::is_file(candidate))
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    #[test]
    fn a_replaced_helper_still_runs_as_it_was_pinned() {
        let dir = std::env::temp_dir().join(format!("isle-helper-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("helper");
        // As pacman installs: the new file written beside and renamed over the old.
        let install = |from: &str| {
            let next = dir.join("next");
            std::fs::copy(from, &next).unwrap();
            std::fs::rename(&next, &path).unwrap();
        };
        install("/usr/bin/true");
        let helper = Helper {
            found: path.clone(),
            file: Some(File::open(&path).unwrap()),
        };
        install("/usr/bin/false");
        assert!(Command::new(helper.program()).status().unwrap().success());
        assert!(!Command::new(&path).status().unwrap().success());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
