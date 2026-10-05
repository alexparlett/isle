//! A keyboard's keymap: its XKB names from the user's layouts and the keyboard's profile,
//! and the keymap compiled to text for the seat.

use std::{collections::HashSet, path::PathBuf};

use xkbcommon::xkb;

use crate::profile::{Profile, guess_iso};

/// The XKB rule names a keymap is compiled from; keyboards with equal specs share a keymap.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeymapSpec {
    pub model: String,
    pub layout: String,
    pub variant: String,
    pub options: String,
}

/// The layouts the user types, from `keyboard` in settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layouts {
    pub layouts: Vec<String>,
    pub variants: Vec<String>,
    pub options: Vec<String>,
}

/// The layouts XKB has a `mac` variant of, from the rules' list.
#[derive(Clone, Debug, Default)]
pub struct MacVariants(HashSet<String>);

impl MacVariants {
    /// Reads `evdev.lst` under `$XKB_CONFIG_ROOT`, else `/usr/share/X11/xkb`.
    pub fn load() -> Self {
        let root = std::env::var_os("XKB_CONFIG_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr/share/X11/xkb"));
        match std::fs::read_to_string(root.join("rules/evdev.lst")) {
            Ok(text) => Self::parse(&text),
            Err(err) => {
                tracing::warn!("no XKB rules list, so no Mac variants: {err}");
                Self::default()
            }
        }
    }

    /// The `! variant` section's lines read `  mac   gb: English (UK, Macintosh)`.
    pub fn parse(text: &str) -> Self {
        let mut in_variants = false;
        let mut layouts = HashSet::new();
        for line in text.lines() {
            if let Some(section) = line.strip_prefix("! ") {
                in_variants = section.trim() == "variant";
                continue;
            }
            if !in_variants {
                continue;
            }
            let mut words = line.split_whitespace();
            if words.next() == Some("mac")
                && let Some(layout) = words.next().and_then(|w| w.strip_suffix(':'))
            {
                layouts.insert(layout.to_owned());
            }
        }
        Self(layouts)
    }

    pub fn has(&self, layout: &str) -> bool {
        self.0.contains(layout)
    }
}

/// Whether the keyboard has the ISO key, when its settings say rather than guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Iso {
    Guess,
    Iso,
    Ansi,
}

impl KeymapSpec {
    /// A Mac keyboard takes each layout's `mac` variant where XKB has one and the user named
    /// none (D61); a PC keyboard takes the layouts as they are.
    pub fn resolve(layouts: &Layouts, profile: Profile, iso: Iso, mac: &MacVariants) -> Self {
        let names: Vec<&str> = if layouts.layouts.is_empty() {
            vec!["us"]
        } else {
            layouts.layouts.iter().map(String::as_str).collect()
        };
        let variants: Vec<String> = names
            .iter()
            .enumerate()
            .map(|(i, layout)| {
                let named = layouts.variants.get(i).map(String::as_str).unwrap_or("");
                if named.is_empty() && profile == Profile::Mac && mac.has(layout) {
                    "mac".to_owned()
                } else {
                    named.to_owned()
                }
            })
            .collect();
        let iso = match iso {
            Iso::Guess => guess_iso(names[0]),
            Iso::Iso => true,
            Iso::Ansi => false,
        };
        KeymapSpec {
            model: if iso { "pc105" } else { "pc104" }.to_owned(),
            layout: names.join(","),
            variant: variants.join(","),
            options: layouts.options.join(","),
        }
    }

    /// `gb(mac)`, `us,de(nodeadkeys)`: the layouts as people write them.
    pub fn describe(&self) -> String {
        let variants: Vec<&str> = self.variant.split(',').collect();
        self.layout
            .split(',')
            .enumerate()
            .map(|(i, layout)| match variants.get(i) {
                Some(v) if !v.is_empty() => format!("{layout}({v})"),
                _ => layout.to_owned(),
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn compile(&self, context: &xkb::Context) -> Option<xkb::Keymap> {
        xkb::Keymap::new_from_names(
            context,
            "evdev",
            &self.model,
            &self.layout,
            &self.variant,
            (!self.options.is_empty()).then(|| self.options.clone()),
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
    }

    /// The keymap as text for the seat; `None` when XKB does not know a name in it.
    pub fn compile_text(&self) -> Option<String> {
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        self.compile(&context)
            .map(|keymap| keymap.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "! model\n  pc105  Generic 105-key PC\n\n! layout\n  gb  English (UK)\n\n\
                        ! variant\n  mac             gb: English (UK, Macintosh)\n  \
                        mac_intl        gb: English (UK, Macintosh, intl.)\n  \
                        mac             us: English (Macintosh, ABC, ANSI)\n\n\
                        ! option\n  mac   Not a variant\n";

    fn layouts(names: &[&str]) -> Layouts {
        Layouts {
            layouts: names.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn the_rules_list_names_mac_variants() {
        let mac = MacVariants::parse(LIST);
        assert!(mac.has("gb") && mac.has("us"));
        assert!(!mac.has("ru"));
    }

    #[test]
    fn a_mac_keyboard_takes_the_mac_variant_where_there_is_one() {
        let mac = MacVariants::parse(LIST);
        let spec = KeymapSpec::resolve(&layouts(&["gb", "ru"]), Profile::Mac, Iso::Guess, &mac);
        assert_eq!(spec.layout, "gb,ru");
        assert_eq!(spec.variant, "mac,");
        assert_eq!(spec.model, "pc105");
        assert_eq!(spec.describe(), "gb(mac),ru");
        let pc = KeymapSpec::resolve(&layouts(&["gb"]), Profile::Pc, Iso::Guess, &mac);
        assert_eq!(pc.describe(), "gb");
        let us = KeymapSpec::resolve(&layouts(&["us"]), Profile::Pc, Iso::Guess, &mac);
        assert_eq!(us.model, "pc104");
    }

    #[test]
    fn a_named_variant_is_kept() {
        let mac = MacVariants::parse(LIST);
        let mut l = layouts(&["gb"]);
        l.variants = vec!["extd".into()];
        let spec = KeymapSpec::resolve(&l, Profile::Mac, Iso::Ansi, &mac);
        assert_eq!(spec.describe(), "gb(extd)");
        assert_eq!(spec.model, "pc104");
    }

    #[test]
    fn specs_compile_and_unknown_names_do_not() {
        let mac = MacVariants::load();
        let spec = KeymapSpec::resolve(&layouts(&["gb"]), Profile::Mac, Iso::Guess, &mac);
        let text = spec.compile_text().expect("gb(mac) compiles");
        assert!(text.contains("xkb_keymap"));
        let bad = KeymapSpec::resolve(&layouts(&["nowhere"]), Profile::Pc, Iso::Guess, &mac);
        assert!(bad.compile_text().is_none());
    }
}
