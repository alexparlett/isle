//! The system's share (D89; `p19b-cleaner.md` choice 18): the package cache, orphaned packages,
//! the journal and crash dumps, sized here without root and cleared by
//! `isle-clean-helper` under pkexec.

use std::{path::PathBuf, process::Command};

/// What the journal is cut down to.
pub const JOURNAL_KEEP: u64 = 100 << 20;
pub const COREDUMPS: &str = "/var/lib/systemd/coredump";
pub const HELPER: &str = "/usr/lib/isle/isle-clean-helper";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    PackageCache,
    Orphans,
    Journal,
    Coredumps,
}

impl Item {
    pub const ALL: [Item; 4] = [
        Item::PackageCache,
        Item::Orphans,
        Item::Journal,
        Item::Coredumps,
    ];

    /// The helper's subcommand, and the end of its polkit action's id.
    pub fn subcommand(self) -> &'static str {
        match self {
            Item::PackageCache => "package-cache",
            Item::Orphans => "orphans",
            Item::Journal => "journal",
            Item::Coredumps => "coredumps",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Item::PackageCache => "Package cache",
            Item::Orphans => "Orphaned packages",
            Item::Journal => "Journal",
            Item::Coredumps => "Crash dumps",
        }
    }

    /// The row's button: packages are removed, the rest cleared.
    pub fn action(self) -> &'static str {
        match self {
            Item::Orphans => "Remove…",
            _ => "Clear…",
        }
    }

    /// The helper run through pkexec; `ISLE_CLEAN_HELPER` names another for tests.
    pub fn command(self) -> Command {
        let helper = std::env::var_os("ISLE_CLEAN_HELPER")
            .map_or_else(|| PathBuf::from(HELPER), PathBuf::from);
        let mut c = Command::new("pkexec");
        c.arg(helper).arg(self.subcommand());
        c
    }
}

/// The helper's read of every btrfs file's bytes on disk (`ondisk`), through pkexec; a helper
/// named by `ISLE_CLEAN_HELPER` runs as it is, so a test never asks for a password.
pub fn measure_command() -> Command {
    match std::env::var_os("ISLE_CLEAN_HELPER") {
        Some(helper) => {
            let mut c = Command::new(helper);
            c.arg("extents");
            c
        }
        None => {
            let mut c = Command::new("pkexec");
            c.arg(HELPER).arg("extents");
            c
        }
    }
}

/// One row of the System card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Share {
    pub item: Item,
    pub bytes: u64,
    pub note: String,
}

fn out(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// "1.20 GiB", "512 KiB", "47.5M": a figure and a unit of powers of 1024.
fn bytes(number: &str, unit: &str) -> u64 {
    let scale: u64 = match unit.chars().next().map(|c| c.to_ascii_uppercase()) {
        Some('K') => 1 << 10,
        Some('M') => 1 << 20,
        Some('G') => 1 << 30,
        Some('T') => 1 << 40,
        _ => 1,
    };
    number
        .parse::<f64>()
        .map_or(0, |n| (n * scale as f64) as u64)
}

/// paccache's dry run: "… (disk space saved: 1.20 GiB)".
pub fn paccache_saved(text: &str) -> u64 {
    text.split_once("disk space saved: ")
        .and_then(|(_, rest)| {
            let mut w = rest.split_whitespace();
            Some(bytes(w.next()?, w.next()?.trim_end_matches(')')))
        })
        .unwrap_or(0)
}

/// `pacman -Qi`'s Installed Size lines, added up.
pub fn installed_sizes(text: &str) -> u64 {
    text.lines()
        .filter_map(|l| l.strip_prefix("Installed Size"))
        .filter_map(|l| {
            let mut w = l.trim_start_matches([' ', ':']).split_whitespace();
            Some(bytes(w.next()?, w.next()?))
        })
        .sum()
}

/// `journalctl --disk-usage`: "… take up 47.5M in the file system."
pub fn journal_usage(text: &str) -> u64 {
    text.split_once("take up ")
        .and_then(|(_, rest)| {
            let figure = rest.split_whitespace().next()?;
            let split = figure
                .find(|c: char| c.is_ascii_alphabetic())
                .unwrap_or(figure.len());
            Some(bytes(&figure[..split], &figure[split..]))
        })
        .unwrap_or(0)
}

/// The system card's rows, read as the user; a tool that is missing reads as nothing.
pub fn read() -> Vec<Share> {
    let package =
        paccache_saved(&out("paccache", &["-dk2"])) + paccache_saved(&out("paccache", &["-duk0"]));
    let orphans: Vec<String> = out("pacman", &["-Qdtq"])
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let orphan_bytes = if orphans.is_empty() {
        0
    } else {
        let mut args = vec!["-Qi"];
        args.extend(orphans.iter().map(String::as_str));
        installed_sizes(&out("pacman", &args))
    };
    let journal = journal_usage(&out("journalctl", &["--disk-usage"])).saturating_sub(JOURNAL_KEEP);
    let dumps: u64 = std::fs::read_dir(COREDUMPS)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| std::os::unix::fs::MetadataExt::blocks(&m) * 512)
        .sum();
    let listed = match orphans.len() {
        0 => "Installed as a dependency of something since removed".to_owned(),
        n if n <= 6 => orphans.join(", "),
        n => format!("{} and {} more", orphans[..6].join(", "), n - 6),
    };
    vec![
        Share {
            item: Item::PackageCache,
            bytes: package,
            note:
                "pacman's downloads of versions no longer installed, keeping the last two of each"
                    .into(),
        },
        Share {
            item: Item::Orphans,
            bytes: orphan_bytes,
            note: listed,
        },
        Share {
            item: Item::Journal,
            bytes: journal,
            note: "The system's log, cut down to its last 100 MB".into(),
        },
        Share {
            item: Item::Coredumps,
            bytes: dumps,
            note: "Memory saved when programs crashed, kept for debugging them".into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tools_figures_read() {
        assert_eq!(
            paccache_saved("==> finished dry run: 12 candidates (disk space saved: 1.50 GiB)\n"),
            1610612736
        );
        assert_eq!(
            paccache_saved("==> no candidate packages found for pruning\n"),
            0
        );
        assert_eq!(
            installed_sizes(
                "Name            : a\nInstalled Size  : 2.00 MiB\nName : b\nInstalled Size  : 512.00 KiB\n"
            ),
            (2 << 20) + (512 << 10)
        );
        assert_eq!(
            journal_usage("Archived and active journals take up 47.5M in the file system.\n"),
            (47.5 * 1048576.0) as u64
        );
        assert_eq!(
            journal_usage("… take up 1.2G in the file system."),
            (1.2 * 1073741824.0) as u64
        );
    }
}
