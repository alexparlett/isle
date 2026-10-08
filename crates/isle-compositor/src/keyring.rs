//! The keyring prompter on the worker: each of gcr's prompts one of Isle's
//! (`p9-lock-and-prompts.md` choice 20, `structure/prompts.md` Keyring).

use isle_prompter::{Reply, Request};
use isle_shell::prompts::{ChoiceSpec, FieldSpec, Kind, Prompt, Status};
use tokio::sync::watch;
use tracing::warn;
use zeroize::Zeroizing;

use crate::prompts::{Answer, Ask, PromptClient};

/// Owns the prompter's name on the session bus.
pub async fn own(conn: &zbus::Connection, prompts: PromptClient) -> zbus::Result<bool> {
    let dbus = zbus::fdo::DBusProxy::new(conn).await?;
    let pids = std::sync::Arc::new(dbus);
    isle_prompter::serve(conn, move |request, dismissed| {
        let (prompts, pids) = (prompts.clone(), pids.clone());
        async move {
            let pid = match zbus::names::BusName::try_from(request.caller.as_str()) {
                Ok(name) => pids.get_connection_unix_process_id(name).await.ok(),
                Err(_) => None,
            };
            ask(prompts, request, pid.map(|p| p as i32), dismissed).await
        }
    })
    .await
}

async fn ask(
    prompts: PromptClient,
    request: Request,
    pid: Option<i32>,
    mut dismissed: watch::Receiver<bool>,
) -> Reply {
    let mut fields = Vec::new();
    if !request.confirm {
        fields.push(FieldSpec {
            id: "password".into(),
            label: "Password".into(),
            secret: true,
            ..Default::default()
        });
        if request.password_new {
            fields.push(FieldSpec {
                id: "confirm".into(),
                label: "Confirm".into(),
                secret: true,
                ..Default::default()
            });
        }
    }
    let choices = request
        .choice_label
        .iter()
        .map(|label| ChoiceSpec {
            id: "choice".into(),
            label: label.clone(),
            checked: request.choice_chosen,
        })
        .collect();
    // gcr's message reads as the title and its description as the message, as GNOME
    // Shell draws them.
    let (title, message) = if request.message.is_empty() {
        (request.title.clone(), request.description.clone())
    } else {
        (request.message.clone(), request.description.clone())
    };
    let action = if request.continue_label.is_empty() {
        if request.confirm {
            "Continue"
        } else {
            "Unlock"
        }
        .to_owned()
    } else {
        request.continue_label.replace('_', "")
    };
    let cancel = if request.cancel_label.is_empty() {
        "Cancel".to_owned()
    } else {
        request.cancel_label.replace('_', "")
    };
    let ask = Ask {
        prompt: Prompt {
            kind: Kind::Keyring,
            title,
            message,
            fields,
            choices,
            cancel,
            action: Some(action),
            confirm: request.password_new,
            status: if request.warning.is_empty() {
                Status::None
            } else {
                Status::Warning(request.warning.clone())
            },
            wrongs: u32::from(!request.warning.is_empty()),
            ..Default::default()
        },
        title_unknown: None,
        pid,
        user_started: false,
        checks: false,
        subject: None,
    };
    let Ok((id, mut answers)) = prompts.ask(ask).await else {
        return Reply::No;
    };
    let answer = tokio::select! {
        answer = answers.recv() => answer,
        _ = dismissed.wait_for(|d| *d) => {
            prompts.done(id);
            return Reply::No;
        }
    };
    match answer {
        Some(answer @ Answer::Values { .. }) => {
            let choice = answer.choice("choice").unwrap_or(false);
            if request.confirm {
                return Reply::Yes { choice };
            }
            match answer.value("password") {
                Some(secret) => Reply::Secret {
                    secret: Zeroizing::new(secret.expose().to_owned()),
                    choice,
                },
                None => {
                    warn!("the keyring's prompt came back with no password");
                    Reply::No
                }
            }
        }
        _ => Reply::No,
    }
}
