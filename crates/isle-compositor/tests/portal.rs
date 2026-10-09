//! The portal backend through a fake frontend that owns `org.freedesktop.portal.Desktop` on
//! the test's bus: who may call it, and its Access dialog. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::collections::HashMap;

use common::{Isle, wait_until};
use zvariant::{ObjectPath, OwnedValue};

const BACKEND: &str = "org.freedesktop.impl.portal.desktop.isle";
const PATH: &str = "/org/freedesktop/portal/desktop";
const RETURN: u32 = 28;
const ESCAPE: u32 = 1;

/// A connection that owns the frontend's name, as xdg-desktop-portal would.
pub async fn frontend(isle: &Isle) -> zbus::Connection {
    let conn = zbus::connection::Builder::address(isle.bus_address().as_str())
        .unwrap()
        .build()
        .await
        .unwrap();
    conn.request_name("org.freedesktop.portal.Desktop")
        .await
        .unwrap();
    conn
}

async fn access(
    conn: &zbus::Connection,
    handle: &str,
) -> zbus::Result<(u32, HashMap<String, OwnedValue>)> {
    let reply = conn
        .call_method(
            Some(BACKEND),
            PATH,
            Some("org.freedesktop.impl.portal.Access"),
            "AccessDialog",
            &(
                ObjectPath::try_from(handle).unwrap(),
                "isle.test.App",
                "",
                "Allow Test to take screenshots?",
                "Test wants to take screenshots",
                "",
                HashMap::<String, OwnedValue>::new(),
            ),
        )
        .await?;
    reply.body().deserialize()
}

async fn prompts(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.prompts().await.unwrap()).unwrap()
}

async fn press(isle: &Isle, key: u32) {
    let test = isle.test().await;
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_frontend_may_call_and_access_asks_in_a_prompt() {
    let isle = Isle::start("portal-access").await;
    // The name is Isle's, and its versions read in lower case.
    let other = zbus::connection::Builder::address(isle.bus_address().as_str())
        .unwrap()
        .build()
        .await
        .unwrap();
    let version: u32 = zbus::fdo::PropertiesProxy::builder(&other)
        .destination(BACKEND)
        .unwrap()
        .path(PATH)
        .unwrap()
        .build()
        .await
        .unwrap()
        .get(
            zbus::names::InterfaceName::try_from("org.freedesktop.impl.portal.ScreenCast").unwrap(),
            "version",
        )
        .await
        .unwrap()
        .try_into()
        .unwrap();
    assert!(version >= 4);

    // Anyone but the frontend is refused.
    let refused = access(&other, "/org/freedesktop/portal/desktop/request/1_1/x").await;
    assert!(
        matches!(&refused, Err(zbus::Error::MethodError(name, _, _)) if name.as_str() == "org.freedesktop.DBus.Error.AccessDenied"),
        "{refused:?}"
    );

    // The frontend's call is a prompt: Allow grants it.
    let frontend = frontend(&isle).await;
    let asked = tokio::spawn({
        let frontend = frontend.clone();
        async move { access(&frontend, "/org/freedesktop/portal/desktop/request/1_1/a").await }
    });
    wait_until(|| async { prompts(&isle).await["showing"].is_u64() }).await;
    wait_until(|| async { prompts(&isle).await["armed"] == true }).await;
    press(&isle, RETURN).await;
    let (response, _) = asked.await.unwrap().unwrap();
    assert_eq!(response, 0);

    // Escape denies it.
    let asked = tokio::spawn({
        let frontend = frontend.clone();
        async move { access(&frontend, "/org/freedesktop/portal/desktop/request/1_1/b").await }
    });
    wait_until(|| async { prompts(&isle).await["showing"].is_u64() }).await;
    wait_until(|| async { prompts(&isle).await["armed"] == true }).await;
    press(&isle, ESCAPE).await;
    let (response, _) = asked.await.unwrap().unwrap();
    assert_eq!(response, 1);

    // The frontend closing the request takes the prompt away and answers it.
    let asked = tokio::spawn({
        let frontend = frontend.clone();
        async move { access(&frontend, "/org/freedesktop/portal/desktop/request/1_1/c").await }
    });
    wait_until(|| async { prompts(&isle).await["showing"].is_u64() }).await;
    frontend
        .call_method(
            Some(BACKEND),
            "/org/freedesktop/portal/desktop/request/1_1/c",
            Some("org.freedesktop.impl.portal.Request"),
            "Close",
            &(),
        )
        .await
        .unwrap();
    let (response, _) = asked.await.unwrap().unwrap();
    assert_ne!(response, 0);
    wait_until(|| async { prompts(&isle).await["showing"].is_null() }).await;
}
