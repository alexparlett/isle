//! A keyboard's keymap: its XKB names from the user's layouts and the keyboard's profile,
//! or a client's own keymap, and the keymap compiled to text for the seat.

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use xkbcommon::xkb;

use crate::profile::{Profile, guess_iso};

/// The XKB rule names a keymap is compiled from; keyboards with equal specs share a keymap.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeymapSpec {
    pub model: String,
    pub layout: String,
    pub variant: String,
    pub options: String,
    /// A virtual keyboard's own keymap as XKB text, which the names do not describe.
    pub text: Option<Arc<str>>,
}

/// The layouts the user types, from `keyboard` in settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layouts {
    pub layouts: Vec<String>,
    pub variants: Vec<String>,
    pub options: Vec<String>,
}

impl Layouts {
    /// The system's layout, as `localectl` sets it: `XKB_DEFAULT_*` in the environment
    /// first, then `XKB*` in `/etc/vconsole.conf`, then Xorg's `00-keyboard.conf`; US
    /// English when none says.
    pub fn system() -> Self {
        let split = |v: &str| -> Vec<String> {
            v.split(',').map(|s| s.trim().to_owned()).collect::<Vec<_>>()
        };
        let from = |layout: String, variant: String, options: String| Layouts {
            layouts: split(&layout),
            variants: if variant.is_empty() { Vec::new() } else { split(&variant) },
            options: if options.is_empty() { Vec::new() } else { split(&options) },
        };
        if let Ok(layout) = std::env::var("XKB_DEFAULT_LAYOUT")
            && !layout.is_empty()
        {
            let var = |k: &str| std::env::var(k).unwrap_or_default();
            return from(layout, var("XKB_DEFAULT_VARIANT"), var("XKB_DEFAULT_OPTIONS"));
        }
        let read = |path: &str| std::fs::read_to_string(path).unwrap_or_default();
        if let Some(found) = Self::from_vconsole(&read("/etc/vconsole.conf"))
            .or_else(|| Self::from_xorg(&read("/etc/X11/xorg.conf.d/00-keyboard.conf")))
        {
            return found;
        }
        from("us".into(), String::new(), String::new())
    }

    /// `XKBLAYOUT=gb` and its siblings, as systemd-localed writes them.
    pub fn from_vconsole(text: &str) -> Option<Self> {
        let get = |key: &str| {
            text.lines()
                .filter_map(|l| l.trim().strip_prefix(key)?.strip_prefix('='))
                .map(|v| v.trim().trim_matches('"').to_owned())
                .next()
                .unwrap_or_default()
        };
        Self::from_parts(get("XKBLAYOUT"), get("XKBVARIANT"), get("XKBOPTIONS"))
    }

    /// `Option "XkbLayout" "gb"` lines in Xorg's keyboard class.
    pub fn from_xorg(text: &str) -> Option<Self> {
        let get = |key: &str| {
            text.lines()
                .filter_map(|l| {
                    let mut words = l.split('"').map(str::trim);
                    (words.next() == Some("Option") && words.next() == Some(key))
                        .then(|| words.nth(1).unwrap_or_default().to_owned())
                })
                .next()
                .unwrap_or_default()
        };
        Self::from_parts(get("XkbLayout"), get("XkbVariant"), get("XkbOptions"))
    }

    fn from_parts(layout: String, variant: String, options: String) -> Option<Self> {
        if layout.is_empty() {
            return None;
        }
        let split = |v: &str| v.split(',').map(|s| s.trim().to_owned()).collect::<Vec<_>>();
        Some(Layouts {
            layouts: split(&layout),
            variants: if variant.is_empty() { Vec::new() } else { split(&variant) },
            options: if options.is_empty() { Vec::new() } else { split(&options) },
        })
    }
}

/// A layout XKB offers, with its variants, as the rules' list names them.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub name: String,
    pub description: String,
    /// Each variant's name and description.
    pub variants: Vec<(String, String)>,
}

/// Every layout XKB offers, by description, for choosing one.
pub fn catalogue() -> Vec<Layout> {
    let root = std::env::var_os("XKB_CONFIG_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/X11/xkb"));
    std::fs::read_to_string(root.join("rules/evdev.lst"))
        .map(|text| parse_catalogue(&text))
        .unwrap_or_default()
}

/// `! layout` lines read `  gb   English (UK)`; `! variant` lines `  mac   gb: English (UK,
/// Macintosh)`.
pub fn parse_catalogue(text: &str) -> Vec<Layout> {
    let mut section = "";
    let mut layouts: Vec<Layout> = Vec::new();
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("! ") {
            section = match name.trim() {
                "layout" => "layout",
                "variant" => "variant",
                _ => "",
            };
            continue;
        }
        let line = line.trim();
        let Some((name, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let rest = rest.trim();
        match section {
            "layout" => layouts.push(Layout {
                name: name.to_owned(),
                description: rest.to_owned(),
                variants: Vec::new(),
            }),
            "variant" => {
                if let Some((layout, description)) = rest.split_once(": ")
                    && let Some(found) = layouts.iter_mut().find(|l| l.name == layout)
                {
                    found.variants.push((name.to_owned(), description.to_owned()));
                }
            }
            _ => {}
        }
    }
    layouts.sort_by(|a, b| a.description.cmp(&b.description));
    layouts
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
            text: None,
        }
    }

    /// A client's own keymap, as the XKB text it sent.
    pub fn from_text(text: Arc<str>) -> Self {
        KeymapSpec {
            text: Some(text),
            ..KeymapSpec::default()
        }
    }

    /// `gb(mac)`, `us,de(nodeadkeys)`: the layouts as people write them.
    pub fn describe(&self) -> String {
        if self.text.is_some() {
            return "client".to_owned();
        }
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
        if let Some(text) = &self.text {
            return xkb::Keymap::new_from_string(
                context,
                text.to_string(),
                xkb::KEYMAP_FORMAT_TEXT_V1,
                xkb::KEYMAP_COMPILE_NO_FLAGS,
            );
        }
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

    #[test]
    fn the_catalogue_reads_layouts_and_their_variants() {
        let text = "! model\n  pc105  Generic 105-key PC\n\n! layout\n  us   English (US)\n  gb   English (UK)\n\n! variant\n  mac   gb: English (UK, Macintosh)\n  intl  us: English (US, intl., with dead keys)\n\n! option\n  caps:escape  Make Caps Lock an additional Esc\n";
        let layouts = parse_catalogue(text);
        assert_eq!(layouts.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["gb", "us"]);
        assert_eq!(layouts[0].variants, [("mac".to_owned(), "English (UK, Macintosh)".to_owned())]);
    }

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
    fn the_system_layout_reads_as_localectl_writes_it() {
        let vconsole = "KEYMAP=uk\nXKBLAYOUT=gb,us\nXKBVARIANT=,dvorak\n";
        let l = Layouts::from_vconsole(vconsole).unwrap();
        assert_eq!(l.layouts, vec!["gb", "us"]);
        assert_eq!(l.variants, vec!["", "dvorak"]);
        assert!(Layouts::from_vconsole("KEYMAP=uk\n").is_none());
        let xorg = "Section \"InputClass\"\n        Option \"XkbLayout\" \"gb\"\nEndSection\n";
        assert_eq!(Layouts::from_xorg(xorg).unwrap().layouts, vec!["gb"]);
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
