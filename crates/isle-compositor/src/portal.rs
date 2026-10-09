//! The loop's side of the portal backend: what each `isle-portal` interface answers with.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use isle_portal::{BoxFuture, Closed, access::AccessRequest};
use isle_shell::prompts::{Kind, Prompt};
use tracing::warn;
use zvariant::OwnedValue;

use crate::{
    prompts::{Answer, Ask, PromptClient},
    worker::ToLoop,
};

/// The appearance namespace as the Settings portal reads it, kept by the loop.
pub type Appearance = Arc<Mutex<HashMap<String, OwnedValue>>>;

/// The handlers, sharing the way to the loop and the prompt queue.
#[derive(Clone)]
pub struct Desktop {
    pub to_loop: ToLoop,
    pub prompts: PromptClient,
    pub appearance: Appearance,
}

impl Desktop {
    pub fn handlers(self) -> isle_portal::Handlers {
        let this = Arc::new(self);
        isle_portal::Handlers {
            access: this.clone(),
            screencast: this.clone(),
            screenshot: this.clone(),
            settings: this.clone(),
            inhibit: this,
        }
    }
}

impl isle_portal::access::Access for Desktop {
    fn dialog(&self, request: AccessRequest, mut closed: Closed) -> BoxFuture<Option<bool>> {
        let (to_loop, prompts) = (self.to_loop.clone(), self.prompts.clone());
        Box::pin(async move {
            let app = request.app_id.trim_end_matches(".desktop").to_owned();
            let icon = to_loop
                .call(move |state| state.isle.app_icon(&app, 64))
                .await
                .ok()
                .flatten();
            let message = [request.subtitle.as_str(), request.body.as_str()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            let prompt = Prompt {
                kind: Kind::Access,
                icon,
                title: request.title,
                message,
                cancel: request.deny_label.unwrap_or_else(|| "Don't Allow".into()),
                action: Some(request.grant_label.unwrap_or_else(|| "Allow".into())),
                ..Default::default()
            };
            let (id, mut answers) = prompts
                .ask(Ask {
                    prompt,
                    title_unknown: None,
                    pid: None,
                    user_started: false,
                    checks: false,
                    subject: None,
                })
                .await
                .ok()?;
            let answer = tokio::select! {
                answer = answers.recv() => answer,
                _ = closed.wait() => {
                    prompts.done(id);
                    return None;
                }
            };
            Some(matches!(answer, Some(Answer::Values { .. })))
        })
    }
}

impl isle_portal::screencast::ScreenCast for Desktop {
    fn source_types(&self) -> u32 {
        isle_portal::screencast::MONITOR | isle_portal::screencast::WINDOW
    }

    fn cursor_modes(&self) -> u32 {
        isle_portal::screencast::CURSOR_HIDDEN
            | isle_portal::screencast::CURSOR_EMBEDDED
            | isle_portal::screencast::CURSOR_METADATA
    }

    fn start(
        &self,
        session: String,
        app_id: String,
        parent_window: String,
        selection: isle_portal::screencast::Selection,
        mut closed: Closed,
    ) -> BoxFuture<Option<isle_portal::screencast::Started>> {
        let to_loop = self.to_loop.clone();
        Box::pin(async move {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let (app, chosen) = (app_id.clone(), selection.clone());
            to_loop
                .call(move |state| state.share_begin(app, &parent_window, chosen, tx))
                .await
                .ok()?;
            let picked = tokio::select! {
                picked = rx => picked.ok().flatten(),
                _ = closed.wait() => {
                    to_loop.post(|state| state.share_cancel());
                    None
                }
            }?;
            let remember = picked.remember;
            let (s, a, sel) = (session.clone(), app_id.clone(), selection.clone());
            let ids = to_loop
                .call(move |state| state.share_start(&s, &a, &sel, &picked))
                .await
                .ok()??;
            // The nodes and their serials come from PipeWire a moment after the streams.
            for _ in 0..100 {
                let (ids, sel) = (ids.clone(), selection.clone());
                let ready = to_loop
                    .call(move |state| state.share_ready(&ids, &sel, remember))
                    .await
                    .ok()?;
                if let Some(started) = ready {
                    return Some(started);
                }
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            }
            to_loop.post(move |state| state.share_closed(&session));
            None
        })
    }

