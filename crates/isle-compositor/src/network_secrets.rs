//! NetworkManager's secret agent on the worker: a Wi-Fi password, an 802.1X sign-in or a
//! VPN's secrets asked in Isle's prompt (`p9-lock-and-prompts.md` choice 20,
//! `structure/prompts.md` Network secrets).

use std::{error::Error, time::Duration};

use isle_bus::NameWatch;
use isle_network::agent::{self, Answer as AgentAnswer, REQUEST_NEW, Request, USER_REQUESTED};
use isle_shell::prompts::{FieldSpec, Kind, Prompt, Status};
use tokio::sync::watch;
use tracing::debug;
use zbus::zvariant::Value;

use crate::prompts::{Answer, Ask, PromptClient};

const NAME: &str = "org.freedesktop.NetworkManager";
/// NetworkManager gives up on an agent after two minutes and sends no cancel; the prompt
/// goes a little before.
const PATIENCE: Duration = Duration::from_secs(115);

/// Registers the agent whenever NetworkManager is on the bus, again after it restarts.
pub async fn run(prompts: PromptClient) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bus = isle_bus::system().await?;
    let mut name = NameWatch::new(&bus, NAME).await?;
    loop {
        name.appeared().await?;
        let conn = zbus::connection::Builder::system()?.build().await?;
        let client = prompts.clone();
        agent::serve(
            &conn,
            move |request, cancelled| answer(client.clone(), request, cancelled),
            |connection| {
                tokio::spawn(save(connection));
            },
        )
        .await?;
        name.vanished().await?;
        debug!("NetworkManager left the bus");
        drop(conn);
    }
}

/// The security a network's key management names.
fn security(key_mgmt: &str) -> &'static str {
    match key_mgmt {
        "sae" => "WPA3 Personal",
        "wpa-psk" | "wpa-none" => "WPA2 Personal",
        "none" => "WEP",
        "wpa-eap" | "wpa-eap-suite-b-192" => "WPA2 Enterprise",
        "owe" => "Enhanced Open",
        _ => "",
    }
}

async fn answer(
    prompts: PromptClient,
    request: Request,
    cancelled: watch::Receiver<bool>,
) -> AgentAnswer {
    // What the keyring keeps for this connection, unless NetworkManager says it was wrong.
    if request.flags & REQUEST_NEW == 0 {
        let kept = agent::keyring::lookup(&request.uuid(), &request.setting)
            .await
            .unwrap_or_default();
        if !kept.is_empty() {
            debug!(setting = request.setting, "secrets from the keyring");
            let items: std::collections::HashMap<String, Value<'static>> =
                if request.setting == "vpn" {
                    std::collections::HashMap::from([("secrets".to_owned(), Value::from(kept))])
                } else {
                    kept.into_iter().map(|(k, v)| (k, Value::from(v))).collect()
                };
            return AgentAnswer::Secrets(std::collections::HashMap::from([(
                request.setting.clone(),
                items,
            )]));
        }
    }
    if !request.interactive() {
        return AgentAnswer::NoSecrets;
    }
    let network = request.ssid().unwrap_or_else(|| request.id());
    let wrong = request.flags & REQUEST_NEW != 0;
    let status = if wrong {
        Status::Warning(format!(
            "Couldn't connect to “{network}”. Check the password and try again."
        ))
    } else {
        Status::None
    };
    match request.setting.as_str() {
        "802-11-wireless-security" => {
            let key_mgmt = request.key_mgmt().unwrap_or_default();
            let key = match key_mgmt.as_str() {
                "none" => {
                    let index = request
                        .connection
                        .get("802-11-wireless-security")
                        .and_then(|s| s.get("wep-tx-keyidx"))
                        .and_then(|v| u32::try_from(v).ok())
                        .unwrap_or(0);
                    format!("wep-key{index}")
                }
                "ieee8021x" => "leap-password".into(),
                _ => "psk".into(),
            };
            let prompt = Prompt {
                kind: Kind::Network,
                title: format!("“{network}” needs a password"),
                message: security(&key_mgmt).into(),
                fields: vec![FieldSpec {
                    id: key.clone(),
                    label: "Password".into(),
                    secret: true,
                    show: true,
                    ..Default::default()
                }],
                cancel: "Cancel".into(),
                action: Some("Join".into()),
                status,
                wrongs: u32::from(wrong),
                ..Default::default()
            };
            ask(&prompts, &request, prompt, cancelled, |answer| {
                let value = answer.value(&key)?.expose().to_owned();
                Some(agent::secrets(
                    "802-11-wireless-security",
                    vec![(key.as_str(), Value::from(value))],
                ))
            })
            .await
        }
        "802-1x" => {
            // The fields the request hints at, else those its EAP method needs.
            let tls = request.eap().iter().any(|m| m == "tls");
            let wanted = |key: &str| {
                if request.hints.is_empty() {
                    match key {
                        "identity" => true,
                        "password" => !tls,
                        "private-key-password" => tls,
                        _ => false,
                    }
                } else {
                    request.hints.iter().any(|h| h == key)
                }
            };
            let fields: Vec<FieldSpec> = [
                ("identity", "User name", false),
                ("password", "Password", true),
                ("private-key-password", "Private key password", true),
            ]
            .into_iter()
            .filter(|(key, _, _)| wanted(key))
            .map(|(key, label, secret)| FieldSpec {
                id: key.into(),
                label: label.into(),
                secret,
                ..Default::default()
            })
            .collect();
            let keys: Vec<String> = fields.iter().map(|f| f.id.clone()).collect();
            let prompt = Prompt {
                kind: Kind::Network,
                title: format!("Sign in to “{network}”"),
                fields,
                cancel: "Cancel".into(),
                action: Some("Join".into()),
                status,
                wrongs: u32::from(wrong),
                ..Default::default()
            };
            ask(&prompts, &request, prompt, cancelled, |answer| {
                let items = keys
                    .iter()
                    .filter_map(|k| {
                        let v = answer.value(k)?.expose().to_owned();
                        Some((k.as_str(), Value::from(v)))
                    })
                    .collect();
                Some(agent::secrets("802-1x", items))
            })
            .await
        }
        "vpn" => crate::vpn_secrets::answer(prompts, request, cancelled).await,
        _ => AgentAnswer::NoSecrets,
    }
}

