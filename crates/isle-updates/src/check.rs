//! What is waiting: `checkupdates` for the repositories, `paru -Qua` for the AUR, and Isle's
//! commit in paru's `devel.toml` against `git ls-remote` (paru never lists a PKGBUILD
//! repository's devel package, `updates-2026.md` §3).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::ISLE_PACKAGE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    Repositories,
    Aur,
    Isle,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    pub name: String,
    pub from: String,
    pub to: String,
    pub source: Source,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    pub pending: Vec<Pending>,
    /// Seconds since the epoch.
    pub checked_at: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    #[error("{0}")]
    Failed(String),
    #[error("couldn't run {0}: {1}")]
    Spawn(String, std::io::Error),
}

/// `name from -> to` lines, as `checkupdates` and `paru -Qua` print them.
pub fn parse_updates(text: &str, source: Source) -> Vec<Pending> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let from = words.next()?;
            (words.next()? == "->").then_some(())?;
            let to = words.next()?;
            Some(Pending {
                name: name.into(),
                from: from.into(),
                to: to.into(),
                source,
            })
        })
        .collect()
}

/// The commit `git ls-remote` names for `refs/heads/<branch>`, or for HEAD without a branch.
pub fn parse_ls_remote(text: &str, branch: Option<&str>) -> Option<String> {
    let want = branch.map_or("HEAD".to_string(), |b| format!("refs/heads/{b}"));
    text.lines().find_map(|line| {
        let (hash, name) = line.split_once('\t')?;
        (name.trim() == want).then(|| hash.to_string())
    })
}

/// The version of an installed package, from pacman's local database without running pacman.
pub(crate) fn installed_version(root: &Path, package: &str) -> Option<String> {
    let prefix = format!("{package}-");
    std::fs::read_dir(root.join("var/lib/pacman/local"))
        .ok()?
        .flatten()
        .find_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let rest = name.strip_prefix(&prefix)?;
            // A longer name sharing the prefix (`nvidia-utils-…` vs `nvidia-utils-beta-…`) has a
            // version that does not start with a digit, an epoch, or `r` and a digit.
            let first = rest.chars().next()?;
            let looks = first.is_ascii_digit()
                || (first == 'r' && rest[1..].starts_with(|c: char| c.is_ascii_digit()));
            (looks && rest.matches('-').count() == 1).then(|| rest.to_string())
        })
}

async fn output(program: &str, args: &[&str]) -> Result<std::process::Output, CheckError> {
    let child = Command::new(program)
        .args(args)
        .env("LC_ALL", "C.UTF-8")
        .env("GIT_TERMINAL_PROMPT", "0")
        .kill_on_drop(true)
        .output();
    match tokio::time::timeout(Duration::from_secs(120), child).await {
        Ok(out) => out.map_err(|e| CheckError::Spawn(program.to_string(), e)),
        Err(_) => Err(CheckError::Failed(format!("{program} took too long"))),
    }
}

fn reason(out: &std::process::Output) -> String {
    let err = String::from_utf8_lossy(&out.stderr);
    let line = err
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();
    let line = line
        .strip_prefix("==> ERROR: ")
        .or_else(|| line.strip_prefix("error: "))
        .unwrap_or(line);
    if line.is_empty() {
        format!("stopped with code {}", out.status.code().unwrap_or(-1))
    } else {
        line.to_string()
    }
}

/// The programs a check and a run use: `ISLE_CHECKUPDATES` and `ISLE_PARU` stand in for
/// them in tests and the VM's rehearsals.
#[derive(Clone, Debug)]
pub struct Tools {
    pub checkupdates: PathBuf,
    /// None where paru is not installed: the AUR and Isle are then not looked at.
    pub paru: Option<PathBuf>,
}

impl Tools {
    pub fn from_env() -> Self {
        let path = |var: &str, name: &str| {
            std::env::var_os(var).map(PathBuf::from).or_else(|| {
                std::env::var_os("PATH").and_then(|dirs| {
                    std::env::split_paths(&dirs)
                        .map(|d| d.join(name))
                        .find(|p| p.is_file())
                })
            })
        };
        Tools {
            checkupdates: path("ISLE_CHECKUPDATES", "checkupdates")
                .unwrap_or_else(|| "checkupdates".into()),
            paru: path("ISLE_PARU", "paru"),
        }
    }

