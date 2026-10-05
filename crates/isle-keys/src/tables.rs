//! The rules every translated app shares, from v1's `profiles.mac.text`.

use crate::engine::Rules;

/// Line and document motion on Cmd, word motion on Option.
const TEXT: &[(&str, &str)] = &[
    ("Cmd+Left", "Home"),
    ("Cmd+Right", "End"),
    ("Cmd+Up", "Ctrl+Home"),
    ("Cmd+Down", "Ctrl+End"),
    ("Shift+Cmd+Left", "Shift+Home"),
    ("Shift+Cmd+Right", "Shift+End"),
    ("Shift+Cmd+Up", "Ctrl+Shift+Home"),
    ("Shift+Cmd+Down", "Ctrl+Shift+End"),
    ("Option+Left", "Ctrl+Left"),
    ("Option+Right", "Ctrl+Right"),
    ("Shift+Option+Left", "Ctrl+Shift+Left"),
    ("Shift+Option+Right", "Ctrl+Shift+Right"),
    ("Option+Backspace", "Ctrl+Backspace"),
    ("Option+Delete", "Ctrl+Delete"),
    ("Cmd+Backspace", "Shift+Home, Backspace"),
];

pub fn text() -> Rules {
    Rules::from_pairs(TEXT)
}
