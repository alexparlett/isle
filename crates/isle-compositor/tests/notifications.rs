//! The notification server: what it says it can do, ids and replacement, what is kept, the
//! user's close and actions with their activation token, and senders leaving.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Client, Isle, wait_until};
use isle_ipc::notifications::NotificationsProxy;

async fn list(isle: &Isle) -> Vec<serde_json::Value> {
    let text = NotificationsProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value["entries"].as_array().cloned().unwrap_or_default()
}

/// The id a client's `notified N` line gave.
async fn notified(client: &Client) -> u32 {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let found = client
            .lines
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find_map(|l| l.strip_prefix("notified ").map(|n| n.parse().unwrap()));
        if let Some(id) = found {
            client.clear();
            return id;
        }
        assert!(std::time::Instant::now() < deadline, "never notified");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_server_says_what_it_is_and_what_it_can_do() {
    let isle = Isle::start("notify-info").await;
    let call = |method: &'static str| {
        let bus = isle.bus.clone();
        async move {
            bus.call_method(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                Some("org.freedesktop.Notifications"),
                method,
                &(),
            )
            .await
            .unwrap()
        }
    };
    let caps: Vec<String> = call("GetCapabilities").await.body().deserialize().unwrap();
    for cap in [
        "actions",
        "body",
        "body-markup",
        "persistence",
        "inline-reply",
        "sound",
    ] {
        assert!(caps.iter().any(|c| c == cap), "{cap} missing from {caps:?}");
    }
    let info: (String, String, String, String) = call("GetServerInformation")
        .await
        .body()
        .deserialize()
        .unwrap();
    assert_eq!((info.0.as_str(), info.3.as_str()), ("Isle", "1.3"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_replacement_keeps_its_id_and_what_is_kept_follows_the_rules() {
    let mut isle = Isle::start("notify-keep").await;
    let client = isle.client("isle.test.Notifier", "3366ff");
    client.wait_for("keyboard-enter").await;
    client.send("notify summary=First");
    let id = notified(&client).await;
    client.send(&format!("notify summary=Second replaces={id}"));
    assert_eq!(notified(&client).await, id);
    wait_until(|| async {
        let entries = list(&isle).await;
        entries.len() == 1 && entries[0]["summary"] == "Second"
    })
    .await;
    assert_eq!(list(&isle).await[0]["fate"], "banner");

    client.send("notify summary=Quiet urgency=0");
    let quiet = notified(&client).await;
    client.send("notify summary=Passing transient");
    let passing = notified(&client).await;
    wait_until(|| async { list(&isle).await.len() == 3 }).await;
    let entries = list(&isle).await;
    let fate = |id: u32| entries.iter().find(|e| e["id"] == id).unwrap()["fate"].clone();
    assert_eq!(fate(quiet), "quiet");
    assert_eq!(fate(passing), "transient");

    // Its sender leaving turns the rest into records and drops the transient one.
    client.send("quit");
    wait_until(|| async {
        let entries = list(&isle).await;
        entries.len() == 2 && entries.iter().all(|e| e["live"] == false)
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    let saved = std::fs::read_to_string(isle.dir.join("state/isle/notifications.json")).unwrap();
    assert!(
        saved.contains("Second") && !saved.contains("Passing"),
        "{saved}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn every_app_that_has_posted_is_listed_once_after_its_notifications_go() {
    let mut isle = Isle::start("notify-apps").await;
    let client = isle.client("isle.test.Notifier", "3366ff");
    client.wait_for("keyboard-enter").await;
    client.send("notify summary=First");
    notified(&client).await;
    client.send("notify summary=Second");
    notified(&client).await;
    let notifications = isle_ipc::notifications::NotificationsProxy::new(&isle.bus).await.unwrap();
    notifications.clear().await.unwrap();
    let apps = notifications.apps().await.unwrap();
    assert_eq!(apps.len(), 1, "{apps:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn closing_and_acting_are_signalled_and_the_token_brings_the_app_forward() {
    let mut isle = Isle::start("notify-act").await;
    let app = isle.client("isle.test.Mail", "3366ff");
    app.wait_for("keyboard-enter").await;
    let other = isle.client("isle.test.Other", "ff6633");
    other.wait_for("keyboard-enter").await;
    app.wait_for("keyboard-leave").await;

    app.send("notify summary=New_mail actions=default:Open,archive:Archive");
    let id = notified(&app).await;
    let proxy = NotificationsProxy::new(&isle.bus).await.unwrap();
    assert!(proxy.invoke(id, "default").await.unwrap());
    let token = loop {
        let found = app
            .lines
            .lock()
            .unwrap()
            .iter()
            .find_map(|l| l.strip_prefix(&format!("token {id} ")).map(str::to_owned));
        if let Some(t) = found {
            break t;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    app.wait_for(&format!("action {id} default")).await;
    app.wait_for(&format!("closed {id} 2")).await;
    app.clear();
    app.send(&format!("use-token {token}"));
    app.wait_for("keyboard-enter").await;

    // The app's own close leaves a record of what was missed.
    app.send("notify summary=Later");
    let later = notified(&app).await;
    app.send(&format!("close-notification {later}"));
    app.wait_for(&format!("closed {later} 3")).await;
    wait_until(|| async {
        list(&isle)
            .await
            .iter()
            .any(|e| e["id"] == later && e["live"] == false)
    })
    .await;
    // The user's close takes the record too.
    assert!(proxy.close(later).await.unwrap());
    assert!(list(&isle).await.iter().all(|e| e["id"] != later));
}
