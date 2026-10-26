//! The shortcut set's rule across keys: every chord reads as one, and none is bound twice.

use std::collections::HashMap;

use isle_keys::chord::{Chord, Mods};

use crate::{CheckError, schema::Settings};

pub fn check(settings: &Settings) -> Result<(), CheckError> {
    let shortcuts = &settings.shortcuts;
    let mut bound: HashMap<Chord, &str> = HashMap::new();
    let mut buttons: HashMap<isle_keys::chord::ButtonChord, &str> = HashMap::new();
    let mut bind = |chord: Chord, id: &'static str| match bound.insert(chord, id) {
        Some(other) if other != id => Err(CheckError {
            message: format!("{chord} is bound to both {other} and {id}"),
            key: format!("shortcuts.{id}"),
        }),
        _ => Ok(()),
    };
    for (id, chords) in shortcuts.chords() {
        for text in chords {
            // A mouse button is a chord too.
            if let Some(button) = isle_keys::chord::ButtonChord::parse(text) {
                match buttons.insert(button, id) {
                    Some(other) if other != id => {
                        return Err(CheckError {
                            message: format!("{} is bound to both {other} and {id}", button.words()),
                            key: format!("shortcuts.{id}"),
                        });
                    }
                    _ => continue,
                }
            }
            let chord = Chord::parse(text).ok_or_else(|| CheckError {
                message: format!("`{text}` is not a chord"),
                key: format!("shortcuts.{id}"),
            })?;
            bind(chord, id)?;
        }
    }
    let jump = Mods::parse(&shortcuts.desktop_jump).ok_or_else(|| CheckError {
        message: format!("`{}` is not a set of modifiers", shortcuts.desktop_jump),
        key: "shortcuts.desktop-jump".into(),
    })?;
    if !jump.is_empty() {
        for n in 1..=9 {
            let sym = isle_keys::xkb::utf32_to_keysym(char::from_digit(n, 10).unwrap() as u32);
            bind(Chord::new(jump, sym), "desktop-jump")?;
        }
    }
    if Mods::parse(&shortcuts.window_drag).is_none() {
        return Err(CheckError {
            message: format!("`{}` is not a set of modifiers", shortcuts.window_drag),
            key: "shortcuts.window-drag".into(),
        });
    }
    Ok(())
}