    fn close(&self, session: String) {
        self.to_loop.post(move |state| state.share_closed(&session));
    }
}

impl isle_portal::screenshot::Screenshot for Desktop {
    fn screenshot(
        &self,
        request: isle_portal::screenshot::ShotRequest,
        mut closed: Closed,
    ) -> BoxFuture<Option<String>> {
        let to_loop = self.to_loop.clone();
        Box::pin(async move {
            let (tx, rx) = tokio::sync::oneshot::channel();
            to_loop
                .call(move |state| {
                    if request.interactive {
                        state.open_toolbar_for(&request.app_id, tx);
                    } else {
                        state.shoot_for_portal(request.target, tx);
                    }
                })
                .await
                .ok()?;
            let path = tokio::select! {
                path = rx => path.ok().flatten(),
                _ = closed.wait() => {
                    to_loop.post(|state| state.close_toolbar());
                    None
                }
            }?;
            Some(format!("file://{}", path.display()))
        })
    }

    fn pick_color(
        &self,
        _app_id: String,
        _parent_window: String,
        mut closed: Closed,
    ) -> BoxFuture<Option<(f64, f64, f64)>> {
        let to_loop = self.to_loop.clone();
        Box::pin(async move {
            let (tx, rx) = tokio::sync::oneshot::channel();
            to_loop
                .call(move |state| {
                    state.when_unlocked(move |state| {
                        if tx.is_closed() {
                            return;
                        }
                        if state.picking() {
                            let _ = tx.send(None);
                            return;
                        }
                        state.isle.pick_reply = Some(tx);
                        state.open_pick(crate::freeze::Purpose::ColourFor);
                    })
                })
                .await
                .ok()?;
            let rgb = tokio::select! {
                rgb = rx => rgb.ok().flatten(),
                _ = closed.wait() => {
                    to_loop.post(|state| state.close_pick());
                    None
                }
            }?;
            let f = |c: u8| c as f64 / 255.0;
            Some((f(rgb[0]), f(rgb[1]), f(rgb[2])))
        })
    }
}

impl isle_portal::settings::Settings for Desktop {
    fn read_all(&self) -> HashMap<String, HashMap<String, OwnedValue>> {
        let keys = self
            .appearance
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), v.try_clone().ok()?)))
            .collect();
        HashMap::from([(isle_portal::settings::APPEARANCE.to_owned(), keys)])
    }
}

impl isle_portal::inhibit::Inhibit for Desktop {
    fn inhibit(&self, handle: String, app: String, flags: u32, reason: String) {
        self.to_loop
            .post(move |state| state.portal_inhibit(handle, app, flags, reason));
    }

    fn release(&self, handle: String) {
        self.to_loop
            .post(move |state| state.portal_release(&handle));
    }

    fn monitor(&self, session: String, _app: String) {
        self.to_loop.post(move |state| {
            state.isle.portal_monitors.insert(session.clone());
            state.portal_monitor_state(Some(session));
        });
    }

    fn monitor_closed(&self, session: String) {
        self.to_loop.post(move |state| {
            state.isle.portal_monitors.remove(&session);
        });
    }

    fn query_end_response(&self, _session: String) {}
}

/// The appearance namespace from the theme in force.
pub fn appearance_of(theme: &isle_tokens::Theme) -> HashMap<String, OwnedValue> {
    use zvariant::Value;
    let a = &theme.appearance;
    let c = theme.tokens.colour_accent;
    let f = |v: u8| v as f64 / 255.0;
    let mut keys = HashMap::new();
    let mut put = |k: &str, v: Value<'_>| {
        if let Ok(v) = OwnedValue::try_from(v) {
            keys.insert(k.to_owned(), v);
        }
    };
    put(
        "color-scheme",
        Value::from(match a.mode {
            isle_tokens::Mode::Dark => 1u32,
            isle_tokens::Mode::Light => 2u32,
        }),
    );
    put("accent-color", Value::from((f(c.r), f(c.g), f(c.b))));
    put("contrast", Value::from(u32::from(a.increase_contrast)));
    put("reduced-motion", Value::from(u32::from(a.reduce_motion)));
    keys
}

