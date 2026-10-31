//! Cleaner: where the space went and what can go (`docs/design/apps.md` Cleaner).

mod basket;
mod cleanup;
mod model;
mod overview;
mod scanner;
mod state;
#[cfg(test)]
mod tests;
mod ui;
mod window;

use isle_app::Spec;

const SPEC: Spec = Spec {
    id: "io.github.alexparlett.Isle.Cleaner",
    title: "Cleaner",
    size: (1200.0, 760.0),
    min_size: (720.0, 480.0),
    max_size: None,
    traffic_light_inset: (0.0, 10.0),
    preferences: false,
    backdrop: None,
};

fn main() {
    isle_app::init_logging("isle_cleaner");
    let folder = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_dir());
    isle_app::run(SPEC, window::Cleaner { folder });
}
