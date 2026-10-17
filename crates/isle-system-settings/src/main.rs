//! System Settings (`docs/design/apps.md` Settings; `p1/structure/settings.md`).

mod account;
mod frame;
mod nav;
mod panes;
#[cfg(test)]
mod tests;

use freya::prelude::*;
use isle_app::Spec;

const SPEC: Spec = Spec {
    id: "io.github.alexparlett.Isle.Settings",
    title: "System Settings",
    size: (900.0, 760.0),
    min_size: (784.0, 564.0),
    // The lights' 32 point group centred in the 52 point toolbar.
    traffic_light_inset: (0.0, 10.0),
    preferences: false,
};

struct Settings;

impl App for Settings {
    fn render(&self) -> impl IntoElement {
        frame::Frame
    }
}

/// A pane's groups of rows, under its header.
pub fn pane_body(_pane: &str, _at: &nav::Location) -> Element {
    rect().into_element()
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--write-index") {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/settings/index.kdl");
        std::fs::write(path, panes::index()).expect("data/settings/index.kdl");
        return;
    }
    isle_app::init_logging(module_path!());
    isle_app::run(SPEC, Settings);
}
