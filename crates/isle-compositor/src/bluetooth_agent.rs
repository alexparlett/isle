//! BlueZ's pairing agent on the worker: each of its questions one of Isle's prompts
//! (`p9-lock-and-prompts.md` choice 20, `structure/prompts.md` Bluetooth pairing).

use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};

use isle_bluetooth::agent::{self, Ask as BlueAsk, Reply};
use isle_bus::NameWatch;
use isle_shell::prompts::{FieldSpec, Kind, Prompt};
use tokio::sync::watch;
use tracing::debug;

use crate::prompts::{Answer, Ask, PromptClient};

const NAME: &str = "org.bluez";

/// The prompt showing a code to type on a device, by the device's path.
type Shown = Arc<Mutex<HashMap<String, u64>>>;

/// Registers the agent whenever BlueZ is on the bus, again after it restarts.
pub async fn run(prompts: PromptClient) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bus = isle_bus::system().await?;
    let mut name = NameWatch::new(&bus, NAME).await?;
    let shown: Shown = Default::default();
    loop {
        name.appeared().await?;
        let conn = zbus::connection::Builder::system()?.build().await?;
        let (client, shown) = (prompts.clone(), shown.clone());
        agent::serve(&conn, move |device, ask, cancelled| {
            answer(client.clone(), shown.clone(), device, ask, cancelled)
        })
        .await?;
        name.vanished().await?;
        debug!("BlueZ left the bus");
        drop(conn);
    }
}

/// A device's name, else its address, and its kind's glyph.
async fn device(path: &str) -> (String, String) {
    let read = async {
        let bus = isle_bus::system().await.ok()?;
        let proxy = zbus::Proxy::new(&bus, NAME, path.to_owned(), "org.bluez.Device1")
            .await
            .ok()?;
        let alias: String = proxy.get_property("Alias").await.ok()?;
        let icon: String = proxy.get_property("Icon").await.unwrap_or_default();
        Some((alias, icon))
    };
    let (name, icon) = read.await.unwrap_or_else(|| {
        let address = path
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim_start_matches("dev_")
            .replace('_', ":");
        (address, String::new())
    });
    let glyph = match icon.as_str() {
        "audio-headphones" | "audio-headset" => "headphones",
        "input-keyboard" => "keyboard",
        "input-mouse" | "input-tablet" => "mouse",
        "input-gaming" => "game-controller",
        "phone" => "device-mobile",
        "audio-card" => "speaker-hifi",
        _ => "bluetooth",
    };
    (name, glyph.to_owned())
}

/// Whether the device has paired, which ends a code shown for it.
async fn paired(path: &str) -> bool {
    async {
        let bus = isle_bus::system().await.ok()?;
        let proxy = zbus::Proxy::new(&bus, NAME, path.to_owned(), "org.bluez.Device1")
            .await
            .ok()?;
        proxy.get_property::<bool>("Paired").await.ok()
    }
    .await
    .unwrap_or(false)
}

async fn answer(
    prompts: PromptClient,
    shown: Shown,
    device_path: String,
    ask: BlueAsk,
    mut cancelled: watch::Receiver<bool>,
) -> Reply {
    // Each digit typed on the device moves the code's cells on.
    if let BlueAsk::DisplayPasskey { entered, .. } = &ask
        && let Some(id) = shown.lock().unwrap().get(&device_path).copied()
    {
        let entered = *entered as usize;
        prompts.update(id, move |p| p.typed = Some(entered));
        return Reply::Yes;
    }
    let (name, glyph) = device(&device_path).await;
    let mut prompt = Prompt {
        kind: Kind::Bluetooth,
        glyph: Some(glyph),
        cancel: "Cancel".into(),
        action: Some("Pair".into()),
        ..Default::default()
    };
    match &ask {
        BlueAsk::Confirm(passkey) => {
            prompt.title = format!("Pair with {name}?");
            prompt.message = format!("Check that {name} shows this code.");
            prompt.code = Some(format!("{passkey:06}"));
        }
        BlueAsk::Passkey | BlueAsk::PinCode => {
            prompt.title = format!("Pair with {name}");
            prompt.fields = vec![FieldSpec {
                id: "code".into(),
                label: if ask == BlueAsk::Passkey {
                    "Code"
                } else {
                    "PIN"
                }
                .into(),
                mono: true,
                ..Default::default()
            }];
        }
        BlueAsk::DisplayPasskey { passkey, entered } => {
            prompt.title = format!("Type this code on {name}");
            prompt.code = Some(format!("{passkey:06}"));
            prompt.typed = Some(*entered as usize);
            prompt.action = None;
        }
        BlueAsk::DisplayPinCode(pin) => {
            prompt.title = format!("Type this code on {name}");
            prompt.code = Some(pin.clone());
            prompt.typed = Some(0);
            prompt.action = None;
        }
        BlueAsk::Authorize => prompt.title = format!("{name} wants to pair"),
        BlueAsk::Service(uuid) => {
            prompt.title = format!("Allow {name} to connect?");
            prompt.message = agent::service_name(uuid);
            prompt.cancel = "Don't Allow".into();
            prompt.action = Some("Allow".into());
        }
    }
    let display = matches!(
        ask,
        BlueAsk::DisplayPasskey { .. } | BlueAsk::DisplayPinCode(_)
    );
    let Ok((id, mut answers)) = prompts
        .ask(Ask {
            prompt,
            title_unknown: None,
            pid: None,
            user_started: false,
            checks: false,
            subject: Some(device_path.clone()),
        })
        .await
    else {
        return Reply::No;
    };
    if display {
        // Shown until the device pairs, BlueZ cancels or the user does.
        shown.lock().unwrap().insert(device_path.clone(), id);
        let shown = shown.clone();
        tokio::spawn(async move {
            let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
            loop {
                tokio::select! {
                    _ = answers.recv() => break,
                    _ = async { let _ = cancelled.wait_for(|c| *c).await; } => break,
                    _ = tokio::time::sleep(Duration::from_millis(500)) => {
                        if paired(&device_path).await || tokio::time::Instant::now() > deadline {
                            break;
                        }
                    }
                }
            }
            shown.lock().unwrap().remove(&device_path);
            prompts.done(id);
        });
        return Reply::Yes;
    }
    let answer = tokio::select! {
        answer = answers.recv() => answer,
        _ = cancelled.wait_for(|c| *c) => {
            prompts.done(id);
            return Reply::No;
        }
    };
    match (answer, &ask) {
        (Some(answer @ Answer::Values { .. }), BlueAsk::Passkey) => answer
            .value("code")
            .and_then(|c| c.expose().trim().parse().ok())
            .map(Reply::Passkey)
            .unwrap_or(Reply::No),
        (Some(answer @ Answer::Values { .. }), BlueAsk::PinCode) => answer
            .value("code")
            .map(|c| Reply::PinCode(c.expose().chars().take(16).collect()))
            .unwrap_or(Reply::No),
        (Some(Answer::Values { .. }), _) => Reply::Yes,
        _ => Reply::No,
    }
}
