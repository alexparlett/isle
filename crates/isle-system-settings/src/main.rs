//! System Settings (`docs/design/apps.md` Settings; `p1/structure/settings.md`).

mod account;
mod frame;
mod nav;
mod pane;
#[cfg(test)]
mod pane_tests;
mod panes;
mod services;
#[cfg(test)]
mod tests;
mod ui;

use freya::prelude::*;
use isle_app::Spec;

const SPEC: Spec = Spec {
    id: "io.github.alexparlett.Isle.Settings",
    title: "System Settings",
    size: (900.0, 760.0),
    min_size: (784.0, 564.0),
    // As macOS's: taller as the screen allows, never wider than it opens.
    max_size: Some((900.0, 16384.0)),
    // The lights' 32 point group centred in the 52 point toolbar.
    traffic_light_inset: (0.0, 10.0),
    preferences: false,
    // The sidebar's width and more than any window's height; the compositor cuts it to the window.
    backdrop: Some((0.0, 0.0, 220.0, 8192.0)),
};

#[derive(Clone)]
struct Settings;

impl App for Settings {
    fn render(&self) -> impl IntoElement {
        let worker = isle_app::worker();
        let bus = isle_app::session_bus();
        let services = use_provide_context({
            let (worker, bus) = (worker.clone(), bus.clone());
            move || services::Services::live(worker, bus)
        });
        use_side_effect(move || {
            if services.acts.read().is_empty() {
                return;
            }
            let mut queue = services.acts;
            let acts = std::mem::take(&mut *queue.write());
            for act in acts {
                let job = worker.spawn(services::run(bus.clone(), act.clone()));
                let mut change = services.display_change;
                spawn(async move {
                    match job.await {
                        Ok(Ok(Some(id))) => change.set(Some(id)),
                        Ok(Ok(None)) => {}
                        Ok(Err(why)) => tracing::warn!(?act, "refused: {why}"),
                        Err(err) => tracing::warn!(?act, "failed: {err}"),
                    }
                });
            }
        });
        frame::Frame
    }
}

fn main() {
    // A maintainer's command, writing into the checkout it was built from; release builds
    // are installed where there is none.
    #[cfg(debug_assertions)]
    if std::env::args().nth(1).as_deref() == Some("--write-index") {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/settings/index.kdl");
        std::fs::write(path, panes::index()).expect("data/settings/index.kdl");
        return;
    }
    isle_app::init_logging(module_path!());
    isle_app::run(SPEC, Settings);
}
