//! What is waiting: `checkupdates` for the repositories, and Isle's commit against `git
//! ls-remote` of its PKGBUILD's branch. The AUR is never looked at.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::ISLE_PACKAGE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    Repositories,
    Isle,
    Flathub,
    Aur,
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
    /// What the repositories' updates hold, named for Software Update's CachyOS card.
    #[serde(default)]
    pub system: crate::System,
    /// Seconds since the epoch.
    pub checked_at: u64,
}

/// What a run updates: the repositories, Isle's own build, or both in one run, for Software
/// Update; a Flathub app or all of them, an AUR package or all of them, for Store.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Part {
    System,
    Isle,
    Both,
    Flathub(Option<String>),
    Aur(Option<String>),
    /// Get: an app from a catalogue (`cachyos`, `flathub`, `aur`), the repositories' with
    /// their updates when any are waiting, since installing against a stale system is a
    /// partial upgrade.
    Install {
        catalogue: String,
        id: String,
        with_updates: bool,
    },
    /// Remove: a package with what only it needed, or a Flatpak app.
    Remove { catalogue: String, id: String },
    /// An AppImage brought up to date from its own update information.
    AppImage(String),
}

impl Part {
    /// `system`, `isle`, `both`, `flathub[:<app id>]`, `aur[:<package>]`,
    /// `install:<catalogue>[+updates]:<id>`, `remove:<catalogue>:<id>` or `appimage:<path>`.
    pub fn parse(word: &str) -> Option<Part> {
        let (kind, one) = match word.split_once(':') {
            Some((k, one)) if !one.is_empty() => (k, Some(one.to_string())),
            _ => (word, None),
        };
        if kind == "appimage" {
            return one.map(Part::AppImage);
        }
        if kind == "remove" {
            let one = one?;
            let (catalogue, id) = one.split_once(':')?;
            return (!id.is_empty()).then(|| Part::Remove {
                catalogue: catalogue.to_string(),
                id: id.to_string(),
            });
        }
        if kind == "install" {
            let (catalogue, id) = one?.split_once(':').map(|(c, i)| (c.to_string(), i.to_string()))?;
            let (catalogue, with_updates) = match catalogue.strip_suffix("+updates") {
                Some(c) => (c.to_string(), true),
                None => (catalogue, false),
            };
            return (!id.is_empty()).then_some(Part::Install { catalogue, id, with_updates });
        }
        match (kind, one) {
            ("system", None) => Some(Part::System),
            ("isle", None) => Some(Part::Isle),
            ("both", None) => Some(Part::Both),
            ("flathub", one) => Some(Part::Flathub(one)),
            ("aur", one) => Some(Part::Aur(one)),
            _ => None,
        }
    }

    pub fn word(&self) -> String {
        match self {
            Part::System => "system".into(),
            Part::Isle => "isle".into(),
            Part::Both => "both".into(),
            Part::Flathub(None) => "flathub".into(),
            Part::Flathub(Some(id)) => format!("flathub:{id}"),
            Part::Aur(None) => "aur".into(),
            Part::Aur(Some(name)) => format!("aur:{name}"),
            Part::Install { catalogue, id, with_updates } => format!(
                "install:{catalogue}{}:{id}",
                if *with_updates { "+updates" } else { "" }
            ),
            Part::Remove { catalogue, id } => format!("remove:{catalogue}:{id}"),
            Part::AppImage(path) => format!("appimage:{path}"),
        }
    }

