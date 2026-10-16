//! A VPN's secrets through its plugin's auth dialog: in external-UI mode the dialog describes
//! its fields and Isle asks for them; a plugin without that mode shows its own window
//! (`lock-and-prompts-2026.md` §4.3).

use std::{collections::HashMap, path::PathBuf, process::Stdio, time::Duration};

use isle_network::agent::{Answer as AgentAnswer, REQUEST_NEW, Request};
use isle_shell::prompts::{FieldSpec, Kind, Prompt, Status};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::watch,
};
use tracing::{info, warn};
use zbus::zvariant::Value;

use crate::prompts::PromptClient;

/// A VPN plugin, from its `.name` file.
struct Plugin {
    dialog: PathBuf,
    external_ui: bool,
    hints: bool,
}

fn plugin_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("NM_VPN_PLUGIN_DIR")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    dirs.push("/usr/lib/NetworkManager/VPN".into());
    dirs.push("/etc/NetworkManager/VPN".into());
    dirs
}

/// The plugin whose `.name` file names `service`.
fn plugin(service: &str) -> Option<Plugin> {
    for dir in plugin_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "name") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let groups = key_file(&text);
            let named = groups
                .iter()
                .find(|(g, _)| g == "VPN Connection")
                .and_then(|(_, kv)| kv.get("service"))
                .is_some_and(|s| s == service);
            if !named {
                continue;
            }
            let gnome = groups
                .iter()
                .find(|(g, _)| g == "GNOME")
                .map(|(_, kv)| kv)?;
            return Some(Plugin {
                dialog: PathBuf::from(gnome.get("auth-dialog")?),
                external_ui: gnome
                    .get("supports-external-ui-mode")
                    .is_some_and(|v| v == "true"),
                hints: gnome.get("supports-hints").is_some_and(|v| v == "true"),
            });
        }
    }
    None
}

/// A key file's groups in order, each with its keys.
fn key_file(text: &str) -> Vec<(String, HashMap<String, String>)> {
    let mut groups: Vec<(String, HashMap<String, String>)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            groups.push((name.to_owned(), HashMap::new()));
        } else if let (Some((key, value)), Some(group)) = (line.split_once('='), groups.last_mut())
        {
            group
                .1
                .insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    groups
}

/// The dialog's input: the connection's data and secrets, then `DONE`.
fn dialog_input(data: &HashMap<String, String>, secrets: &HashMap<String, String>) -> String {
    let mut input = String::new();
    for (k, v) in data {
        input.push_str(&format!("DATA_KEY={k}\nDATA_VAL={v}\n\n"));
    }
    for (k, v) in secrets {
        input.push_str(&format!("SECRET_KEY={k}\nSECRET_VAL={v}\n\n"));
    }
    input.push_str("DONE\n\n");
    input
}

fn vpn_reply(secrets: HashMap<String, String>) -> AgentAnswer {
    isle_network::agent::secrets("vpn", vec![("secrets", Value::from(secrets))])
}

pub async fn answer(
    prompts: PromptClient,
    request: Request,
    cancelled: watch::Receiver<bool>,
) -> AgentAnswer {
    let Some((service, data)) = request.vpn() else {
        return AgentAnswer::NoSecrets;
    };
    let Some(plugin) = plugin(&service) else {
        warn!(service, "no VPN plugin names this service");
        return AgentAnswer::NoSecrets;
    };
    let mut args = vec![
        "-u".to_owned(),
        request.uuid(),
        "-n".into(),
        request.id(),
        "-s".into(),
        service.clone(),
        "-i".into(),
    ];
    if request.flags & REQUEST_NEW != 0 {
        args.push("-r".into());
    }
    if plugin.hints {
        for hint in &request.hints {
            args.push("-t".into());
            args.push(hint.clone());
        }
    }
    if plugin.external_ui {
        args.push("--external-ui-mode".into());
        external(prompts, request, plugin, args, data, cancelled).await
    } else {
        own_window(plugin, args, data).await
    }
}

