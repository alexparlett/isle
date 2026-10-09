//! The loop's side of the portal backend: what each `isle-portal` interface answers with.

use std::{collections::HashMap, sync::Arc};

use isle_portal::{BoxFuture, Closed, access::AccessRequest};
use isle_shell::prompts::{Kind, Prompt};
use tracing::warn;
use zvariant::OwnedValue;

use crate::{
    prompts::{Answer, Ask, PromptClient},
    worker::ToLoop,
};

/// The handlers, sharing the way to the loop and the prompt queue.
#[derive(Clone)]
pub struct Desktop {
    pub to_loop: ToLoop,
    pub prompts: PromptClient,
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
        isle_portal::screencast::CURSOR_HIDDEN | isle_portal::screencast::CURSOR_EMBEDDED
    }

    fn start(
        &self,
        session: String,
        app_id: String,
        _parent_window: String,
        selection: isle_portal::screencast::Selection,
        mut closed: Closed,
    ) -> BoxFuture<Option<isle_portal::screencast::Started>> {
        let to_loop = self.to_loop.clone();
        Box::pin(async move {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let (app, chosen) = (app_id.clone(), selection.clone());
            to_loop
                .call(move |state| state.share_begin(app, chosen, tx))
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
        _request: isle_portal::screenshot::ShotRequest,
        _closed: Closed,
    ) -> BoxFuture<Option<String>> {
        Box::pin(async { None })
    }

    fn pick_color(
        &self,
        _app_id: String,
        _parent_window: String,
        _closed: Closed,
    ) -> BoxFuture<Option<(f64, f64, f64)>> {
        Box::pin(async { None })
    }
}

impl isle_portal::settings::Settings for Desktop {
    fn read_all(&self) -> HashMap<String, HashMap<String, OwnedValue>> {
        HashMap::new()
    }
}

impl isle_portal::inhibit::Inhibit for Desktop {
    fn inhibit(&self, _handle: String, _app: String, _flags: u32, _reason: String) {}
    fn release(&self, _handle: String) {}
    fn monitor(&self, _session: String, _app: String) {}
    fn monitor_closed(&self, _session: String) {}
    fn query_end_response(&self, _session: String) {}
}

/// Owns the backend's name on a connection of its own, then has the frontend read it.
pub fn own(runtime: &tokio::runtime::Handle, to_loop: ToLoop) -> Option<zbus::Connection> {
    let desktop = Desktop {
        to_loop: to_loop.clone(),
        prompts: PromptClient(to_loop),
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