    /// Whether the run goes through paru on Isle's config.
    pub fn builds_isle(&self) -> bool {
        matches!(self, Part::Isle | Part::Both)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    #[error("{0}")]
    Failed(String),
    #[error("couldn't run {0}: {1}")]
    Spawn(String, std::io::Error),
}

/// `name from -> to` lines, as `checkupdates` prints them.
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

/// The programs a check and a run use: `ISLE_CHECKUPDATES`, `ISLE_PACMAN` (for `pkexec
/// pacman`) and `ISLE_PARU` stand in for them in tests and the VM's rehearsals.
#[derive(Clone, Debug)]
pub struct Tools {
    pub checkupdates: PathBuf,
    pub pacman: Option<PathBuf>,
    /// None where paru is not installed: Isle's own package and the AUR are then not looked at.
    pub paru: Option<PathBuf>,
    /// None where flatpak is not installed: Flathub is then not looked at.
    pub flatpak: Option<PathBuf>,
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
            pacman: std::env::var_os("ISLE_PACMAN").map(PathBuf::from),
            paru: path("ISLE_PARU", "paru"),
            flatpak: path("ISLE_FLATPAK", "flatpak"),
        }
    }

    /// The run. The system is `pacman -Syu` through pkexec. Isle's build is paru in PKGBUILD
    /// mode, naming Isle's package, which paru rebuilds only when named until it has installed
    /// it once; both together are one paru in repository and PKGBUILD mode, so one process asks
    /// once and the AUR is never looked at. Without paru, Isle cannot be built.
    pub fn run_argv(&self, part: &Part) -> Vec<std::ffi::OsString> {
        const PARU: [&str; 5] = [
            "--noconfirm",
            "--skipreview",
            "--noprogressbar",
            "--sudo",
            "pkexec",
        ];
        let flatpak = || {
            self.flatpak
                .clone()
                .unwrap_or_else(|| "flatpak".into())
                .into_os_string()
        };
        let paru = || {
            self.paru
                .clone()
                .unwrap_or_else(|| "paru".into())
                .into_os_string()
        };
        let (first, words): (std::ffi::OsString, Vec<&str>) = match (&self.paru, part) {
            (_, Part::Flathub(one)) => (
                flatpak(),
                [
                    &["update", "-y", "--noninteractive"][..],
                    one.as_deref().as_slice(),
                ]
                .concat(),
            ),
            (_, Part::Aur(None)) => (paru(), [&["-Sua", "--mode=a"][..], &PARU].concat()),
            (_, Part::AppImage(path)) => (
                appimage_tool()
                    .map_or_else(|| "appimageupdatetool".into(), PathBuf::into_os_string),
                vec!["--overwrite", path.as_str()],
            ),
            (_, Part::Remove { catalogue, id }) => match catalogue.as_str() {
                "flathub" => (
                    flatpak(),
                    vec!["uninstall", "-y", "--noninteractive", id.as_str()],
                ),
                _ => match &self.pacman {
                    Some(pacman) => (
                        pacman.clone().into_os_string(),
                        vec!["-Rns", "--noconfirm", "--noprogressbar", id.as_str()],
                    ),
                    None => (
                        "pkexec".into(),
                        vec!["pacman", "-Rns", "--noconfirm", "--noprogressbar", id.as_str()],
                    ),
                },
            },
            (_, Part::Install { catalogue, id, with_updates }) => match catalogue.as_str() {
                "flathub" => (
                    flatpak(),
                    vec![
                        "install",
                        "-y",
                        "--noninteractive",
                        flathub_installation(),
                        "flathub",
                        id.as_str(),
                    ],
                ),
                // `-a` takes the target alone from the AUR; its dependencies come from the
                // repositories where they are.
                "aur" => (paru(), [&["-S", "-a"][..], &PARU, &[id.as_str()]].concat()),
                _ => {
                    let sync = if *with_updates { "-Syu" } else { "-S" };
                    match &self.pacman {
                        Some(pacman) => (
                            pacman.clone().into_os_string(),
                            vec![sync, "--needed", "--noconfirm", "--noprogressbar", id.as_str()],
                        ),
                        None => (
                            "pkexec".into(),
                            vec!["pacman", sync, "--needed", "--noconfirm", "--noprogressbar", id.as_str()],
                        ),
                    }
                }
            },
            (_, Part::Aur(Some(name))) => (
                paru(),
                [&["-S", "--mode=a"][..], &PARU, &[name.as_str()]].concat(),
            ),
            (Some(paru), Part::Both) => (
                paru.clone().into_os_string(),
                [&["-Syu", "--mode=rp"][..], &PARU, &[ISLE_PACKAGE]].concat(),
            ),
            (Some(paru), Part::Isle) => (
                paru.clone().into_os_string(),
                [&["-S", "--mode=p"][..], &PARU, &[ISLE_PACKAGE]].concat(),
            ),
            _ => match &self.pacman {
                Some(pacman) => (
                    pacman.clone().into_os_string(),
                    vec!["-Syu", "--noconfirm", "--noprogressbar"],
                ),
                None => (
                    "pkexec".into(),
                    vec!["pacman", "-Syu", "--noconfirm", "--noprogressbar"],
                ),
            },
        };
        std::iter::once(first)
            .chain(words.iter().map(Into::into))
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

/// Looks at the repositories and Isle's branch.
pub async fn check(tools: &Tools, now: u64) -> Result<Check, CheckError> {
    // checkupdates syncs into its own copy of the databases: 0 with updates, 2 with none.
    let checkupdates = tools.checkupdates.to_string_lossy().into_owned();
    let out = output(&checkupdates, &[]).await?;
    if !matches!(out.status.code(), Some(0 | 2)) {
        return Err(CheckError::Failed(reason(&out)));
    }
    let mut pending = parse_updates(&String::from_utf8_lossy(&out.stdout), Source::Repositories);

    let root =
        std::env::var_os("ISLE_UPDATES_ROOT").map_or_else(|| PathBuf::from("/"), PathBuf::from);
    // Only paru can build Isle, so without it Isle's commit is not looked at.
    if tools.paru.is_some()
        && let Some(isle) = isle_update(&root).await?
    {
        pending.push(isle);
    }
    if let Some(flatpak) = tools.flatpak.as_deref().and_then(|p| p.to_str()) {
        pending.extend(flathub_updates(flatpak).await?);
    }
    if let Some(paru) = tools.paru.as_deref().and_then(|p| p.to_str()) {
        let out = output(paru, &["-Qua", "--mode=a"]).await?;
        // paru exits 1 when nothing is waiting; only an error line on stderr is a failure.
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
    }
    let system = crate::system::summarise(&root, &pending);
    Ok(Check {
        pending,
        system,
        checked_at: now,
    })
}

/// Flathub's apps with a newer build: `remote-ls --updates` gives the new version, `list` the
/// installed one; both are tab-separated columns.
async fn flathub_updates(flatpak: &str) -> Result<Vec<Pending>, CheckError> {
    let columns = "--columns=application,version";
    let newer = output(flatpak, &["remote-ls", "--updates", "--app", columns]).await?;
    if !newer.status.success() {
        return Err(CheckError::Failed(reason(&newer)));
    }
    let installed = output(flatpak, &["list", "--app", columns]).await?;
    Ok(parse_flathub(
        &String::from_utf8_lossy(&newer.stdout),
        &String::from_utf8_lossy(&installed.stdout),
    ))
}

/// The tool that updates AppImages: `appimageupdatetool` on the path, else its own AppImage put
/// in `~/Applications`.
pub fn appimage_tool() -> Option<PathBuf> {
    let on_path = std::env::var_os("PATH").into_iter().flat_map(|p| {
        std::env::split_paths(&p)
            .map(|d| d.join("appimageupdatetool"))
            .collect::<Vec<_>>()
    });
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let dropped = std::fs::read_dir(home.join("Applications"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("appimageupdatetool") && n.ends_with(".AppImage"))
        });
    on_path.chain(dropped).find(|p| p.is_file())
}

