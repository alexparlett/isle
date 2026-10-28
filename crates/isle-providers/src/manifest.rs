//! A widget provider's `widget.json`: who it is, its sizes and settings, what it runs and
//! what it may reach (`p17-widgets.md` choices 17 and 19; v1's manifest without QML).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{grant::Grant, template::Template};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Permissions {
    #[serde(default)]
    pub connect: Vec<String>,
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub change: Vec<String>,
    #[serde(default)]
    pub bus: Vec<String>,
    #[serde(default, rename = "system-bus")]
    pub system_bus: Vec<String>,
    #[serde(default)]
    pub run: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingType {
    Toggle,
    Choice,
    Number,
    Text,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: SettingType,
    /// A choice's values.
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub author: Author,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub homepage: Option<String>,
    /// As `"2x1"`, in the dashboard's cells.
    pub sizes: Vec<String>,
    pub default: String,
    /// More than one may be placed.
    #[serde(default)]
    pub multiple: bool,
    pub template: Template,
    /// A program in the folder, or on the path when granted, with its arguments.
    #[serde(default)]
    pub exec: Option<Vec<String>>,
    /// A Python script in the folder, run by python3.
    #[serde(default)]
    pub script: Option<String>,
    /// Seconds between runs; none for a stream.
    #[serde(default)]
    pub interval: Option<u64>,
    /// It runs on, writing an answer a line whenever it has one.
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default)]
    pub settings: Vec<Setting>,
}

/// The shortest interval a provider may ask for.
pub const SHORTEST: u64 = 5;

pub fn size(text: &str) -> Option<(i32, i32)> {
    let (w, h) = text.split_once('x')?;
    let (w, h) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    ((1..=12).contains(&w) && (1..=8).contains(&h)).then_some((w, h))
}

impl Manifest {
    /// Reads and checks `widget.json` in `folder`.
    pub fn load(folder: &Path) -> Result<Manifest, Vec<String>> {
        let text = std::fs::read_to_string(folder.join("widget.json"))
            .map_err(|e| vec![format!("widget.json can't be read: {e}")])?;
        let manifest: Manifest = serde_json::from_str(&text).map_err(|e| vec![format!("widget.json: {e}")])?;
        let problems = manifest.problems(folder);
        if problems.is_empty() { Ok(manifest) } else { Err(problems) }
    }

    /// Reads a manifest's text, checked as if its folder held `has`'s files.
    pub fn read(text: &str, has: impl Fn(&str) -> bool) -> Result<Manifest, Vec<String>> {
        let manifest: Manifest = serde_json::from_str(text).map_err(|e| vec![format!("widget.json: {e}")])?;
        let problems = manifest.problems_with(has);
        if problems.is_empty() { Ok(manifest) } else { Err(problems) }
    }

    /// Everything wrong with it, in words an author can act on.
    pub fn problems(&self, folder: &Path) -> Vec<String> {
        self.problems_with(|file| folder.join(file).is_file())
    }

    fn problems_with(&self, has: impl Fn(&str) -> bool) -> Vec<String> {
        let mut out = Vec::new();
        let slug = |s: &str| {
            !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        };
        if !slug(&self.id) {
            out.push(format!("“{}” is not an id: lowercase letters, digits and hyphens", self.id));
        }
        if self.name.trim().is_empty() {
            out.push("It has no name".into());
        }
        if self.version.split('.').count() != 3 || self.version.split('.').any(|p| p.parse::<u64>().is_err()) {
            out.push(format!("“{}” is not a version as 1.2.3", self.version));
        }
        let sizes: Vec<_> = self.sizes.iter().filter_map(|s| size(s)).collect();
        if sizes.len() != self.sizes.len() || sizes.is_empty() {
            out.push("Its sizes are not all as 2x1, up to 12x8".into());
        }
        if !self.sizes.contains(&self.default) {
            out.push(format!("Its default size {} is not one of its sizes", self.default));
        }
        match (&self.exec, &self.script) {
            (Some(exec), None) if exec.is_empty() => out.push("Its exec is empty".into()),
            (Some(_), None) | (None, Some(_)) => {}
            _ => out.push("It names exec or script, one of them".into()),
        }
        if let Some(script) = &self.script
            && !has(script)
        {
            out.push(format!("Its script {script} is not in its folder"));
        }
        match (self.stream, self.interval) {
            (true, Some(_)) => out.push("A stream has no interval".into()),
            (false, None) => out.push("It names an interval in seconds, or stream".into()),
            (false, Some(i)) if i < SHORTEST => out.push(format!("Its interval is under {SHORTEST} seconds")),
            _ => {}
        }
        for grant in self.declared() {
            if let Err(e) = grant.check() {
                out.push(e);
            }
        }
        for s in &self.settings {
            if s.kind == SettingType::Choice && s.options.is_empty() {
                out.push(format!("Its setting {} is a choice of nothing", s.key));
            }
        }
        out
    }

