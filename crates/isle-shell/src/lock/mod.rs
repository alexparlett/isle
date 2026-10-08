//! The lock's face and its minimal form (`p1/surfaces/lock.md`). The compositor holds the lock
//! and the typed password; these only draw what it hands them.

mod minimal;
mod surface;

use std::path::PathBuf;

pub use minimal::MinimalSurface;
pub use surface::LockSurface;

/// What the status line says, one message at a time in `structure/lock.md`'s order.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Line {
    #[default]
    None,
    CapsLock,
    /// `long` once the check has run past `timing-check-long-ms`.
    Checking {
        long: bool,
    },
    /// The count of wrong passwords, and the layout to check when several are set.
    Wrong {
        count: u32,
        layout: Option<String>,
    },
    LockedOut {
        minutes: u32,
    },
    Error,
    /// The session's end was stopped, or refused.
    Stopped(String),
}

impl Line {
    /// The message and its glyph; checking is told by the spinner instead.
    pub fn text(&self) -> Option<(Option<&'static str>, String)> {
        let warning = Some("warning-circle");
        Some(match self {
            Line::None => return None,
            Line::CapsLock => (Some("arrow-fat-line-up"), "Caps Lock is on".into()),
            Line::Checking { long: false } => (None, "Checking".into()),
            Line::Checking { long: true } => (None, "Still checking".into()),
            Line::Wrong {
                layout: Some(name), ..
            } => (
                warning,
                format!("Wrong password. Check the keyboard layout ({name})."),
            ),
            Line::Wrong { count, .. } if *count >= 2 => (
                warning,
                format!("Wrong password, {count} attempts. Try again."),
            ),
            Line::Wrong { .. } => (warning, "Wrong password. Try again.".into()),
            Line::LockedOut { minutes } => (
                warning,
                format!("Too many attempts. Try again in {minutes} min."),
            ),
            Line::Error => (warning, "Couldn't check the password. Try again.".into()),
            Line::Stopped(words) => (warning, words.clone()),
        })
    }

    /// Whether it asks something of the user, and so is drawn in the stronger colour.
    pub fn acts(&self) -> bool {
        !matches!(self, Line::None | Line::Checking { .. })
    }
}

/// The player Now Playing shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Playing {
    pub title: String,
    pub artist: String,
    pub art: Option<PathBuf>,
    pub playing: bool,
}

/// One app's notifications on the lock.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Note {
    pub app: String,
    pub name: String,
    pub icon: Option<PathBuf>,
    pub count: u32,
    /// The newest summary, when the setting shows previews.
    pub summary: Option<String>,
}

/// The face's content, the same on every screen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Face {
    pub date: String,
    pub time: String,
    /// The Focus in force's glyph, before the date.
    pub focus: Option<String>,
    pub playing: Option<Playing>,
    pub notes: Vec<Note>,
    pub more: u32,
    pub user: String,
    pub picture: Option<PathBuf>,
    pub dots: usize,
    pub line: Line,
    /// The field takes no keys: checking, or locked out.
    pub held: bool,
    pub locked_out: bool,
    /// The field's error edge, until the next key.
    pub error_edge: bool,
    /// Counts up with each wrong password; a change shakes the field.
    pub shakes: u32,
    pub awake: bool,
    /// The layout's short name, when more than one is set, and whether a wrong password
    /// marks it.
    pub layout: Option<String>,
    pub layout_marked: bool,
    /// The battery's percentage and whether it charges, on a machine that has one.
    pub battery: Option<(u8, bool)>,
    pub power: Power,
    pub solid: bool,
    pub still: bool,
}

/// The power row: its circles, Restart or Shut Down asked once in place, then the apps
/// quitting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Power {
    #[default]
    Row,
    Confirm {
        restart: bool,
    },
    Quitting,
}

/// One screen's frame: the face's content, and whether this screen has the face.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub face: Face,
    pub here: bool,
    pub width: f32,
    pub height: f32,
    /// Counts the Test interface's injected panics; a change makes the tree panic once.
    pub panics: u32,
}

/// What the face asks of the compositor.
#[derive(Clone, Debug, PartialEq)]
pub enum LockAction {
    /// The field took keyboard focus, or gave it up to another control.
    FieldFocus(bool),
    /// Any pointer movement or press on the face.
    Wake,
    Sleep,
    Restart,
    ShutDown,
    /// The confirm in place: go on, or back to the circles.
    PowerConfirm,
    PowerCancel,
    PlayPause,
    Next,
    Previous,
    NextLayout,
}
