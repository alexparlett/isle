//! Keychain: saved logins, Wi-Fi passwords, app secrets and SSH keys under one search
//! (`docs/design/apps.md` Keychain; `p1/structure/keychain.md`).

mod copy;
mod detail;
mod edit;
mod generator_view;
mod list;
mod model;
mod settings;
mod sheets;
mod sidebar;
mod ssh_sheets;
mod state;
mod store;
#[cfg(test)]
mod tests;
mod window;

use std::{cell::Cell, rc::Rc};

use isle_app::{AppHandle, Delegate, Request, Spec, WindowKey, WindowSpec};

const SPEC: Spec = Spec {
    id: settings::APP,
    title: "Keychain",
    size: (1000.0, 640.0),
    min_size: (560.0, 400.0),
    max_size: None,
    traffic_light_inset: (0.0, 10.0),
    preferences: true,
    backdrop: None,
};

/// What the app keeps above its windows: the copy waiting to be withdrawn, and the windows.
#[derive(Clone, Default)]
pub struct Shared {
    pub copies: copy::Copies,
    main: Rc<Cell<Option<WindowKey>>>,
    settings: Rc<Cell<Option<WindowKey>>>,
}

impl PartialEq for Shared {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.main, &other.main)
    }
}

impl Shared {
    pub fn open_settings(&self, app: &AppHandle) {
        if self
            .settings
            .get()
            .is_some_and(|k| app.windows().contains(&k))
        {
            return;
        }
        let spec = WindowSpec::new("Keychain Settings", (480., 200.), (420., 160.));
        self.settings
            .set(Some(app.open(spec, settings::SettingsWindow)));
    }
}

struct Keychain(Shared);

impl Delegate for Keychain {
    fn request(&self, app: &AppHandle, request: Request) {
        if matches!(&request, Request::Action { name, .. } if name == isle_app::SERVICE) {
            return;
        }
        let token = request.token().map(str::to_owned);
        match self.0.main.get().filter(|k| app.windows().contains(k)) {
            Some(key) => {
                if let Some(token) = token {
                    app.activate(key, &token);
                }
            }
            None => {
                let spec = WindowSpec::main(app.spec()).token(token);
                let key = app.open(
                    spec,
                    window::KeychainWindow {
                        shared: self.0.clone(),
                    },
                );
                self.0.main.set(Some(key));
            }
        }
    }

    /// A copy waiting to be withdrawn keeps the process after its windows close; it ends at
    /// the withdrawal (choice 18).
    fn quit(&self, app: &AppHandle) -> bool {
        if !self.0.copies.waiting() {
            return true;
        }
        for key in app.windows() {
            app.close(key);
        }
        false
    }
}

fn main() {
    isle_app::init_logging("isle_keychain");
    isle_app::run_with(SPEC, Keychain(Shared::default()));
}