/// The dialog describes its fields and exits; Isle asks for those it marks.
async fn external(
    prompts: PromptClient,
    request: Request,
    plugin: Plugin,
    args: Vec<String>,
    data: HashMap<String, String>,
    cancelled: watch::Receiver<bool>,
) -> AgentAnswer {
    let output = async {
        let mut child = Command::new(&plugin.dialog)
            .args(&args)
            .envs(crate::child_env::vars())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        let mut stdin = child.stdin.take().expect("piped");
        stdin
            .write_all(dialog_input(&data, &HashMap::new()).as_bytes())
            .await?;
        drop(stdin);
        let mut out = String::new();
        child
            .stdout
            .take()
            .expect("piped")
            .read_to_string(&mut out)
            .await?;
        let _ = child.wait().await;
        Ok::<_, std::io::Error>(out)
    };
    let text = match tokio::time::timeout(Duration::from_secs(20), output).await {
        Ok(Ok(text)) => text,
        Ok(Err(err)) => {
            warn!(dialog = %plugin.dialog.display(), "the VPN's auth dialog: {err}");
            return AgentAnswer::NoSecrets;
        }
        Err(_) => return AgentAnswer::NoSecrets,
    };
    let groups = key_file(&text);
    let ui = groups
        .iter()
        .find(|(g, _)| g == "VPN Plugin UI")
        .map(|(_, kv)| kv.clone())
        .unwrap_or_default();
    let entries: Vec<(String, HashMap<String, String>)> = groups
        .into_iter()
        .filter(|(g, _)| g != "VPN Plugin UI")
        .collect();
    if entries.is_empty() {
        return AgentAnswer::NoSecrets;
    }
    let mut given: HashMap<String, String> = HashMap::new();
    let mut fields = Vec::new();
    for (key, kv) in &entries {
        let ask = kv.get("ShouldAsk").is_some_and(|v| v == "true");
        if !ask {
            if let Some(value) = kv.get("Value").filter(|v| !v.is_empty()) {
                given.insert(key.clone(), value.clone());
            }
            continue;
        }
        let secret = kv.get("IsSecret").is_some_and(|v| v == "true");
        fields.push(FieldSpec {
            id: key.clone(),
            label: kv
                .get("Label")
                .map(|l| l.trim_end_matches(':').to_owned())
                .unwrap_or_else(|| key.clone()),
            secret,
            show: secret,
            ..Default::default()
        });
    }
    if fields.is_empty() {
        return vpn_reply(given);
    }
    info!(vpn = request.id(), "the VPN asks");
    let keys: Vec<String> = fields.iter().map(|f| f.id.clone()).collect();
    let wrong = request.flags & REQUEST_NEW != 0;
    let prompt = Prompt {
        kind: Kind::Vpn,
        title: format!("“{}” needs a password", request.id()),
        message: ui.get("Description").cloned().unwrap_or_default(),
        fields,
        cancel: "Cancel".into(),
        action: Some("Connect".into()),
        status: if wrong {
            Status::Warning(format!(
                "Couldn't connect to “{}”. Check the password and try again.",
                request.id()
            ))
        } else {
            Status::None
        },
        wrongs: u32::from(wrong),
        ..Default::default()
    };
    crate::network_secrets::ask(&prompts, &request, prompt, cancelled, move |answer| {
        let mut secrets = given;
        for key in &keys {
            if let Some(value) = answer.value(key) {
                secrets.insert(key.clone(), value.expose().to_owned());
            }
        }
        Some(vpn_reply(secrets))
    })
    .await
}

/// The plugin's own window: it prints `key` and `value` lines and a blank one, then waits
/// for `QUIT`.
async fn own_window(
    plugin: Plugin,
    args: Vec<String>,
    data: HashMap<String, String>,
) -> AgentAnswer {
    let run = async {
        let mut child = Command::new(&plugin.dialog)
            .args(&args)
            .envs(crate::child_env::vars())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        let mut stdin = child.stdin.take().expect("piped");
        stdin
            .write_all(dialog_input(&data, &HashMap::new()).as_bytes())
            .await?;
        let mut lines = BufReader::new(child.stdout.take().expect("piped")).lines();
        let mut secrets = HashMap::new();
        while let Some(key) = lines.next_line().await? {
            if key.is_empty() {
                break;
            }
            let value = lines.next_line().await?.unwrap_or_default();
            secrets.insert(key, value);
        }
        let _ = stdin.write_all(b"QUIT\n").await;
        let status = child.wait().await?;
        Ok::<_, std::io::Error>(status.success().then_some(secrets))
    };
    match tokio::time::timeout(Duration::from_secs(115), run).await {
        Ok(Ok(Some(secrets))) => vpn_reply(secrets),
        Ok(Ok(None)) => AgentAnswer::UserCanceled,
        _ => AgentAnswer::NoSecrets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plugins_description_reads_as_groups() {
        let text = "[VPN Plugin UI]\nVersion=2\nDescription=Sign in\n\n[password]\nValue=\nLabel=Password:\nIsSecret=true\nShouldAsk=true\n";
        let groups = key_file(text);
        assert_eq!(groups[0].0, "VPN Plugin UI");
        assert_eq!(groups[1].1["Label"], "Password:");
        assert_eq!(groups[1].1["ShouldAsk"], "true");
    }
}
