//! `tokens.kdl`: the design tokens, typed, merged and resolved (`data/tokens/schema.md`).

mod flat;

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};

pub use flat::{Colour, Mode, Shadow};
use flat::{Entry, Raw};

include!(concat!(env!("OUT_DIR"), "/tokens.rs"));

/// The file Isle ships, built in so a missing install never leaves the shell without tokens.
pub const SHIPPED: &str = include_str!("../../../data/tokens/tokens.kdl");

/// The newest `version` this build reads.
pub const VERSION: u32 = 1;

/// Why a token file is not in force, at a place in it.
#[derive(Clone, Debug, PartialEq)]
pub struct TokenError {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.file.display(),
            self.line,
            self.column,
            self.message
        )
    }
}

impl std::error::Error for TokenError {}

/// The user's choices within the design, from `settings.kdl`'s `appearance` section.
#[derive(Clone, Debug, PartialEq)]
pub struct Appearance {
    pub mode: Mode,
    pub accent: String,
    /// The Glass setting: 0 clear, 0.5 the default, 1 tinted.
    pub glass: f32,
    pub text_scale: f32,
    pub increase_contrast: bool,
    pub reduce_motion: bool,
    pub reduce_transparency: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            mode: Mode::Dark,
            accent: String::new(),
            glass: 0.5,
            text_scale: 1.0,
            increase_contrast: false,
            reduce_motion: false,
            reduce_transparency: false,
        }
    }
}

/// The tokens resolved for an appearance, which every surface and Isle app draws from.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub tokens: Tokens,
    pub appearance: Appearance,
}

/// A spring, as the motion tokens give it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub stiffness: f32,
    pub damping: f32,
}

impl Theme {
    pub fn spring_open(&self) -> Spring {
        Spring {
            stiffness: self.tokens.motion_open_stiffness,
            damping: self.tokens.motion_open_damping,
        }
    }

    pub fn spring_exit(&self) -> Spring {
        Spring {
            stiffness: self.tokens.motion_exit_stiffness,
            damping: self.tokens.motion_exit_damping,
        }
    }

    pub fn spring_morph(&self) -> Spring {
        Spring {
            stiffness: self.tokens.motion_morph_stiffness,
            damping: self.tokens.motion_morph_damping,
        }
    }

    pub fn millis(ms: u32) -> Duration {
        Duration::from_millis(ms.into())
    }
}

/// A set of token files laid one over another: the shipped file, then the user's.
#[derive(Clone, Debug)]
pub struct Source {
    entries: Vec<Entry>,
}

impl Source {
    pub fn shipped() -> Self {
        let entries = flat::parse(SHIPPED).expect("the shipped tokens.kdl parses");
        Self { entries }
    }

    /// The shipped tokens with `text`, a user's file at `file`, laid over them. Tokens the
    /// shipped file does not have are left out and returned, so a newer file degrades.
    pub fn with_user(text: &str, file: &Path) -> Result<(Self, Vec<String>), TokenError> {
        let error = |offset: usize, message: String| {
            let (line, column) = line_column(text, offset);
            TokenError {
                file: file.to_owned(),
                line,
                column,
                message,
            }
        };
        let user = flat::parse(text).map_err(|p| error(p.offset, p.message))?;
        match user.iter().find(|e| e.path == ["version"]) {
            Some(Entry {
                value: Raw::Number(v),
                offset,
                ..
            }) if *v as u32 > VERSION => {
                return Err(error(
                    *offset,
                    format!("version {v} is newer than this Isle reads ({VERSION})"),
                ));
            }
            _ => {}
        }
        let shipped = Self::shipped();
        let mut unknown = Vec::new();
        let mut known = Vec::new();
        for entry in user {
            if shipped.entries.iter().any(|e| e.path == entry.path) {
                known.push(entry);
            } else {
                unknown.push(entry.path.join("-"));
            }
        }
        let merged = Self {
            entries: flat::merge(&shipped.entries, &known),
        };
        // A value of the wrong kind is refused here, not when a surface asks for it.
        for mode in [Mode::Dark, Mode::Light] {
            let accent = merged.default_accent();
            let map = flat::resolve(&merged.entries, mode, &accent, true);
            if let Err(message) = Tokens::from_map(&map) {
                let offset = merged
                    .entries
                    .iter()
                    .find(|e| message.contains(&e.path.join("-")) || message.contains(&strip(e)))
                    .map_or(0, |e| e.offset);
                return Err(error(offset, message));
            }
        }
        Ok((merged, unknown))
    }

