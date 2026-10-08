//! The polkit agent on the worker: each request a prompt, each attempt one conversation with
//! polkit's helper (`p9-lock-and-prompts.md` choice 20).

use std::{error::Error, time::Duration};

use isle_bus::NameWatch;
use isle_polkit::{Conversation, Line, Outcome, Request};
use isle_shell::prompts::{FieldSpec, Kind, Prompt, Status};
use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::prompts::{Answer, Ask, PromptClient};

const NAME: &str = "org.freedesktop.PolicyKit1";

/// The helper's socket; tests name their own.
fn helper_socket() -> String {
    #[cfg(feature = "test-hooks")]
    if let Ok(path) = std::env::var("ISLE_POLKIT_SOCKET") {
        return path;
    }
    isle_polkit::HELPER_SOCKET.to_owned()
}

/// The session polkitd finds for Isle's processes: `XDG_SESSION_ID`, else logind's session
/// of this process.
async fn session_id(bus: &zbus::Connection) -> Result<String, Box<dyn Error + Send + Sync>> {
    if let Ok(id) = std::env::var("XDG_SESSION_ID")
        && !id.is_empty()
    {
        return Ok(id);
    }
    let manager = zbus::Proxy::new(
        bus,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await?;
    let path: zbus::zvariant::OwnedObjectPath = manager
        .call("GetSessionByPID", &(std::process::id()))
        .await?;
    let session = zbus::Proxy::new(
        bus,
        "org.freedesktop.login1",
        path,
        "org.freedesktop.login1.Session",
    )
    .await?;
    Ok(session.get_property::<String>("Id").await?)
}

/// Registers the agent whenever polkitd is on the bus, again after it restarts.
pub async fn run(prompts: PromptClient) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bus = isle_bus::system().await?;
    let mut name = NameWatch::new(&bus, NAME).await?;
    loop {
        name.appeared().await?;
        // A connection of its own, so a registration starts afresh with polkitd.
        let conn = zbus::connection::Builder::system()?.build().await?;
        let session = session_id(&conn).await?;
        let client = prompts.clone();
        isle_polkit::serve(&conn, &session, move |request, cancelled| {
            authenticate(client.clone(), request, cancelled)
        })
        .await?;
        name.vanished().await?;
        debug!("polkitd left the bus");
        drop(conn);
    }
}

/// The answering users: this one first, then root, then the rest, with their names.
fn users(identities: &[u32]) -> Vec<(String, String)> {
    let me = unsafe { libc::getuid() };
    let mut uids = identities.to_vec();
    uids.sort_by_key(|uid| (*uid != me, *uid != 0, *uid));
    uids.into_iter()
        .filter_map(|uid| unsafe {
            let entry = libc::getpwuid(uid);
            if entry.is_null() {
                return None;
            }
            let login = std::ffi::CStr::from_ptr((*entry).pw_name)
                .to_string_lossy()
                .into_owned();
            let full = if (*entry).pw_gecos.is_null() {
                String::new()
            } else {
                std::ffi::CStr::from_ptr((*entry).pw_gecos)
                    .to_string_lossy()
                    .split(',')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            };
            let name = if full.is_empty() { login.clone() } else { full };
            Some((login, name))
        })
        .collect()
}

/// What Details shows: the action, and for `pkexec` the command.
fn details(request: &Request) -> String {
    let mut lines = vec![format!("Action: {}", request.action_id)];
    if let Some(program) = request.details.get("program") {
        lines.push(format!("Program: {program}"));
    }
    if let Some(command) = request.details.get("command_line") {
        lines.push(format!("Command: {command}"));
    }
    lines.join("\n")
}

async fn authenticate(
    prompts: PromptClient,
    request: Request,
    mut cancelled: watch::Receiver<bool>,
) -> Outcome {
    let users = users(&request.identities);
    if users.is_empty() {
        warn!("polkit asked for no user this agent can find");
        return Outcome::Cancelled;
    }
    let ask = Ask {
        prompt: Prompt {
            kind: Kind::Polkit,
            title: "{app} needs your password".into(),
            message: request.message.clone(),
            fields: vec![FieldSpec {
                id: "password".into(),
                label: format!("Password for {}", users[0].1),
                secret: true,
                ..Default::default()
            }],
            identities: if users.len() > 1 {
                users.iter().map(|(_, name)| name.clone()).collect()
            } else {
                Vec::new()
            },
            details: Some(details(&request)),
            cancel: "Cancel".into(),
            action: Some("Allow".into()),
            ..Default::default()
        },
        title_unknown: Some("An administrator's password is needed".into()),
        pid: request.subject_pid(),
        user_started: false,
        checks: true,
        subject: None,
    };
    let Ok((id, mut answers)) = prompts.ask(ask).await else {
        return Outcome::Cancelled;
    };
    loop {
        let answer = tokio::select! {
            answer = answers.recv() => answer,
            _ = cancelled.wait_for(|c| *c) => {
                info!("polkit cancelled its request");
                prompts.done(id);
                return Outcome::Cancelled;
            }
        };
        let Some(Answer::Values {
            values, identity, ..
        }) = answer
        else {
            return Outcome::Cancelled;
        };
        let user = &users[identity.min(users.len() - 1)].0;
        let password = values
            .iter()
            .find(|(k, _)| k == "password")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let attempt = tokio::select! {
            result = attempt(&prompts, id, user, &request.cookie, password.expose()) => result,
            _ = cancelled.wait_for(|c| *c) => {
                prompts.done(id);
                return Outcome::Cancelled;
            }
        };
        match attempt {
            Attempt::Authorised => {
                prompts.done(id);
                return Outcome::Authorised;
            }
            Attempt::Wrong(message) => prompts.wrong(id, message),
        }
    }
}

enum Attempt {
    Authorised,
    Wrong(String),
}

/// One conversation: PAM's questions answered from the prompt, its messages passed on.
async fn attempt(
    prompts: &PromptClient,
    id: u64,
    user: &str,
    cookie: &str,
    password: &str,
) -> Attempt {
    let broken = || Attempt::Wrong("Couldn't check the password. Try again.".into());
    let mut conversation = match Conversation::start(&helper_socket(), user, cookie).await {
        Ok(c) => c,
        Err(err) => {
            warn!("polkit's helper: {err}");
            return broken();
        }
    };
    let mut answered = false;
    let mut errors: Vec<String> = Vec::new();
    loop {
        let line = match tokio::time::timeout(Duration::from_secs(120), conversation.next()).await {
            Ok(Ok(Some(line))) => line,
            Ok(Ok(None)) | Ok(Err(_)) | Err(_) => return broken(),
        };
        match line {
            Line::Secret(_) if !answered => {
                answered = true;
                if conversation.respond(password).await.is_err() {
                    return broken();
                }
                prompts.status(id, Status::Checking);
            }
            // A second secret this prompt cannot answer.
            Line::Secret(_) => {
                let _ = conversation.respond("").await;
            }
            Line::Visible(_) => {
                let _ = conversation.respond(user).await;
            }
            Line::Info(message) => {
                errors.push(message.clone());
                prompts.status(id, Status::Info(message));
            }
            Line::Error(message) => errors.push(message),
            Line::Success => return Attempt::Authorised,
            Line::Failure => {
                let locked = errors
                    .iter()
                    .any(|m| m.starts_with("The account is locked"));
                return Attempt::Wrong(if locked {
                    "Too many attempts. Try again later.".into()
                } else {
                    "Wrong password. Try again.".into()
                });
            }
        }
    }
}
