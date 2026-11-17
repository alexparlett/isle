//! Software Update: what is waiting, the run's progress, and whether a restart is owed (`p14d-updates.md`).

mod check;
mod restart;
mod run;
mod system;

pub use check::{
    Check, CheckError, PKGBUILD, Part, Pending, Source, Tools, check, parse_flathub, parse_ls_remote, parse_updates,
    paru_config, pkgbuild_source, version_commit,
};
pub use restart::{Reason, restart_owed};
pub use system::{AppUpdate, Named, System, summarise};
pub use run::{Progress, Reader, RunState, read_progress, run, write_progress};

/// Isle's own package, built from git through paru's PKGBUILD repository.
pub const ISLE_PACKAGE: &str = "isle-git";
/// The transient user unit a run lives in.
pub const UNIT: &str = "isle-update.service";
/// Where a run keeps its progress: under the runtime directory, so it is gone after a reboot.
pub fn progress_path() -> std::path::PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from)
        .join("isle/update.json")
}

/// pacman's lock, held for the length of a transaction.
pub const DB_LOCK: &str = "/var/lib/pacman/db.lck";
