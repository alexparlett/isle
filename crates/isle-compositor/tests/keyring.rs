//! The keyring prompter, driven as gnome-keyring drives gcr's: begin, a prompt with the
//! caller's half of the exchange, the password back encrypted, stop. Needs `--features
//! test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::collections::HashMap;

use common::{Isle, wait_until};
use isle_prompter::exchange::{Exchange, receive};
use tokio::sync::mpsc;
use zbus::zvariant::{OwnedValue, Value};

const RETURN: u32 = 28;
const S: u32 = 31;
const E: u32 = 18;
const C: u32 = 46;

/// What the prompter called back with: the reply, its properties and the exchange; or done.
#[derive(Debug)]
enum Called {
    Ready(String, HashMap<String, OwnedValue>, String),
    Done,
}

struct Callback(mpsc::UnboundedSender<Called>);

#[zbus::interface(name = "org.gnome.keyring.internal.Prompter.Callback")]
impl Callback {
    fn prompt_ready(
        &self,
        reply: String,
        properties: HashMap<String, OwnedValue>,
        exchange: String,
    ) {
        let _ = self.0.send(Called::Ready(reply, properties, exchange));
    }

    fn prompt_done(&self) {
        let _ = self.0.send(Called::Done);
    }
}

const CALLBACK: &str = "/org/gnome/keyring/Prompt/p1";

async fn prompts(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.prompts().await.unwrap()).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unlock_prompt_returns_the_password_through_the_exchange() {
    let isle = Isle::start("keyring-unlock").await;
    let (send, mut called) = mpsc::unbounded_channel();
    let keyring = zbus::connection::Builder::address(isle.bus_address().as_str())
        .unwrap()
        .serve_at(CALLBACK, Callback(send))
        .unwrap()
        .build()
        .await
        .unwrap();
    let prompter = zbus::Proxy::new(
        &keyring,
        "org.gnome.keyring.SystemPrompter",
        "/org/gnome/keyring/Prompter",
        "org.gnome.keyring.internal.Prompter",
    )
    .await
    .unwrap();
    let path = zbus::zvariant::ObjectPath::try_from(CALLBACK).unwrap();
    let _: () = prompter.call("BeginPrompting", &(&path,)).await.unwrap();
    let Some(Called::Ready(reply, _, opening)) = called.recv().await else {
        panic!("no turn");
    };
    assert_eq!(reply, "");

    let mine = Exchange::new().unwrap();
    let mut props: HashMap<&str, Value<'_>> = HashMap::new();
    props.insert("title", Value::from("Unlock Keyring"));
    props.insert("message", Value::from("Authentication required"));
    props.insert(
        "description",
        Value::from("An application wants access to the keyring “login”, but it is locked"),
    );
    props.insert(
        "choice-label",
        Value::from("Automatically unlock this keyring whenever I'm logged in"),
    );
    props.insert("continue-label", Value::from("_Unlock"));
    let _: () = prompter
        .call("PerformPrompt", &(&path, "password", props, mine.begin()))
        .await
        .unwrap();

    wait_until(|| async { prompts(&isle).await["armed"] == true }).await;
    let shown = prompts(&isle).await;
    assert_eq!(shown["title"], "Authentication required", "{shown}");
    let test = isle.test().await;
    for key in [S, E, C, RETURN] {
        test.key(key, true).await.unwrap();
        test.key(key, false).await.unwrap();
    }
    let Some(Called::Ready(reply, properties, closing)) = called.recv().await else {
        panic!("no answer");
    };
    assert_eq!(reply, "yes");
    // gcr's doubled variant around each property.
    let chosen = properties.get("choice-chosen").unwrap();
    assert!(matches!(&**chosen, Value::Value(_)), "{chosen:?}");
    let key = mine.key(&opening).unwrap();
    let secret = receive(&closing, &key).unwrap();
    assert_eq!(secret.as_slice(), b"sec");

    let _: () = prompter.call("StopPrompting", &(&path,)).await.unwrap();
    assert!(matches!(called.recv().await, Some(Called::Done)));
    wait_until(|| async { prompts(&isle).await["showing"].is_null() }).await;
}
