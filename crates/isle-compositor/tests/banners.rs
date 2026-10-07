//! Banners: on screen fast, one per app, waiting behind Control Centre, the click and the
//! reply, and the stack as drawn.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::{Duration, Instant};

use common::{Client, Isle, wait_until};

async fn banners(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.banners().await.unwrap()).unwrap()
}

async fn cards(isle: &Isle) -> Vec<serde_json::Value> {
    banners(isle).await["cards"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

async fn notified(client: &Client) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(5);
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
        assert!(Instant::now() < deadline, "never notified");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_banner_is_on_screen_within_a_tenth_of_a_second() {
    let isle = Isle::start("banners-fast").await;
    let started = Instant::now();
    isle.bus
        .call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "Notify",
            &(
                "Test",
                0u32,
                "",
                "Fast",
                "",
                Vec::<String>::new(),
                std::collections::HashMap::<&str, zvariant::Value>::new(),
                -1i32,
            ),
        )
        .await
        .unwrap();
    loop {
        if banners(&isle).await["open"] == true {
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(2));
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    let took = started.elapsed();
    assert!(took < Duration::from_millis(100), "{took:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn one_banner_per_app_and_arrivals_wait_behind_control_centre() {
    let mut isle = Isle::start("banners-group").await;
    let app = isle.client("isle.test.Chat", "3366ff");
    app.wait_for("keyboard-enter").await;
    app.send("notify summary=One body=Hello_there");
    notified(&app).await;
    app.send("notify summary=Two body=Again");
    notified(&app).await;
    wait_until(|| async {
        let c = cards(&isle).await;
        c.len() == 1 && c[0]["count"] == 2 && c[0]["summary"] == "Two"
    })
    .await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    isle.keep_shot("banner").await;

    isle.open_panel(None).await;
    let other = isle.client("isle.test.Mail", "ff6633");
    other.wait_for("keyboard-enter").await;
    other.send("notify summary=Mail");
    notified(&other).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        cards(&isle).await.len(),
        1,
        "it waits while the panel is open"
    );
    isle.panel(serde_json::json!({ "page": null })).await;
    isle.chord(&[29, 66]).await;
    wait_until(|| async { cards(&isle).await.len() == 2 }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_click_runs_the_default_action_and_a_reply_reaches_the_app() {
    let mut isle = Isle::start("banners-act").await;
    let app = isle.client("isle.test.Chat", "3366ff");
    app.wait_for("keyboard-enter").await;
    app.send("notify summary=Sam actions=default:Open,inline-reply:Reply");
    let id = notified(&app).await;
    wait_until(|| async { cards(&isle).await.len() == 1 }).await;
    let test = isle.test().await;
    test.banner(&serde_json::json!({ "reply": id }).to_string())
        .await
        .unwrap();
    app.wait_for("keyboard-leave").await;
    test.banner(&serde_json::json!({ "send": [id, "On my way"] }).to_string())
        .await
        .unwrap();
    app.wait_for(&format!("replied {id} On my way")).await;
    app.wait_for("keyboard-enter").await;

    app.send("notify summary=Again actions=default:Open");
    let again = notified(&app).await;
    wait_until(|| async { cards(&isle).await.iter().any(|c| c["id"] == again) }).await;
    test.banner(&serde_json::json!({ "open": again }).to_string())
        .await
        .unwrap();
    app.wait_for(&format!("action {again} default")).await;
    wait_until(|| async { cards(&isle).await.is_empty() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_pointer_on_a_card_holds_it_and_a_click_runs_its_default() {
    let mut isle = Isle::start("banners-pointer").await;
    let chat = isle.client("isle.test.Chat", "3366ff");
    chat.wait_for("keyboard-enter").await;
    chat.send("notify summary=Hello actions=default:Open,later:Later");
    let id = notified(&chat).await;
    wait_until(|| async { banners(&isle).await["open"] == true }).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    let surface = isle_ipc::shell::ShellProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.name == "banners")
        .unwrap();
    let test = isle.test().await;
    // The card's middle: the surface's margin above it, then about half a card.
    test.pointer_to(
        (surface.x + surface.width / 2) as f64,
        (surface.y + 90) as f64,
    )
    .await
    .unwrap();
    wait_until(|| async { banners(&isle).await["hovered"] == true }).await;
    // Held past a temporary banner's life while the pointer rests on it.
    tokio::time::sleep(Duration::from_secs(7)).await;
    assert_eq!(
        cards(&isle).await.len(),
        1,
        "the card waits under the pointer"
    );
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    chat.wait_for(&format!("action {id} default")).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cards_button_takes_the_click_and_leaving_the_card_lets_it_go() {
    let mut isle = Isle::start("banners-button").await;
    let chat = isle.client("isle.test.Chat", "3366ff");
    chat.wait_for("keyboard-enter").await;
    chat.send("notify summary=Hello actions=open:Open,later:Later");
    let id = notified(&chat).await;
    wait_until(|| async { banners(&isle).await["open"] == true }).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    let s = isle_ipc::shell::ShellProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.name == "banners")
        .unwrap();
    let test = isle.test().await;
    test.pointer_to((s.x + s.width / 2) as f64, (s.y + 90) as f64)
        .await
        .unwrap();
    wait_until(|| async { banners(&isle).await["hovered"] == true }).await;
    // Onto Open, the first button under the card's text: still on the card.
    test.pointer_to((s.x + 148) as f64, (s.y + 132) as f64)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(banners(&isle).await["hovered"], true);
    isle.keep_shot("banner-actions").await;
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    chat.wait_for(&format!("action {id} open")).await;

    // An arrival waits while the pointer is on the stack.
    test.pointer_to(400.0, 600.0).await.unwrap();
    chat.send("notify summary=Again");
    notified(&chat).await;
    wait_until(|| async { cards(&isle).await.len() == 1 }).await;
    test.pointer_to((s.x + s.width / 2) as f64, (s.y + 90) as f64)
        .await
        .unwrap();
    wait_until(|| async { banners(&isle).await["hovered"] == true }).await;
    test.pointer_to(400.0, 600.0).await.unwrap();
    wait_until(|| async { banners(&isle).await["hovered"] == false }).await;
}
