//! OSDs: a level or a line on the island, raised when a level changes whoever changed it
//! (`p8-services.md` choice 4).

use std::time::{Duration, Instant};

use isle_shell::island::model::{Event, EventKind, Form};

use crate::state::Isle;

/// How long after Control Centre changes a level its echo raises no OSD.
const OWN_CHANGE: Duration = Duration::from_millis(1500);

impl Isle {
    /// A level OSD on `output`'s island, or the main screen's.
    pub fn osd_level(
        &mut self,
        symbol: &str,
        name: &str,
        level: f32,
        muted: bool,
        output: Option<String>,
    ) {
        self.post_osd(
            Form::Level {
                symbol: symbol.into(),
                name: name.into(),
                level,
                muted,
            },
            output,
        );
    }

    /// A line OSD: a microphone muted, a layout chosen.
    pub fn osd_line(&mut self, symbol: &str, text: &str) {
        self.post_osd(
            Form::Line {
                symbol: symbol.into(),
                fill: false,
                text: text.into(),
            },
            None,
        );
    }

    fn post_osd(&mut self, form: Form, output: Option<String>) {
        self.island.model.post(
            Event {
                kind: EventKind::Osd,
                form,
                output,
                app: None,
                urgent: false,
            },
            Instant::now(),
        );
        self.refresh_island();
    }

    /// Control Centre is about to change a level; its echo is not an OSD.
    pub fn expect_own_change(&mut self) {
        self.services.own_change = Some(Instant::now());
    }

    pub fn own_change(&self) -> bool {
        self.services
            .own_change
            .is_some_and(|at| at.elapsed() < OWN_CHANGE)
    }
}

/// The volume glyph for a level (`surfaces/island.md` Level bar).
pub fn volume_symbol(level: f32, muted: bool) -> &'static str {
    if muted {
        "speaker-slash"
    } else if level <= 0.0 {
        "speaker-none"
    } else if level < 0.5 {
        "speaker-low"
    } else {
        "speaker-high"
    }
}
