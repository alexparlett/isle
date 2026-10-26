//! Chords: modifiers and a key symbol, written `Shift+Cmd+K`, `⇧⌘K` or P4's `Super+Shift+K`.

use std::fmt;

use xkbcommon::xkb::{self, Keysym, keysyms};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mods {
    pub ctrl: bool,
    pub option: bool,
    pub shift: bool,
    pub cmd: bool,
}

impl Mods {
    pub const NONE: Mods = Mods {
        ctrl: false,
        option: false,
        shift: false,
        cmd: false,
    };

    pub fn is_empty(self) -> bool {
        self == Mods::NONE
    }

    /// Modifiers alone, `Ctrl+Cmd`; `None` when a part is not a modifier.
    pub fn parse(text: &str) -> Option<Mods> {
        let mut mods = Mods::default();
        for part in text.split('+').map(str::trim).filter(|p| !p.is_empty()) {
            match part.to_ascii_lowercase().as_str() {
                "cmd" | "command" | "super" | "logo" | "⌘" => mods.cmd = true,
                "option" | "opt" | "alt" | "⌥" => mods.option = true,
                "ctrl" | "control" | "⌃" => mods.ctrl = true,
                "shift" | "⇧" => mods.shift = true,
                _ => return None,
            }
        }
        Some(mods)
    }

    /// Whether every modifier of `other` is held in these.
    pub fn contains(self, other: Mods) -> bool {
        (!other.ctrl || self.ctrl)
            && (!other.option || self.option)
            && (!other.shift || self.shift)
            && (!other.cmd || self.cmd)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub mods: Mods,
    pub sym: Keysym,
}

/// Names for keys whose XKB names people do not write.
const ALIASES: &[(&str, u32)] = &[
    ("left", keysyms::KEY_Left),
    ("right", keysyms::KEY_Right),
    ("up", keysyms::KEY_Up),
    ("down", keysyms::KEY_Down),
    ("backspace", keysyms::KEY_BackSpace),
    ("delete", keysyms::KEY_Delete),
    ("enter", keysyms::KEY_Return),
    ("return", keysyms::KEY_Return),
    ("esc", keysyms::KEY_Escape),
    ("escape", keysyms::KEY_Escape),
    ("space", keysyms::KEY_space),
    ("tab", keysyms::KEY_Tab),
    ("pageup", keysyms::KEY_Prior),
    ("pagedown", keysyms::KEY_Next),
    ("home", keysyms::KEY_Home),
    ("end", keysyms::KEY_End),
    ("`", keysyms::KEY_grave),
    ("-", keysyms::KEY_minus),
    ("−", keysyms::KEY_minus),
    ("=", keysyms::KEY_equal),
    ("+", keysyms::KEY_plus),
    (",", keysyms::KEY_comma),
    (".", keysyms::KEY_period),
    ("/", keysyms::KEY_slash),
    ("\\", keysyms::KEY_backslash),
    (";", keysyms::KEY_semicolon),
    ("'", keysyms::KEY_apostrophe),
    ("[", keysyms::KEY_bracketleft),
    ("]", keysyms::KEY_bracketright),
    ("←", keysyms::KEY_Left),
    ("→", keysyms::KEY_Right),
    ("↑", keysyms::KEY_Up),
    ("↓", keysyms::KEY_Down),
];

fn key_from_name(name: &str) -> Option<Keysym> {
    let lower = name.to_lowercase();
    if let Some((_, sym)) = ALIASES.iter().find(|(n, _)| *n == lower) {
        return Some(Keysym::from(*sym));
    }
    let sym = xkb::keysym_from_name(name, xkb::KEYSYM_CASE_INSENSITIVE);
    if sym.raw() != keysyms::KEY_NoSymbol {
        return Some(sym);
    }
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => {
            let sym = xkb::utf32_to_keysym(c.to_lowercase().next().unwrap_or(c) as u32);
            (sym.raw() != keysyms::KEY_NoSymbol).then_some(sym)
        }
        _ => None,
    }
}

impl Chord {
    pub fn new(mods: Mods, sym: Keysym) -> Self {
        Chord { mods, sym }
    }

    /// Reads a chord in Mac words, symbols, or P4's names; `None` when it names no key.
    pub fn parse(text: &str) -> Option<Chord> {
        let text = text.trim();
        let mut mods = Mods::default();
        let mut rest = text;
        // Leading symbols: ⌃⌥⇧⌘ before the key, as macOS writes them.
        loop {
            let mut chars = rest.chars();
            match chars.next() {
                Some('⌃') => mods.ctrl = true,
                Some('⌥') => mods.option = true,
                Some('⇧') => mods.shift = true,
                Some('⌘') => mods.cmd = true,
                _ => break,
            }
            rest = chars.as_str();
        }
        // A `+` key is written last after a `+`: `Cmd++`.
        let (mod_part, key) = match rest.strip_suffix("++") {
            Some(before) => (before, "+"),
            None => match rest.rfind('+') {
                Some(i) if i + 1 < rest.len() => (&rest[..i], &rest[i + 1..]),
                _ => ("", rest),
            },
        };
        for part in mod_part.split('+').map(str::trim).filter(|p| !p.is_empty()) {
            match part.to_ascii_lowercase().as_str() {
                "cmd" | "command" | "super" | "logo" => mods.cmd = true,
                "option" | "opt" | "alt" => mods.option = true,
                "ctrl" | "control" => mods.ctrl = true,
                "shift" => mods.shift = true,
                _ => return None,
            }
        }
        let key = key.trim();
        if key.is_empty() {
            return None;
        }
        Some(Chord {
            mods,
            sym: key_from_name(key)?,
        })
    }

