//! Store: apps from CachyOS, Flathub, the AUR and AppImages, their updates and what is
//! installed (`p14e-store.md`).

mod data;
mod installed;
mod pages;
mod plan;
#[cfg(test)]
mod tests;
mod window;

use std::{cell::Cell, rc::Rc};

use isle_app::{AppHandle, Delegate, Request, Spec, WindowKey, WindowSpec};

pub const APP: &str = "io.github.alexparlett.Isle.Store";
/// The link that opens Store on its Updates page.
pub const UPDATES: &str = "isle-store://updates";

const SPEC: Spec = Spec {
    id: APP,
    title: "Store",
    size: (1100.0, 760.0),
    min_size: (640.0, 440.0),
    max_size: None,
    traffic_light_inset: (0.0, 10.0),
    preferences: false,
    // The sidebar, `size-sidebar` wide: glass on the compositor's blur.
    backdrop: Some((0.0, 0.0, 220.0, 8192.0)),
    drops: &[],
};

#[derive(Default)]
struct Store {
    main: Rc<Cell<Option<WindowKey>>>,
}

impl Delegate for Store {
    fn request(&self, app: &AppHandle, request: Request) {
        if matches!(&request, Request::Action { name, .. } if name == isle_app::SERVICE) {
            return;
        }
        // Left in `Desktop::requests` for the window: `isle-store://updates` opens its Updates.
        if matches!(&request, Request::Open { uris, .. } if uris.iter().any(|u| u == UPDATES)) {
            let mut requests = app.desktop().requests;
            requests.write().push(request.clone());
        }
        let token = request.token().map(str::to_owned);
        match self.main.get().filter(|k| app.windows().contains(k)) {
            Some(key) => {
                if let Some(token) = token {
                    app.activate(key, &token);
                }
            }
            None => {
                let spec = WindowSpec::main(app.spec()).token(token);
                self.main.set(Some(app.open(spec, window::StoreWindow)));
            }
        }
    }
}

fn main() {
    isle_app::init_logging("isle_store");
    isle_app::run_with(SPEC, Store::default());
}
