//! Lock Screen: when the screen dims, locks and turns off.

use freya::prelude::*;
use isle_ui::form;

use crate::{nav::Location, ui::Rows};

/// The steps each delay offers, in minutes; 0 is never.
const DELAYS: &[(&str, &str)] = &[
    ("1", "1 min"),
    ("2", "2 min"),
    ("5", "5 min"),
    ("10", "10 min"),
    ("15", "15 min"),
    ("30", "30 min"),
    ("60", "1 hr"),
    ("120", "2 hr"),
    ("240", "4 hr"),
    ("0", "Never"),
];

#[derive(PartialEq)]
pub struct LockScreen {
    pub at: Location,
}

impl Component for LockScreen {
    fn render(&self) -> impl IntoElement {
        let r = Rows::new(&self.at);
        form::group(
            None,
            vec![
                r.choice_int(
                    "dim-after",
                    "Dim the display after",
                    "lock-screen.dim-after",
                    DELAYS,
                )
                .into_element(),
                r.choice_int("lock-after", "Lock after", "lock-screen.lock-after", DELAYS)
                    .into_element(),
                r.choice_int(
                    "display-off-after",
                    "Turn display off after",
                    "lock-screen.display-off-after",
                    DELAYS,
                )
                .into_element(),
                r.switch(
                    "lock-on-sleep",
                    "Lock when the computer sleeps",
                    "lock-screen.lock-on-sleep",
                )
                .into_element(),
            ],
        )
    }
}