    /// In words, modifiers in macOS's order: `Ctrl+Option+Shift+Cmd+K`.
    pub fn words(&self) -> String {
        let mut parts = Vec::new();
        if self.mods.ctrl {
            parts.push("Ctrl".to_owned());
        }
        if self.mods.option {
            parts.push("Option".to_owned());
        }
        if self.mods.shift {
            parts.push("Shift".to_owned());
        }
        if self.mods.cmd {
            parts.push("Cmd".to_owned());
        }
        parts.push(key_name(self.sym));
        parts.join("+")
    }
}

/// A mouse button beyond the left, right and middle ones, with modifiers: a shortcut's
/// pointer route (`p1/structure/mission-control.md`, Mouse routes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ButtonChord {
    pub mods: Mods,
    /// The evdev code.
    pub button: u32,
}

/// The buttons a shortcut may take, by name and evdev code.
pub const BUTTONS: &[(&str, u32)] = &[
    ("Back", 0x113),
    ("Forward", 0x114),
    ("Button6", 0x115),
    ("Button7", 0x116),
    ("Button8", 0x117),
];

impl ButtonChord {
    /// `Back`, `Ctrl+Forward`, `Button6`; never a key.
    pub fn parse(text: &str) -> Option<ButtonChord> {
        let text = text.trim();
        let (mod_part, name) = match text.rfind('+') {
            Some(i) => (&text[..i], &text[i + 1..]),
            None => ("", text),
        };
        let button = BUTTONS
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name.trim()) || n.replace("Button", "Button ").eq_ignore_ascii_case(name.trim()))?
            .1;
        Some(ButtonChord {
            mods: Mods::parse(mod_part)?,
            button,
        })
    }

    pub fn words(&self) -> String {
        let mut parts = Vec::new();
        for (on, word) in [
            (self.mods.ctrl, "Ctrl"),
            (self.mods.option, "Option"),
            (self.mods.shift, "Shift"),
            (self.mods.cmd, "Cmd"),
        ] {
            if on {
                parts.push(word);
            }
        }
        let name = BUTTONS.iter().find(|(_, b)| *b == self.button).map_or("Button", |(n, _)| n);
        parts.push(name);
        parts.join("+")
    }
}

/// A key's name for people: a printable key as its character, upper case for letters.
pub fn key_name(sym: Keysym) -> String {
    if let Some((name, _)) = ALIASES
        .iter()
        .find(|(n, s)| *s == sym.raw() && n.chars().all(|c| c.is_ascii_lowercase()))
    {
        let mut chars = name.chars();
        let first = chars.next().unwrap().to_ascii_uppercase();
        return std::iter::once(first).chain(chars).collect();
    }
    match sym.key_char() {
        Some(c) if !c.is_control() && c != ' ' => c.to_uppercase().collect(),
        _ => xkb::keysym_get_name(sym),
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.words())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(raw: u32) -> Keysym {
        Keysym::from(raw)
    }

    #[test]
    fn mac_words_symbols_and_p4_names_read_alike() {
        let c = Chord::parse("Shift+Cmd+K").unwrap();
        assert!(c.mods.shift && c.mods.cmd && !c.mods.ctrl && !c.mods.option);
        assert_eq!(c.sym, sym(keysyms::KEY_k));
        assert_eq!(Chord::parse("⇧⌘K"), Some(c));
        assert_eq!(Chord::parse("super+shift+k"), Some(c));
        assert_eq!(
            Chord::parse("Ctrl+Option+Cmd+W").unwrap().words(),
            "Ctrl+Option+Cmd+W"
        );
        assert_eq!(Chord::parse("Cmd+`").unwrap().sym, sym(keysyms::KEY_grave));
        assert_eq!(
            Chord::parse("Cmd+grave").unwrap().sym,
            sym(keysyms::KEY_grave)
        );
        assert_eq!(Chord::parse("Cmd++").unwrap().sym, sym(keysyms::KEY_plus));
        assert_eq!(
            Chord::parse("Option+Cmd+−").unwrap().sym,
            sym(keysyms::KEY_minus)
        );
        assert_eq!(
            Chord::parse("Ctrl+Space").unwrap().sym,
            sym(keysyms::KEY_space)
        );
        assert_eq!(Chord::parse("Cmd+Left").unwrap().words(), "Cmd+Left");
        assert_eq!(Chord::parse("Cmd+,").unwrap().words(), "Cmd+,");
        assert!(Chord::parse("Hyper+Q").is_none());
        assert!(Chord::parse("Cmd+Nonsense").is_none());
        assert!(Chord::parse("").is_none());
    }

    #[test]
    fn buttons_read_with_modifiers_and_never_as_keys() {
        let b = ButtonChord::parse("Ctrl+Back").unwrap();
        assert!(b.mods.ctrl && !b.mods.cmd);
        assert_eq!(b.button, 0x113);
        assert_eq!(b.words(), "Ctrl+Back");
        assert_eq!(ButtonChord::parse("Button6").unwrap().button, 0x115);
        assert!(ButtonChord::parse("Ctrl+Left").is_none());
        assert!(Chord::parse("Back").is_none());
    }
}
