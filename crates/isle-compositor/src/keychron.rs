//! Keychron receivers' input nodes as the keyboard or mouse behind them.

use std::path::Path;

use isle_keychron::{Peripheral, protocol::Kind};

use crate::state::{Isle, State};

impl Isle {
    /// The Keychron keyboard or mouse whose receiver or cable a sysfs path is under; where a
    /// receiver has both paired, the one of `kind`.
    pub fn keychron_at(&self, syspath: &Path, kind: Kind) -> Option<&Peripheral> {
        let mut under = self
            .services
            .keychron
            .state
            .iter()
            .filter(|p| p.usb.iter().any(|usb| syspath.starts_with(usb)));
        let first = under.next()?;
        if first.kind == kind {
            return Some(first);
        }
        Some(under.find(|p| p.kind == kind).unwrap_or(first))
    }

    /// As `keychron_at`, for an input node by its name under `/sys/class/input`.
    pub fn keychron_node(&self, id: &str, kind: Kind) -> Option<&Peripheral> {
        let path = std::fs::canonicalize(Path::new("/sys/class/input").join(id)).ok()?;
        self.keychron_at(&path, kind)
    }
}

impl State {
    /// Names, kinds and profiles of the input nodes behind Keychron receivers, after they change.
    pub fn keychron_devices(&mut self) {
        self.keychron_keyboards();
        self.reapply_pointer_settings();
        self.isle.notify(crate::worker::Event::KeyboardsChanged);
    }
}