    /// The accents the user can choose from, in the file's order.
    pub fn accents(&self) -> Vec<String> {
        flat::accents(&self.entries)
    }

    pub fn default_accent(&self) -> String {
        self.entries
            .iter()
            .find(|e| e.path == ["accent", "default"])
            .and_then(|e| match &e.value {
                Raw::Text(t) => Some(t.clone()),
                Raw::Number(_) => None,
            })
            .unwrap_or_else(|| "blue".into())
    }

    /// The tokens for `appearance`; an accent the file does not have falls back to its default.
    pub fn theme(&self, appearance: &Appearance) -> Result<Theme, String> {
        let mut appearance = appearance.clone();
        if !self.accents().contains(&appearance.accent) {
            appearance.accent = self.default_accent();
        }
        let map = flat::resolve(
            &self.entries,
            appearance.mode,
            &appearance.accent,
            appearance.increase_contrast,
        );
        let mut tokens = Tokens::from_map(&map)?;
        // The glass runs from clear through the default to tinted along the setting; Increase
        // Contrast takes the tinted end and Reduce Transparency the solid form.
        tokens.material_glass = if appearance.reduce_transparency {
            tokens.material_solid
        } else if appearance.increase_contrast {
            tokens.material_glass_tinted
        } else if appearance.glass < 0.5 {
            tokens
                .material_glass_clear
                .mix(tokens.material_glass, appearance.glass / 0.5)
        } else {
            tokens
                .material_glass
                .mix(tokens.material_glass_tinted, (appearance.glass - 0.5) / 0.5)
        };
        appearance.text_scale = appearance.text_scale.clamp(1.0, tokens.type_scale_max);
        Ok(Theme { tokens, appearance })
    }
}

/// An entry's key with its appearance segment taken out.
fn strip(entry: &Entry) -> String {
    entry
        .path
        .iter()
        .filter(|s| *s != "dark" && *s != "light")
        .cloned()
        .collect::<Vec<_>>()
        .join("-")
}

fn line_column(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset.min(text.len())];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    (line, column)
}

mod get {
    use super::*;

