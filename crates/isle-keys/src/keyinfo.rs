//! What a keymap says about each key: its role, the symbol chords match, whether it has a
//! third-level symbol, and which key gives a symbol. Codes are evdev's.

use std::collections::HashMap;

use xkbcommon::xkb::{self, Keysym, keysyms};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    /// Super, which Isle reads as Cmd.
    Cmd,
    /// Alt, which Isle reads as Option.
    Option,
    Ctrl,
    Shift,
    /// AltGr and the Mac variant's right Option: a key's third level.
    Level3,
    Key,
}

#[derive(Clone, Debug, Default)]
pub struct KeyInfo {
    /// By evdev code.
    keys: HashMap<u32, Key>,
    /// The lowest code giving each symbol at the first level of the first layout.
    codes: HashMap<u32, u32>,
}

#[derive(Clone, Copy, Debug)]
struct Key {
    role: Role,
    /// The first-level symbol in the first layout that gives a Latin one, else the first.
    latin: Keysym,
    level3: bool,
}

fn role_of(sym: u32) -> Role {
    match sym {
        keysyms::KEY_Super_L | keysyms::KEY_Super_R | keysyms::KEY_Meta_L | keysyms::KEY_Meta_R => {
            Role::Cmd
        }
        keysyms::KEY_Alt_L | keysyms::KEY_Alt_R => Role::Option,
        keysyms::KEY_Control_L | keysyms::KEY_Control_R => Role::Ctrl,
        keysyms::KEY_Shift_L | keysyms::KEY_Shift_R => Role::Shift,
        keysyms::KEY_ISO_Level3_Shift | keysyms::KEY_Mode_switch => Role::Level3,
        _ => Role::Key,
    }
}

fn is_latin(sym: Keysym) -> bool {
    (0x20..0x7f).contains(&sym.raw())
}

impl KeyInfo {
    pub fn new(keymap: &xkb::Keymap) -> Self {
        let mut info = KeyInfo::default();
        let layouts = keymap.num_layouts();
        let first = |code: xkb::Keycode, layout: u32| -> Option<Keysym> {
            keymap
                .key_get_syms_by_level(code, layout, 0)
                .first()
                .copied()
                .filter(|s| s.raw() != keysyms::KEY_NoSymbol)
        };
        keymap.key_for_each(|_, code| {
            let Some(base) = first(code, 0) else {
                return;
            };
            let latin = (0..layouts)
                .filter_map(|l| first(code, l))
                .find(|s| is_latin(*s))
                .unwrap_or(base);
            let level3 = keymap
                .key_get_syms_by_level(code, 0, 2)
                .iter()
                .any(|s| s.raw() != keysyms::KEY_NoSymbol);
            let evdev = code.raw() - 8;
            info.keys.insert(
                evdev,
                Key {
                    role: role_of(base.raw()),
                    latin,
                    level3,
                },
            );
            info.codes.entry(base.raw()).or_insert(evdev);
        });
        info
    }

    pub fn role(&self, code: u32) -> Role {
        self.keys.get(&code).map_or(Role::Key, |k| k.role)
    }

    /// The symbol chords match a key by.
    pub fn sym(&self, code: u32) -> Option<Keysym> {
        self.keys.get(&code).map(|k| k.latin)
    }

    pub fn has_level3(&self, code: u32) -> bool {
        self.keys.get(&code).is_some_and(|k| k.level3)
    }

    /// The key that gives `sym` with no modifier.
    pub fn code_for(&self, sym: Keysym) -> Option<u32> {
        self.codes.get(&sym.raw()).copied()
    }

    /// The key Isle presses to send a role: the usual left-hand key, or right Alt for the
    /// third level, when the keymap gives it that role; else any key that does.
    pub fn code_for_role(&self, role: Role) -> Option<u32> {
        let (usual, sym) = match role {
            Role::Cmd => (125, keysyms::KEY_Super_L),
            Role::Option => (56, keysyms::KEY_Alt_L),
            Role::Ctrl => (29, keysyms::KEY_Control_L),
            Role::Shift => (42, keysyms::KEY_Shift_L),
            Role::Level3 => (100, keysyms::KEY_ISO_Level3_Shift),
            Role::Key => return None,
        };
        if self.role(usual) == role {
            return Some(usual);
        }
        self.code_for(Keysym::from(sym))
    }
}

#[cfg(test)]
pub(crate) fn info_for(layout: &str, variant: &str) -> KeyInfo {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap = xkb::Keymap::new_from_names(
        &context,
        "evdev",
        "pc105",
        layout,
        variant,
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .expect("the fixture keymap compiles");
    KeyInfo::new(&keymap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_symbols_and_codes_come_from_the_keymap() {
        let gb_mac = info_for("gb", "mac");
        assert_eq!(gb_mac.role(125), Role::Cmd);
        assert_eq!(gb_mac.role(56), Role::Option);
        assert_eq!(gb_mac.role(100), Role::Level3);
        assert_eq!(gb_mac.role(29), Role::Ctrl);
        assert_eq!(gb_mac.sym(46), Some(Keysym::from(keysyms::KEY_c)));
        assert!(
            gb_mac.has_level3(4),
            "Option+3 types # on a UK Mac keyboard"
        );
        // The Mac variant puts grave left of Z.
        assert_eq!(gb_mac.code_for(Keysym::from(keysyms::KEY_grave)), Some(86));
        assert_eq!(gb_mac.code_for_role(Role::Level3), Some(100));

        let ru = info_for("ru,us", ",");
        assert_eq!(
            ru.sym(46),
            Some(Keysym::from(keysyms::KEY_c)),
            "Cmd+C on Russian"
        );

        let fr = info_for("fr", "");
        assert_eq!(fr.sym(16), Some(Keysym::from(keysyms::KEY_a)), "AZERTY's A");
    }
}
