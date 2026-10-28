//! What a provider may reach: one permission, in the manifest's words, the user's and the
//! sandbox's (`p17-widgets.md` choice 19).

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Grant {
    /// A host, by name, on 80 and 443.
    Connect(String),
    Read(String),
    Change(String),
    /// A session bus name.
    Bus(String),
    SystemBus(String),
    /// A command, by its name on the path.
    Run(String),
}

impl Grant {
    /// From `settings.kdl`'s `kind:target`.
    pub fn parse(text: &str) -> Option<Grant> {
        let (kind, target) = text.split_once(':')?;
        let target = target.trim().to_owned();
        if target.is_empty() {
            return None;
        }
        Some(match kind {
            "connect" => Grant::Connect(target),
            "read" => Grant::Read(target),
            "change" => Grant::Change(target),
            "bus" => Grant::Bus(target),
            "system-bus" => Grant::SystemBus(target),
            "run" => Grant::Run(target),
            _ => return None,
        })
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Grant::Connect(_) => "connect",
            Grant::Read(_) => "read",
            Grant::Change(_) => "change",
            Grant::Bus(_) => "bus",
            Grant::SystemBus(_) => "system-bus",
            Grant::Run(_) => "run",
        }
    }

    pub fn target(&self) -> &str {
        match self {
            Grant::Connect(t)
            | Grant::Read(t)
            | Grant::Change(t)
            | Grant::Bus(t)
            | Grant::SystemBus(t)
            | Grant::Run(t) => t,
        }
    }

    /// As `settings.kdl` keeps it.
    pub fn key(&self) -> String {
        format!("{}:{}", self.kind(), self.target())
    }

    /// In the grant rows' words (`structure/widgets.md` Grants).
    pub fn words(&self) -> String {
        match self {
            Grant::Connect(h) => format!("Connect to {h}"),
            Grant::Read(p) => format!("Read {p}"),
            Grant::Change(p) => format!("Read and change {p}"),
            Grant::Bus(n) | Grant::SystemBus(n) => format!("Use {n}"),
            Grant::Run(c) => format!("Run {c}"),
        }
    }

    /// The grant row's glyph.
    pub fn glyph(&self) -> &'static str {
        match self {
            Grant::Connect(_) => "globe",
            Grant::Read(_) => "folder",
            Grant::Change(_) => "pencil-simple",
            Grant::Bus(_) | Grant::SystemBus(_) => "plugs-connected",
            Grant::Run(_) => "terminal-window",
        }
    }

    /// Why the manifest's target cannot be granted, if it cannot.
    pub fn check(&self) -> Result<(), String> {
        let t = self.target();
        match self {
            Grant::Connect(h) => {
                let ok = !h.contains(['/', ':', ' ', '@'])
                    && h.split('.').all(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
                ok.then_some(()).ok_or_else(|| format!("“{h}” is not a host name"))
            }
            Grant::Read(p) | Grant::Change(p) => (p.starts_with('/') || p == "~" || p.starts_with("~/"))
                .then_some(())
                .ok_or_else(|| format!("“{p}” is not a path from / or ~")),
            Grant::Bus(n) | Grant::SystemBus(n) => (n.contains('.')
                && n.split('.').all(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')))
            .then_some(())
            .ok_or_else(|| format!("“{n}” is not a bus name")),
            Grant::Run(c) => (!c.contains(['/', ' ']) && !t.is_empty())
                .then_some(())
                .ok_or_else(|| format!("“{c}” is not a command name")),
        }
    }
}

/// `~` and `~/…` against `home`.
pub fn expand(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix('~') {
        Some(rest) => home.join(rest.trim_start_matches('/')),
        None => PathBuf::from(path),
    }
}

/// Places a provider never reaches whatever it declares: Isle's own settings and state, and
/// every provider's folder, so none can grant itself more (`p17-widgets.md` choice 18).
pub fn guarded(home: &Path) -> Vec<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME").map_or_else(|| home.join(".config"), PathBuf::from);
    let state = std::env::var_os("XDG_STATE_HOME").map_or_else(|| home.join(".local/state"), PathBuf::from);
    let data = std::env::var_os("XDG_DATA_HOME").map_or_else(|| home.join(".local/share"), PathBuf::from);
    let mut out = vec![config.join("isle"), state.join("isle"), data.join("isle")];
    if let Some(dir) = std::env::var_os("ISLE_CONFIG_DIR") {
        out.push(PathBuf::from(dir));
    }
    out
}

/// Whether `path` is in a guarded place, which no grant reaches; a grant of a folder that
/// holds one has the guarded place hidden inside it.
pub fn inside_guarded(path: &Path, home: &Path) -> bool {
    guarded(home).iter().any(|g| path.starts_with(g))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grants_read_back_and_say_themselves_in_words() {
        let g = Grant::parse("connect:api.open-meteo.com").unwrap();
        assert_eq!(g.words(), "Connect to api.open-meteo.com");
        assert_eq!(Grant::parse(&g.key()), Some(g));
        assert_eq!(Grant::parse("change:~/Notes").unwrap().words(), "Read and change ~/Notes");
        assert_eq!(Grant::parse("system-bus:org.freedesktop.NetworkManager").unwrap().words(), "Use org.freedesktop.NetworkManager");
        assert_eq!(Grant::parse("fly:away"), None);
        assert_eq!(Grant::parse("run:"), None);
    }

    #[test]
    fn targets_are_checked() {
        assert!(Grant::Connect("api.open-meteo.com".into()).check().is_ok());
        assert!(Grant::Connect("http://x.com".into()).check().is_err());
        assert!(Grant::Connect("x.com:8080".into()).check().is_err());
        assert!(Grant::Read("Documents".into()).check().is_err());
        assert!(Grant::Read("~/Documents".into()).check().is_ok());
        assert!(Grant::Run("/usr/bin/git".into()).check().is_err());
        assert!(Grant::Bus("org.kde.kdeconnect".into()).check().is_ok());
        assert!(Grant::Bus("kdeconnect".into()).check().is_err());
    }

    #[test]
    fn isles_own_places_are_guarded_however_they_are_named() {
        let home = Path::new("/home/u");
        assert!(inside_guarded(Path::new("/home/u/.config/isle/settings.kdl"), home));
        assert!(!inside_guarded(Path::new("/home/u/.config"), home), "a parent has it hidden instead");
        assert!(!inside_guarded(Path::new("/home/u/Documents"), home));
        assert_eq!(expand("~/Notes", home), PathBuf::from("/home/u/Notes"));
        assert_eq!(expand("~", home), PathBuf::from("/home/u"));
    }
}