    /// The run: a full upgrade through paru, which builds as the user and hands pacman to
    /// pkexec; pacman alone without it. `isle` names Isle's package, which paru rebuilds only
    /// when named until it has installed it once.
    pub fn run_argv(&self, isle: bool) -> Vec<std::ffi::OsString> {
        let words: &[&str] = match &self.paru {
            Some(_) => &[
                "-Syu",
                "--noconfirm",
                "--skipreview",
                "--noprogressbar",
                "--sudo",
                "pkexec",
            ],
            None => &["pacman", "-Syu", "--noconfirm", "--noprogressbar"],
        };
        let first = self
            .paru
            .clone()
            .map_or_else(|| "pkexec".into(), PathBuf::into_os_string);
        let isle = (isle && self.paru.is_some()).then_some(ISLE_PACKAGE);
        std::iter::once(first)
            .chain(words.iter().copied().chain(isle).map(Into::into))
            .collect()
    }
}

/// paru's config for a run: the user's own, or the system's, with Isle's PKGBUILD as a
/// repository of its own in `dir`, where the build keeps its target between updates.
pub fn paru_config(dir: &Path, config_home: &Path) -> std::io::Result<PathBuf> {
    let pkgbuild = dir.join("pkgbuild");
    std::fs::create_dir_all(&pkgbuild)?;
    if std::fs::read_to_string(pkgbuild.join("PKGBUILD"))
        .ok()
        .as_deref()
        != Some(PKGBUILD)
    {
        std::fs::write(pkgbuild.join("PKGBUILD"), PKGBUILD)?;
    }
    let users = config_home.join("paru/paru.conf");
    let include = if users.is_file() {
        users
    } else {
        PathBuf::from("/etc/paru.conf")
    };
    let text = format!(
        "Include = {}\n\n[isle]\nPath = {}\nGenerateSrcinfo\nSkipReview\n",
        include.display(),
        pkgbuild.display()
    );
    let path = dir.join("paru.conf");
    std::fs::write(&path, text)?;
    Ok(path)
}

/// Looks at all three sources.
pub async fn check(tools: &Tools, now: u64) -> Result<Check, CheckError> {
    // checkupdates syncs into its own copy of the databases: 0 with updates, 2 with none.
    let checkupdates = tools.checkupdates.to_string_lossy().into_owned();
    let out = output(&checkupdates, &[]).await?;
    if !matches!(out.status.code(), Some(0 | 2)) {
        return Err(CheckError::Failed(reason(&out)));
    }
    let mut pending = parse_updates(&String::from_utf8_lossy(&out.stdout), Source::Repositories);

    if let Some(paru) = tools.paru.as_deref().and_then(|p| p.to_str()) {
        let out = Command::new(paru)
            .args(["-Qua"])
            .env("LC_ALL", "C.UTF-8")
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| CheckError::Spawn("paru".into(), e))?;
        // paru exits 1 when nothing is waiting, and says why on stderr only when something failed.
        let text = String::from_utf8_lossy(&out.stdout);
        if !out.status.success()
            && text.trim().is_empty()
            && String::from_utf8_lossy(&out.stderr).contains("error")
        {
            return Err(CheckError::Failed(reason(&out)));
        }
        pending.extend(
            parse_updates(&text, Source::Aur)
                .into_iter()
                .filter(|p| p.name != ISLE_PACKAGE),
        );

        let root =
            std::env::var_os("ISLE_UPDATES_ROOT").map_or_else(|| PathBuf::from("/"), PathBuf::from);
        if let Some(isle) = isle_update(&root).await? {
            pending.push(isle);
        }
    }
    Ok(Check {
        pending,
        checked_at: now,
    })
}

/// The PKGBUILD the run builds Isle from, as this build was packaged with it.
pub const PKGBUILD: &str = include_str!("../../../pkg/arch/PKGBUILD");

/// The git URL and branch of the PKGBUILD's Isle source.
pub fn pkgbuild_source(pkgbuild: &str) -> Option<(String, Option<String>)> {
    let at = pkgbuild.find("git+")? + 4;
    let rest = &pkgbuild[at..];
    let end = rest.find(['}', '"', '\'', ' ', '\n']).unwrap_or(rest.len());
    let source = &rest[..end];
    Some(match source.split_once("#branch=") {
        Some((url, branch)) => (url.to_string(), Some(branch.to_string())),
        None => (source.split('#').next().unwrap_or(source).to_string(), None),
    })
}

