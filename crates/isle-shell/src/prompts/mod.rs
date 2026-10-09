//! System prompts: one frame for every agent's question, over a scrim the compositor draws,
//! one at a time from a queue (`p1/structure/prompts.md`, `p1/surfaces/prompts.md`).

pub mod queue;
pub mod session;
mod surface;

use std::path::PathBuf;

pub use surface::PromptSurface;

use isle_ui::Secret;

/// Who is asking, by the badge on the asker's icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Polkit,
    Keyring,
    Network,
    Vpn,
    Bluetooth,
    /// A portal's permission for an app.
    Access,
}

impl Kind {
    pub fn badge(self) -> &'static str {
        match self {
            Kind::Polkit => "lock",
            Kind::Keyring => "key",
            Kind::Network => "wifi-high",
            Kind::Vpn => "isle-vpn",
            Kind::Bluetooth => "bluetooth",
            Kind::Access => "shield-check",
        }
    }
}

/// A field the requester asked for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldSpec {
    pub id: String,
    pub label: String,
    pub secret: bool,
    /// Offers "Show password".
    pub show: bool,
    /// Its text in the requester's monospace: a pairing PIN.
    pub mono: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChoiceSpec {
    pub id: String,
    pub label: String,
    pub checked: bool,
}

/// The status line: one message at a time, its slot reserved.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Status {
    #[default]
    None,
    Checking,
    /// Something to act on: wrong, locked out, a mismatch; with the warning glyph.
    Warning(String),
    /// What the requester says on the way: a reader's message.
    Info(String),
}

/// One prompt's content.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Prompt {
    pub id: u64,
    pub kind: Kind,
    /// The asking app's icon, else the kind's glyph alone.
    pub icon: Option<PathBuf>,
    /// A glyph in place of the icon: a Bluetooth device's kind.
    pub glyph: Option<String>,
    pub title: String,
    pub message: String,
    pub fields: Vec<FieldSpec>,
    pub choices: Vec<ChoiceSpec>,
    /// Administrators to choose from, when there are several.
    pub identities: Vec<String>,
    pub details: Option<String>,
    /// A code shown large: one to confirm, or one to type on the device in cells.
    pub code: Option<String>,
    /// The code's digits typed on the device so far; `None` shows it whole, not in cells.
    pub typed: Option<usize>,
    pub cancel: String,
    /// The action button; a prompt with none only cancels.
    pub action: Option<String>,
    /// The second field must match the first: a new password and its confirmation.
    pub confirm: bool,
    pub status: Status,
    /// Counts the wrong answers; a change clears the secrets and shakes the field.
    pub wrongs: u32,
}

/// What the surface shows now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub prompt: Option<Prompt>,
    /// Return, Space and clicks answer once armed.
    pub armed: bool,
    /// Requests waiting behind this one.
    pub more: usize,
    pub solid: bool,
    pub still: bool,
}

/// What the prompt asks of the compositor.
#[derive(Clone, Debug, PartialEq)]
pub enum PromptAction {
    Answer {
        id: u64,
        values: Vec<(String, Secret)>,
        choices: Vec<(String, bool)>,
        identity: usize,
    },
    Cancel(u64),
}

/// The copy of the queue line under the buttons.
pub fn more_line(more: usize) -> Option<String> {
    match more {
        0 => None,
        1 => Some("1 more request".into()),
        n => Some(format!("{n} more requests")),
    }
}
