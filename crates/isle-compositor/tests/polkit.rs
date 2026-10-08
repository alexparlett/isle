//! The polkit agent: registered for the session, each request a prompt, each attempt a
//! conversation with polkit's helper on its socket, cancels both ways. Needs `--features
//! test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::collections::HashMap;

use common::{Isle, wait_until};
use isle_fakes::polkit::{Helper, Polkit};

const RETURN: u32 = 28;
const ESC: u32 = 1;
const I: u32 = 23;
const S: u32 = 31;
const L: u32 = 38;
const E: u32 = 18;
const X: u32 = 45;

async fn start(name: &str) -> (Isle, Polkit, Helper) {
    let isle = Isle::start_env(
        name,
        "",
        |_| {},
        |dir| {
            vec![
                (
                    "ISLE_POLKIT_SOCKET".into(),
                    dir.join("helper.socket").display().to_string(),
                ),
                ("XDG_SESSION_ID".into(), "c7".into()),
            ]
        },
    )
    .await;
    let helper = Helper::start(&isle.dir.join("helper.socket"), "isle").unwrap();
    let polkit = Polkit::start(&isle.system_address()).await.unwrap();
    wait_until(|| async { polkit.registered().as_deref() == Some("c7") }).await;
    (isle, polkit, helper)
}

async fn prompts(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.prompts().await.unwrap()).unwrap()
}

async fn type_keys(isle: &Isle, keys: &[u32]) {
    let test = isle.test().await;
    for &key in keys {
        test.key(key, true).await.unwrap();
        test.key(key, false).await.unwrap();
    }
}

fn me() -> u32 {
    unsafe { libc::getuid() }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wrong_password_asks_again_and_the_right_one_authorises() {
    let (isle, polkit, helper) = start("polkit-auth").await;
    let request = tokio::spawn(async move {
        let mut details = HashMap::new();
        details.insert("program", "/usr/bin/true");
        polkit
            .begin(
                "cookie-1",
                "org.isle.test.run",
                "Run a test",
                details,
                &[me()],
            )
            .await
    });
    wait_until(|| async { prompts(&isle).await["armed"] == true }).await;
    let shown = prompts(&isle).await;
    assert_eq!(
        shown["title"], "An administrator's password is needed",
        "{shown}"
    );

    type_keys(&isle, &[X, RETURN]).await;
    wait_until(|| async {
        prompts(&isle).await["status"]
            .as_str()
            .is_some_and(|s| s.contains("Wrong password"))
    })
    .await;
    type_keys(&isle, &[I, S, L, E, RETURN]).await;
    assert_eq!(request.await.unwrap(), Ok(()));
    assert_eq!(helper.attempts(), 2, "one conversation an attempt");
    wait_until(|| async { prompts(&isle).await["showing"].is_null() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn polkit_cancelling_closes_the_prompt_and_escape_refuses() {
    let (isle, polkit, _helper) = start("polkit-cancel").await;
    let polkit = std::sync::Arc::new(polkit);
    let asking = polkit.clone();
    let request = tokio::spawn(async move {
        asking
            .begin(
                "cookie-2",
                "org.isle.test.run",
                "Run a test",
                HashMap::new(),
                &[me()],
            )
            .await
    });
    wait_until(|| async { !prompts(&isle).await["showing"].is_null() }).await;
    polkit.cancel("cookie-2").await.unwrap();
    let result = request.await.unwrap();
    assert!(result.unwrap_err().contains("Cancelled"));
    wait_until(|| async { prompts(&isle).await["showing"].is_null() }).await;

    let asking = polkit.clone();
    let request = tokio::spawn(async move {
        asking
            .begin(
                "cookie-3",
                "org.isle.test.run",
                "Run a test",
                HashMap::new(),
                &[me()],
            )
            .await
    });
    wait_until(|| async { !prompts(&isle).await["showing"].is_null() }).await;
    type_keys(&isle, &[ESC]).await;
    let result = request.await.unwrap();
    assert!(
        result.unwrap_err().contains("Cancelled"),
        "Escape is polkit's cancel"
    );
}