/// The commit an `r<count>.g<hash>-<rel>` version was built from.
pub fn version_commit(version: &str) -> Option<&str> {
    let hash = version.split_once(".g")?.1;
    Some(hash.split('-').next().unwrap_or(hash)).filter(|h| !h.is_empty())
}

async fn isle_update(root: &Path) -> Result<Option<Pending>, CheckError> {
    let Some(installed) = installed_version(root, ISLE_PACKAGE) else {
        return Ok(None);
    };
    let Some(built) = version_commit(&installed) else {
        return Ok(None);
    };
    // `ISLE_UPDATES_SOURCE`, `url#branch=name`, stands in for the PKGBUILD's in tests.
    let pkgbuild = std::env::var("ISLE_UPDATES_SOURCE")
        .map(|s| format!("source=(\"git+{s}\")"))
        .unwrap_or_else(|_| PKGBUILD.to_string());
    let Some((url, branch)) = pkgbuild_source(&pkgbuild) else {
        return Ok(None);
    };
    let refspec = branch
        .as_deref()
        .map_or("HEAD".to_string(), |b| format!("refs/heads/{b}"));
    let out = output("git", &["ls-remote", &url, &refspec]).await?;
    if !out.status.success() {
        return Err(CheckError::Failed(reason(&out)));
    }
    let Some(remote) = parse_ls_remote(&String::from_utf8_lossy(&out.stdout), branch.as_deref())
    else {
        return Ok(None);
    };
    if remote.starts_with(built) {
        return Ok(None);
    }
    Ok(Some(Pending {
        name: ISLE_PACKAGE.into(),
        from: installed,
        to: remote[..12.min(remote.len())].to_string(),
        source: Source::Isle,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");

    #[test]
    fn checkupdates_on_the_host() {
        let text = std::fs::read_to_string(format!("{DATA}/checkupdates.txt")).unwrap();
        let got = parse_updates(&text, Source::Repositories);
        assert_eq!(got.len(), 470);
        assert_eq!(
            got[0],
            Pending {
                name: "7zip".into(),
                from: "26.04-1.1".into(),
                to: "26.06-1.1".into(),
                source: Source::Repositories
            }
        );
    }

    #[test]
    fn paru_names_a_devel_package_latest_commit() {
        let text = std::fs::read_to_string(format!("{DATA}/paru-qua.txt")).unwrap();
        let got = parse_updates(&text, Source::Aur);
        assert_eq!(got.len(), 2);
        assert!(
            got.iter()
                .any(|p| p.name == "appimagetool-bin" && p.to == "latest-commit")
        );
    }

    #[test]
    fn isle_s_source_and_commit() {
        let (url, branch) = pkgbuild_source(PKGBUILD).unwrap();
        assert_eq!(url, "https://github.com/alexparlett/isle.git");
        assert_eq!(branch.as_deref(), Some("feature/rust-desktop"));
        assert_eq!(version_commit("r725.g0f8209c46951-1"), Some("0f8209c46951"));
        assert_eq!(version_commit("1.0-1"), None);
    }

    #[test]
    fn ls_remote_by_branch() {
        let text = "aaa\tHEAD\nbbb\trefs/heads/feature/rust-desktop\nccc\trefs/heads/main\n";
        assert_eq!(
            parse_ls_remote(text, Some("feature/rust-desktop")).as_deref(),
            Some("bbb")
        );
        assert_eq!(parse_ls_remote(text, None).as_deref(), Some("aaa"));
        assert_eq!(parse_ls_remote(text, Some("gone")), None);
    }

    #[test]
    fn installed_versions_from_the_local_database() {
        let root = std::env::temp_dir().join(format!("isle-updates-local-{}", std::process::id()));
        for d in [
            "nvidia-utils-615.78.08-2",
            "lib32-nvidia-utils-615.78.08-1",
            "nvidia-utils-beta-git-r1.gabc-1",
            "isle-git-r725.g0f8209c46951-1",
        ] {
            std::fs::create_dir_all(root.join("var/lib/pacman/local").join(d)).unwrap();
        }
        assert_eq!(
            installed_version(&root, "nvidia-utils").as_deref(),
            Some("615.78.08-2")
        );
        assert_eq!(
            installed_version(&root, "isle-git").as_deref(),
            Some("r725.g0f8209c46951-1")
        );
        assert_eq!(installed_version(&root, "linux"), None);
        std::fs::remove_dir_all(root).unwrap();
    }
}
