//! Energy: the power profile.

use freya::prelude::*;
use isle_ui::form;

use crate::{
    nav::Location,
    services::{Act, Follow, use_follow},
    ui::{self, Rows},
};

const PROFILES: [(&str, &str); 3] = [
    ("power-saver", "Low Power"),
    ("balanced", "Automatic"),
    ("performance", "High Power"),
];

#[derive(PartialEq)]
pub struct Energy {
    pub at: Location,
}

impl Component for Energy {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Profiles]);
        let r = Rows::new(&self.at);
        let snapshot = services.profiles.read().clone();
        if !snapshot.available {
            return ui::missing(
                "Energy modes aren't available: power-profiles-daemon isn't running.",
            );
        }
        let profiles = snapshot.state;
        let selected = PROFILES
            .iter()
            .position(|(p, _)| *p == profiles.active)
            .unwrap_or(1);
        let held = profiles.holds.iter().any(|h| h.profile == "performance");
        let mut row = r
            .row("energy-mode", "Energy Mode", None)
            .control(form::Segmented::new(
                PROFILES.iter().map(|(_, l)| (*l).to_owned()).collect(),
                selected,
                "Energy Mode",
                move |i: usize| services.act(Act::Profile(PROFILES[i].0.into())),
            ));
        if held {
            row = row.description("High Power while Game Mode is on.");
        }
        form::group(None, vec![row.into_element()]).into_element()
    }
}