    fn value<'a>(map: &'a BTreeMap<String, Raw>, key: &str) -> Result<&'a Raw, String> {
        map.get(key).ok_or_else(|| format!("{key} is missing"))
    }

    pub fn colour(map: &BTreeMap<String, Raw>, key: &str) -> Result<Colour, String> {
        match value(map, key)? {
            Raw::Text(t) => Colour::parse(t).ok_or_else(|| format!("{key} is not a colour: {t}")),
            Raw::Number(_) => Err(format!("{key} is a number, not a colour")),
        }
    }

    pub fn shadows(map: &BTreeMap<String, Raw>, key: &str) -> Result<Vec<Shadow>, String> {
        match value(map, key)? {
            Raw::Text(t) => {
                flat::parse_shadows(t).ok_or_else(|| format!("{key} is not a shadow list: {t}"))
            }
            Raw::Number(_) => Err(format!("{key} is a number, not a shadow")),
        }
    }

    pub fn count(map: &BTreeMap<String, Raw>, key: &str) -> Result<u32, String> {
        match value(map, key)? {
            Raw::Number(n) if *n >= 0.0 => Ok(*n as u32),
            _ => Err(format!("{key} is not a whole number")),
        }
    }

    pub fn number(map: &BTreeMap<String, Raw>, key: &str) -> Result<f32, String> {
        match value(map, key)? {
            Raw::Number(n) => Ok(*n as f32),
            Raw::Text(_) => Err(format!("{key} is text, not a number")),
        }
    }

    pub fn text(map: &BTreeMap<String, Raw>, key: &str) -> Result<String, String> {
        match value(map, key)? {
            Raw::Text(t) => Ok(t.clone()),
            Raw::Number(_) => Err(format!("{key} is a number, not text")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_appearance() -> Vec<Appearance> {
        let mut all = Vec::new();
        for mode in [Mode::Dark, Mode::Light] {
            for contrast in [false, true] {
                for accent in Source::shipped().accents() {
                    all.push(Appearance {
                        mode,
                        accent,
                        increase_contrast: contrast,
                        ..Default::default()
                    });
                }
            }
        }
        all
    }

    #[test]
    fn every_shipped_token_resolves_for_every_appearance() {
        let source = Source::shipped();
        assert!(Tokens::KEYS.len() > 300);
        for appearance in every_appearance() {
            source.theme(&appearance).unwrap();
        }
    }

    #[test]
    fn appearances_choose_their_values() {
        let source = Source::shipped();
        let dark = source.theme(&Appearance::default()).unwrap();
        assert_eq!(Colour::parse("#1e1f23"), Some(dark.tokens.colour_window));
        assert_eq!(dark.appearance.accent, "blue");
        assert_eq!(Colour::parse("#2f6fe4"), Some(dark.tokens.colour_accent));
        let light = source
            .theme(&Appearance {
                mode: Mode::Light,
                accent: "green".into(),
                increase_contrast: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(Colour::parse("#f6f6f7"), Some(light.tokens.colour_window));
        assert_eq!(Colour::parse("#1a7d48"), Some(light.tokens.colour_accent));
        assert_eq!(
            Colour::parse("rgba(27,28,31,0.88)"),
            Some(light.tokens.colour_text_2),
            "Increase Contrast replaces text-2"
        );
        assert_eq!(light.tokens.material_glass, light.tokens.material_glass_tinted);
        assert_eq!(dark.tokens.shadow_window.len(), 2);
        assert_eq!(dark.tokens.motion_open_stiffness, 420.0);
        assert_eq!(dark.tokens.timing_osd_ms, 1500);
    }

    #[test]
    fn a_user_file_overrides_by_path_and_ignores_what_it_does_not_know() {
        let text = "version 1\ncolour { dark { window \"#102030\" } }\nnew-thing 3\n";
        let (source, unknown) = Source::with_user(text, Path::new("t.kdl")).unwrap();
        assert_eq!(unknown, vec!["new-thing".to_string()]);
        let theme = source.theme(&Appearance::default()).unwrap();
        assert_eq!(Colour::parse("#102030"), Some(theme.tokens.colour_window));
        let light = source
            .theme(&Appearance {
                mode: Mode::Light,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(Colour::parse("#f6f6f7"), Some(light.tokens.colour_window));
    }

    #[test]
    fn broken_and_newer_files_are_refused_with_a_place() {
        let err = Source::with_user("colour {\n  dark { window \"#1020\n", Path::new("t.kdl"))
            .unwrap_err();
        assert!(err.line >= 2, "{err}");
        let err = Source::with_user("version 2\n", Path::new("t.kdl")).unwrap_err();
        assert!(err.message.contains("newer"), "{err}");
        let err = Source::with_user("colour { dark { window \"teal\" } }\n", Path::new("t.kdl"))
            .unwrap_err();
        assert!(err.message.contains("colour-window"), "{err}");
        assert_eq!(err.line, 1);
    }

    #[test]
    fn the_glass_setting_runs_from_clear_to_tinted() {
        let source = Source::shipped();
        let at = |glass, reduce_transparency| {
            source
                .theme(&Appearance {
                    glass,
                    reduce_transparency,
                    ..Default::default()
                })
                .unwrap()
                .tokens
        };
        assert_eq!(at(0.0, false).material_glass, at(0.0, false).material_glass_clear);
        assert_eq!(
            at(0.5, false).material_glass,
            Colour::parse("rgba(34,35,40,0.76)").unwrap()
        );
        assert_eq!(at(1.0, false).material_glass, at(1.0, false).material_glass_tinted);
        assert_eq!(at(0.5, true).material_glass, at(0.5, true).material_solid);
    }
}