impl crate::state::State {
    /// The theme changed: the Settings portal's keys follow, each change told to the
    /// frontend.
    pub fn portal_appearance_changed(&mut self) {
        let now = appearance_of(self.isle.shell.theme());
        let changed: Vec<(String, OwnedValue)> = {
            let mut kept = self.isle.portal_appearance.lock().unwrap();
            let changed = now
                .iter()
                .filter(|(k, v)| kept.get(*k).map(|old| old != *v).unwrap_or(true))
                .filter_map(|(k, v)| Some((k.clone(), v.try_clone().ok()?)))
                .collect();
            *kept = now;
            changed
        };
        if changed.is_empty() {
            return;
        }
        if let (Some(conn), Some(worker)) = (self.isle.portal.clone(), self.isle.worker.clone()) {
            worker.spawn(async move {
                for (key, value) in changed {
                    isle_portal::settings::changed(
                        &conn,
                        isle_portal::settings::APPEARANCE,
                        &key,
                        zvariant::Value::from(value),
                    )
                    .await;
                }
            });
        }
    }

    /// An app's Inhibit: idle and suspend hold idle; logout and user switch hold the
    /// session's end.
    pub fn portal_inhibit(&mut self, handle: String, app: String, flags: u32, reason: String) {
        let app = app.trim_end_matches(".desktop").to_owned();
        let name = if app.is_empty() {
            "An app".to_owned()
        } else {
            self.isle.app_name(&app)
        };
        self.isle.portal_holds.insert(handle, (name, flags, reason));
        self.idle_holds_changed();
    }

    pub fn portal_release(&mut self, handle: &str) {
        if self.isle.portal_holds.remove(handle).is_some() {
            self.idle_holds_changed();
        }
    }

    /// Tells monitors the session's state: `session` alone, or every one.
    pub fn portal_monitor_state(&mut self, session: Option<String>) {
        let state = match &self.isle.session.ending {
            Some(e) if e.finishing => isle_portal::inhibit::ENDING,
            Some(_) => isle_portal::inhibit::QUERY_END,
            None => isle_portal::inhibit::RUNNING,
        };
        let locked = self.isle.lock.locked;
        let sessions: Vec<String> = match session {
            Some(s) => vec![s],
            None => self.isle.portal_monitors.iter().cloned().collect(),
        };
        if sessions.is_empty() {
            return;
        }
        if let (Some(conn), Some(worker)) = (self.isle.portal.clone(), self.isle.worker.clone()) {
            worker.spawn(async move {
                for s in sessions {
                    isle_portal::inhibit::state_changed(&conn, &s, locked, state).await;
                }
            });
        }
    }
}

/// Owns the backend's name on a connection of its own, then has the frontend read it.
pub fn own(
    runtime: &tokio::runtime::Handle,
    to_loop: ToLoop,
    appearance: Appearance,
) -> Option<zbus::Connection> {
    let desktop = Desktop {
        to_loop: to_loop.clone(),
        prompts: PromptClient(to_loop),
        appearance,
    };
    let owned = runtime.block_on(async {
        let conn = zbus::Connection::session().await?;
        let owned = isle_portal::serve(&conn, desktop.handlers()).await?;
        if owned {
            isle_portal::restart_frontend(&conn).await;
        }
        zbus::Result::Ok((conn, owned))
    });
    match owned {
        Ok((conn, true)) => Some(conn),
        Ok((_, false)) => {
            warn!("another program is the Isle portal backend");
            None
        }
        Err(err) => {
            warn!("no portal backend: {err}");
            None
        }
    }
}