/// Where Store installs from Flathub: the user's installation when it has the remote, so no
/// password is asked, else the system's. Named, since a remote in both makes flatpak ask.
pub fn flathub_installation() -> &'static str {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let config = std::fs::read_to_string(home.join(".local/share/flatpak/repo/config"))
        .unwrap_or_default();
    if config.contains("[remote \"flathub\"]") {
        "--user"
    } else {
        "--system"
    }
}

pub fn parse_flathub(newer: &str, installed: &str) -> Vec<Pending> {
    let row = |line: &str| {
        let mut cols = line.split('\t');
        Some((
            cols.next()?.trim().to_string(),
            cols.next().unwrap_or("").trim().to_string(),
        ))
    };
    let have: std::collections::HashMap<String, String> =
        installed.lines().filter_map(row).collect();
    newer
        .lines()
        .filter_map(row)
        .filter(|(id, _)| !id.is_empty())
        .map(|(id, to)| Pending {
            from: have.get(&id).cloned().unwrap_or_default(),
            to,
            name: id,
            source: Source::Flathub,
        })
        .collect()
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
    fn flathub_s_columns() {
        let got = parse_flathub(
            "com.usebottles.bottles\t51.24\ncom.spotify.Client\t1.2.77\n",
            "com.usebottles.bottles\t51.21\ncom.spotify.Client\t1.2.74\norg.other.App\t1\n",
        );
        assert_eq!(got.len(), 2);
        assert_eq!(
            (
                got[0].name.as_str(),
                got[0].from.as_str(),
                got[0].to.as_str()
            ),
            ("com.usebottles.bottles", "51.21", "51.24")
        );
        assert_eq!(got[0].source, Source::Flathub);
    }

    #[test]
    fn parts_by_word() {
        for w in [
            "system",
            "isle",
            "both",
            "flathub",
            "flathub:com.spotify.Client",
            "aur",
            "aur:xremap-hypr-bin",
            "install:cachyos:krita",
            "install:cachyos+updates:krita",
            "install:flathub:com.obsproject.Studio",
            "install:aur:obs-vkcapture",
            "remove:cachyos:krita",
            "remove:flathub:com.obsproject.Studio",
            "appimage:/home/u/Applications/Obsidian.AppImage",
        ] {
            assert_eq!(Part::parse(w).unwrap().word(), w);
        }
        assert_eq!(Part::parse("nothing"), None);
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
