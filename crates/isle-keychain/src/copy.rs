//! Copies: a secret marked so clipboard history skips it and withdrawn after the period set
//! unless replaced, the process held until then (choice 18); anything else copied plainly.

use std::{cell::RefCell, rc::Rc, time::Duration};

use futures_util::future::{Either, select};
use isle_app::{AppHandle, Hold};
use zeroize::Zeroizing;

const TEXT: [&str; 3] = ["text/plain;charset=utf-8", "UTF8_STRING", "text/plain"];

/// The copy waiting to be withdrawn, held by the app above its windows.
#[derive(Clone, Default)]
pub struct Copies(Rc<RefCell<(u64, Option<Hold>)>>);

impl Copies {
    /// Whether a copied secret is still waiting to be withdrawn.
    pub fn waiting(&self) -> bool {
        self.0.borrow().1.is_some()
    }
}

/// A sleep on the event loop, from a timer thread.
pub async fn sleep(duration: Duration) {
    let (tx, rx) = futures_channel::oneshot::channel::<()>();
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        let _ = tx.send(());
    });
    let _ = rx.await;
}

fn island(app: &AppHandle, kind: &'static str) {
    let Some(bus) = app.bus() else { return };
    app.worker().spawn(async move {
        if let Ok(shell) = isle_ipc::shell::ShellProxy::new(&bus).await {
            let _ = shell.copied(kind).await;
        }
    });
}

/// Copies a secret: marked, withdrawn after `clear_after` unless replaced, the app held
/// until it ends.
pub fn secret(app: &AppHandle, copies: &Copies, text: Zeroizing<String>, clear_after: Duration) {
    let Some(data) = app.data() else { return };
    let mut contents: Vec<(String, Vec<u8>)> = TEXT
        .iter()
        .map(|t| ((*t).to_owned(), text.as_bytes().to_vec()))
        .collect();
    contents.push(("x-kde-passwordManagerHint".into(), b"secret".to_vec()));
    let ended = data.set_secret_selection(contents);
    let generation = {
        let mut slot = copies.0.borrow_mut();
        slot.0 += 1;
        slot.1 = Some(app.hold());
        slot.0
    };
    let slot = copies.0.clone();
    app.spawn(async move {
        let mut ended = ended;
        let first = select(&mut ended, Box::pin(sleep(clear_after))).await;
        if let Either::Right(_) = first {
            data.withdraw_secret();
            let _ = ended.await;
        }
        // A later copy holds the app for itself; only this copy's hold is let go.
        let mut slot = slot.borrow_mut();
        if slot.0 == generation {
            slot.1.take();
        }
    });
    island(app, "password");
}

/// Copies text that is not a secret: a user name, a public key.
pub fn plain(app: &AppHandle, text: &str) {
    let Some(data) = app.data() else { return };
    data.set_selection(
        TEXT.iter()
            .map(|t| ((*t).to_owned(), text.as_bytes().to_vec()))
            .collect(),
    );
    island(app, "");
}
