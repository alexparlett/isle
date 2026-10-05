//! Isle's own keys, aimed at a window and never translated, and the recording grab that
//! Settings captures a chord or detects a keyboard with.

use isle_keys::{
    chord::{Chord, Mods},
    xkb::{self, keysyms},
};
use smithay::input::keyboard::Keysym;
use tokio::sync::oneshot;
use tracing::info;

use crate::state::State;

/// What a recording returns: the chord in Mac words, the shortcut that holds it, and whether
/// it was cancelled. Detect returns the keyboard's name and whether it has the ISO key.
#[derive(Debug, Default)]
pub struct Recorded {
    pub chord: String,
    pub holder: String,
    pub cancelled: bool,
    pub keyboard: String,
    pub iso: bool,
}

pub enum Recording {
    Chord(oneshot::Sender<Recorded>),
    Keyboard(oneshot::Sender<Recorded>),
}

const KEY_102ND: u32 = 86;

impl State {
    /// Types `text` into a window, giving it focus first; false when it is gone. The keys
    /// held now are released around the text, so they do not change it.
    pub fn type_into(&mut self, id: u64, text: &str) -> bool {
        if !self.focus_for_injection(id) {
            return false;
        }
        let syms: Vec<Keysym> = text
            .chars()
            .map(|c| match c {
                '\n' => Keysym::from(keysyms::KEY_Return),
                '\t' => Keysym::from(keysyms::KEY_Tab),
                c => xkb::utf32_to_keysym(c as u32),
            })
            .filter(|s| s.raw() != keysyms::KEY_NoSymbol)
            .collect();
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        keyboard.inject_text_keysyms(self, &syms);
        true
    }

    /// Sends a window a Mac chord as its own: Cmd+V is Ctrl+Shift+V in a terminal.
    pub fn send_chord_to(&mut self, id: u64, chord: Chord) -> bool {
        if !self.focus_for_injection(id) {
            return false;
        }
        let names = self.focus_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let strokes = self.isle.keyboards.tables().strokes_for(&names, &chord);
        self.send_strokes(&strokes);
        true
    }

    fn focus_for_injection(&mut self, id: u64) -> bool {
        let Some(window) = self.isle.window_by_id(id) else {
            return false;
        };
        if self.focused_window() != Some(id) {
            self.set_focus(Some(&window));
        }
        self.focused_window() == Some(id)
    }

    /// The recording grab takes a key before anything else: a keyboard to detect, or a chord
    /// whose modifiers the engine has seen pressed.
    pub fn record_keyboard(&mut self, name: &str, code: u32) -> bool {
        if !matches!(self.isle.recording, Some(Recording::Keyboard(_))) {
            return false;
        }
        let Some(Recording::Keyboard(reply)) = self.isle.recording.take() else {
            unreachable!()
        };
        info!(keyboard = name, "a keyboard detected");
        let _ = reply.send(Recorded {
            keyboard: name.to_owned(),
            iso: code == KEY_102ND,
            ..Recorded::default()
        });
        true
    }

    /// A chord recorded: Escape cancels, Delete or Backspace alone clears.
    pub fn finish_recording(&mut self, chord: Chord) {
        let Some(Recording::Chord(reply)) = self.isle.recording.take() else {
            return;
        };
        let raw = chord.sym.raw();
        let mut out = Recorded::default();
        if chord.mods == Mods::NONE && raw == keysyms::KEY_Escape {
            out.cancelled = true;
        } else if chord.mods == Mods::NONE
            && (raw == keysyms::KEY_BackSpace || raw == keysyms::KEY_Delete)
        {
        } else {
            out.chord = chord.words();
            out.holder = self
                .isle
                .bindings
                .holder(chord)
                .unwrap_or_default()
                .to_owned();
        }
        info!(
            chord = out.chord,
            holder = out.holder,
            cancelled = out.cancelled,
            "recorded"
        );
        let _ = reply.send(out);
    }

    /// Ends a recording without a chord: focus left, or the caller gave up.
    pub fn cancel_recording(&mut self) {
        let reply = match self.isle.recording.take() {
            Some(Recording::Chord(reply)) | Some(Recording::Keyboard(reply)) => reply,
            None => return,
        };
        let _ = reply.send(Recorded {
            cancelled: true,
            ..Recorded::default()
        });
    }
}