/// Shows `prompt` and turns its answer into secrets with `secrets`.
pub async fn ask(
    prompts: &PromptClient,
    request: &Request,
    prompt: Prompt,
    mut cancelled: watch::Receiver<bool>,
    secrets: impl FnOnce(&Answer) -> Option<AgentAnswer>,
) -> AgentAnswer {
    let ask = Ask {
        prompt,
        title_unknown: None,
        pid: None,
        user_started: request.flags & USER_REQUESTED != 0,
        checks: false,
        subject: None,
    };
    let Ok((id, mut answers)) = prompts.ask(ask).await else {
        return AgentAnswer::NoSecrets;
    };
    let answer = tokio::select! {
        answer = answers.recv() => answer,
        _ = cancelled.wait_for(|c| *c) => {
            prompts.done(id);
            return AgentAnswer::UserCanceled;
        }
        _ = tokio::time::sleep(PATIENCE) => {
            prompts.done(id);
            return AgentAnswer::UserCanceled;
        }
    };
    match answer {
        Some(answer @ Answer::Values { .. }) => secrets(&answer).unwrap_or(AgentAnswer::NoSecrets),
        _ => AgentAnswer::UserCanceled,
    }
}

/// Keeps the agent-owned secrets NetworkManager hands back, in the keyring as nm-applet does.
async fn save(connection: agent::Settings) {
    let text = |setting: &str, key: &str| {
        let v = connection.get(setting)?.get(key)?;
        String::try_from(v.try_clone().ok()?).ok()
    };
    let (Some(id), Some(uuid)) = (text("connection", "id"), text("connection", "uuid")) else {
        return;
    };
    for (setting, items) in &connection {
        for (key, value) in items {
            let owned = items
                .get(&format!("{key}-flags"))
                .and_then(|f| u32::try_from(f).ok())
                .is_some_and(|f| f == 1);
            let Ok(secret) = String::try_from(value.try_clone().unwrap_or(value.clone())) else {
                continue;
            };
            if owned && let Err(err) = agent::keyring::save(&id, &uuid, setting, key, &secret).await
            {
                debug!("the keyring did not keep a network secret: {err}");
            }
        }
    }
}