    /// Every permission it declares.
    pub fn declared(&self) -> Vec<Grant> {
        let p = &self.permissions;
        let mut out = Vec::new();
        out.extend(p.connect.iter().cloned().map(Grant::Connect));
        out.extend(p.read.iter().cloned().map(Grant::Read));
        out.extend(p.change.iter().cloned().map(Grant::Change));
        out.extend(p.bus.iter().cloned().map(Grant::Bus));
        out.extend(p.system_bus.iter().cloned().map(Grant::SystemBus));
        out.extend(p.run.iter().cloned().map(Grant::Run));
        out
    }

    /// Whether `other` is a later version than this one.
    pub fn older_than(&self, other: &Manifest) -> bool {
        let parts = |v: &str| -> Vec<u64> { v.split('.').filter_map(|p| p.parse().ok()).collect() };
        parts(&self.version) < parts(&other.version)
    }

    pub fn cells(&self) -> Vec<(i32, i32)> {
        self.sizes.iter().filter_map(|s| size(s)).collect()
    }

    /// What it runs, as the consent panel shows it.
    pub fn command(&self) -> String {
        match (&self.exec, &self.script) {
            (Some(exec), _) => exec.join(" "),
            (_, Some(script)) => format!("python3 {script}"),
            _ => String::new(),
        }
    }

    /// The program and arguments as run inside the sandbox, from `folder`.
    pub fn argv(&self, folder: &Path) -> Vec<String> {
        match (&self.exec, &self.script) {
            (Some(exec), _) => {
                let mut argv = exec.clone();
                // A program named by a path is the folder's.
                if let Some(first) = argv.first_mut()
                    && first.contains('/')
                {
                    *first = folder.join(first.trim_start_matches("./")).to_string_lossy().into_owned();
                }
                argv
            }
            (_, Some(script)) => vec!["python3".into(), folder.join(script).to_string_lossy().into_owned()],
            _ => Vec::new(),
        }
    }
}

/// The user's providers' folder: `~/.config/isle/widgets/<id>/`.
pub fn own_folder() -> PathBuf {
    let config = std::env::var_os("ISLE_CONFIG_DIR").map(PathBuf::from).unwrap_or_else(|| {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"))
            .join("isle")
    });
    config.join("widgets")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: &str) -> Manifest {
        serde_json::from_str(json).unwrap()
    }

    const GOOD: &str = r#"{
        "id": "moon", "name": "Moon", "version": "1.0.0", "sizes": ["2x1", "2x2"],
        "default": "2x2", "template": "number", "exec": ["./moon.sh"], "interval": 3600
    }"#;

    #[test]
    fn a_good_manifest_has_no_problems() {
        let m = manifest(GOOD);
        assert!(m.problems(Path::new("/x")).is_empty(), "{:?}", m.problems(Path::new("/x")));
        assert_eq!(m.cells(), vec![(2, 1), (2, 2)]);
        assert_eq!(m.argv(Path::new("/w/moon")), vec!["/w/moon/moon.sh".to_owned()]);
        assert_eq!(m.command(), "./moon.sh");
    }

    #[test]
    fn its_problems_are_named() {
        let m = manifest(r#"{
            "id": "Moon Phase", "name": "", "version": "1.0", "sizes": ["2x1", "40x1"],
            "default": "4x4", "template": "text", "exec": ["x"], "script": "y.py",
            "interval": 1, "permissions": {"connect": ["https://x"], "run": ["/bin/sh"]}
        }"#);
        let problems = m.problems(Path::new("/nowhere"));
        for part in ["not an id", "no name", "not a version", "sizes", "default size", "exec or script", "under 5", "not a host", "not a command"] {
            assert!(problems.iter().any(|p| p.contains(part)), "{part}: {problems:?}");
        }
    }

    #[test]
    fn unknown_permission_kinds_are_refused() {
        let json = GOOD.replace(r#""interval": 3600"#, r#""interval": 3600, "permissions": {"camera": ["x"]}"#);
        assert!(serde_json::from_str::<Manifest>(&json).is_err());
    }
}
