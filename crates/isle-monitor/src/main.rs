//! Monitor: Activity Monitor for Isle (`docs/design/apps.md` Monitor).

mod act;
mod actions;
mod footer;
mod history;
mod model;
mod sampler;
mod state;
#[cfg(test)]
mod tests;
mod window;

use isle_app::Spec;

const SPEC: Spec = Spec {
    id: "io.github.alexparlett.Isle.Monitor",
    title: "Monitor",
    size: (1040.0, 680.0),
    min_size: (640.0, 420.0),
    max_size: None,
    traffic_light_inset: (0.0, 10.0),
    preferences: false,
    backdrop: None,
};

fn main() {
    isle_app::init_logging("isle_monitor");
    isle_app::run(SPEC, window::Monitor);
}
