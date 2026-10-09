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

const RIGHT: u32 = 106;
const SPACE: u32 = 57;

async fn call(
    conn: &zbus::Connection,
    method: &str,
    body: &(impl serde::Serialize + zvariant::DynamicType),
) -> (u32, HashMap<String, OwnedValue>) {
    let reply = conn
        .call_method(
            Some(BACKEND),
            PATH,
            Some("org.freedesktop.impl.portal.ScreenCast"),
            method,
            body,
        )
        .await
        .unwrap();
    reply.body().deserialize().unwrap()
}

async fn share(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.share().await.unwrap()).unwrap()
}

/// CreateSession and SelectSources for screens and windows, keeping the share.
async fn session(frontend: &zbus::Connection, n: u32, restore: Option<OwnedValue>) -> String {
    let session = format!("/org/freedesktop/portal/desktop/session/1_1/s{n}");
    let (r, _) = call(
        frontend,
        "CreateSession",
        &(
            ObjectPath::try_from(format!("/org/freedesktop/portal/desktop/request/1_1/c{n}"))
                .unwrap(),
            ObjectPath::try_from(session.as_str()).unwrap(),
            "",
            HashMap::<String, OwnedValue>::new(),
        ),
    )
    .await;
    assert_eq!(r, 0);
    let mut options: HashMap<String, OwnedValue> = HashMap::new();
    options.insert("types".into(), OwnedValue::from(3u32));
    options.insert("cursor_mode".into(), OwnedValue::from(1u32));
    options.insert("persist_mode".into(), OwnedValue::from(2u32));
    if let Some(restore) = restore {
        options.insert("restore_data".into(), restore);
    }
    let (r, _) = call(
        frontend,
        "SelectSources",
        &(
            ObjectPath::try_from(format!("/org/freedesktop/portal/desktop/request/1_1/s{n}"))
                .unwrap(),
            ObjectPath::try_from(session.as_str()).unwrap(),
            "",
            options,
        ),
    )
    .await;
    assert_eq!(r, 0);
    session
}

fn start(
    frontend: &zbus::Connection,
    session: &str,
    n: u32,
) -> tokio::task::JoinHandle<(u32, HashMap<String, OwnedValue>)> {
    let frontend = frontend.clone();
    let session = session.to_owned();
    tokio::spawn(async move {
        call(
            &frontend,
            "Start",
            &(
                ObjectPath::try_from(format!("/org/freedesktop/portal/desktop/request/1_1/t{n}"))
                    .unwrap(),
                ObjectPath::try_from(session.as_str()).unwrap(),
                "",
                "",
                HashMap::<String, OwnedValue>::new(),
            ),
        )
        .await
    })
}

/// The first stream's node from Start's results.
fn first_node(results: &HashMap<String, OwnedValue>) -> (u32, HashMap<String, OwnedValue>) {
    let streams: Vec<(u32, HashMap<String, OwnedValue>)> =
        results["streams"].try_clone().unwrap().try_into().unwrap();
    streams.into_iter().next().unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_share_chosen_in_the_picker_streams_comes_back_without_asking_and_stops() {
    let mut isle = Isle::start("portal-screencast").await;
    if !isle.start_audio().await {
        return;
    }
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    let frontend = frontend(&isle).await;

    // The picker opens on Screens; the keys choose the screen and share it.
    let s1 = session(&frontend, 1, None).await;
    let started = start(&frontend, &s1, 1);
    wait_until(|| async { share(&isle).await["picker"]["drawn"] == true }).await;
    let picker = share(&isle).await["picker"].clone();
    assert_eq!(picker["tabs"], serde_json::json!(["Screens", "Windows"]));
    assert_eq!(picker["remember"], true);
    wait_until(|| async { share(&isle).await["picker"]["previews"].as_u64() == Some(1) }).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    isle.keep_shot("share-picker").await;
    press(&isle, RIGHT).await;
    press(&isle, SPACE).await;
    wait_until(|| async {
        share(&isle).await["picker"]["chosen"] == serde_json::json!(["screen:HEADLESS-1"])
    })
    .await;
    press(&isle, RETURN).await;
    let (response, results) = started.await.unwrap();
    assert_eq!(response, 0);
    let (node, props) = first_node(&results);
    assert_eq!(
        u32::try_from(props["source_type"].try_clone().unwrap()).unwrap(),
        1
    );
    assert!(props.contains_key("pipewire-serial"));
    assert_eq!(
        u32::try_from(results["persist_mode"].try_clone().unwrap()).unwrap(),
        2
    );
    let restore = results["restore_data"].try_clone().unwrap();
    let consumer = isle.consumer(node);
    consumer.wait_for("format 1920x1080").await;
    wait_until(|| async { isle.island_output().await["privacy"] == 1 }).await;

    // The kept share comes back with no picker.
    let s2 = session(&frontend, 2, Some(restore)).await;
    let (response, results) = start(&frontend, &s2, 2).await.unwrap();
    assert_eq!(response, 0);
    assert!(share(&isle).await["picker"].is_null());
    first_node(&results);
    assert_eq!(share(&isle).await["sessions"].as_object().unwrap().len(), 2);

    // The frontend closing a session ends its streams; Stop in the island ends the other.
    frontend
        .call_method(
            Some(BACKEND),
            s1.as_str(),
            Some("org.freedesktop.impl.portal.Session"),
            "Close",
            &(),
        )
        .await
        .unwrap();
    wait_until(|| async { share(&isle).await["sessions"].as_object().unwrap().len() == 1 }).await;
    isle.open_panel(None).await;
    isle.test()
        .await
        .panel(&serde_json::json!({"activity": ["sharing", format!("stop:{s2}")]}).to_string())
        .await
        .unwrap();
    wait_until(|| async {
        share(&isle).await["sessions"]
            .as_object()
            .unwrap()
            .is_empty()
    })
    .await;
    let casts: Vec<serde_json::Value> =
        serde_json::from_str(&isle.test().await.casts().await.unwrap()).unwrap();
    assert!(casts.is_empty());
    wait_until(|| async { isle.island_output().await["privacy"] == 0 }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_in_the_picker_answers_cancel() {
    let isle = Isle::start("portal-cancel").await;
    let frontend = frontend(&isle).await;
    let s = session(&frontend, 1, None).await;
    let started = start(&frontend, &s, 1);
    wait_until(|| async { share(&isle).await["picker"]["drawn"] == true }).await;
    press(&isle, ESCAPE).await;
    let (response, _) = started.await.unwrap();
    assert_eq!(response, 1);
    assert!(share(&isle).await["picker"].is_null());
}

async fn shot(
    frontend: &zbus::Connection,
    n: u32,
    options: HashMap<String, OwnedValue>,
) -> (u32, HashMap<String, OwnedValue>) {
    let reply = frontend
        .call_method(
            Some(BACKEND),
            PATH,
            Some("org.freedesktop.impl.portal.Screenshot"),
            "Screenshot",
            &(
                ObjectPath::try_from(format!(
                    "/org/freedesktop/portal/desktop/request/1_1/shot{n}"
                ))
                .unwrap(),
                "",
                "",
                options,
            ),
        )
        .await
        .unwrap();
    reply.body().deserialize().unwrap()
}

/// The PNG a URI names, its size, and the file removed.
fn taken(results: &HashMap<String, OwnedValue>) -> (u32, u32) {
    let uri: String = results["uri"].try_clone().unwrap().try_into().unwrap();
    let path = uri.strip_prefix("file://").unwrap().to_owned();
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap()));
    let info = decoder.read_info().unwrap().info().clone();
    let _ = std::fs::remove_file(&path);
    (info.width, info.height)
}

#[tokio::test(flavor = "multi_thread")]
async fn screenshots_for_apps_with_and_without_asking() {
    let mut isle = Isle::start("portal-screenshot").await;
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Red").await;
    let frontend = frontend(&isle).await;

    // Not interactive: the screens, or the front window.
    let (response, results) = shot(&frontend, 1, HashMap::new()).await;
    assert_eq!(response, 0);
    assert_eq!(taken(&results), (1920, 1080));
    let mut options: HashMap<String, OwnedValue> = HashMap::new();
    options.insert("target".into(), OwnedValue::from(8u32));
    let (_, results) = shot(&frontend, 2, options).await;
    let (w, _) = taken(&results);
    assert!(w as i32 > info.width, "the window with its shadow: {w}");

    // Interactive: the toolbar names the app, and its action shares the shot.
    let mut options: HashMap<String, OwnedValue> = HashMap::new();
    options.insert("interactive".into(), OwnedValue::from(true));
    let asked = tokio::spawn({
        let frontend = frontend.clone();
        async move { shot(&frontend, 3, options).await }
    });
    let toolbar = || async {
        serde_json::from_str::<serde_json::Value>(&isle.test().await.toolbar().await.unwrap())
            .unwrap()
    };
    wait_until(|| async { toolbar().await["request"].is_string() }).await;
    press(&isle, RETURN).await;
    let (response, results) = asked.await.unwrap();
    assert_eq!(response, 0);
    assert_eq!(taken(&results), (1920, 1080));
    assert!(!isle.dir.join("home/Pictures/Screenshots").exists());
    let thumbnail: serde_json::Value =
        serde_json::from_str(&isle.test().await.thumbnail().await.unwrap()).unwrap();
    assert!(thumbnail.is_null());

    // Escape answers cancel.
    let asked = tokio::spawn({
        let frontend = frontend.clone();
        let mut options: HashMap<String, OwnedValue> = HashMap::new();
        options.insert("interactive".into(), OwnedValue::from(true));
        async move { shot(&frontend, 4, options).await }
    });
    wait_until(|| async { toolbar().await["request"].is_string() }).await;
    press(&isle, ESCAPE).await;
    assert_eq!(asked.await.unwrap().0, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn pick_color_answers_the_colour_under_the_loupe() {
    let mut isle = Isle::start("portal-pick").await;
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Red").await;
    let frontend = frontend(&isle).await;
    let test = isle.test().await;
    test.pointer_to(
        (info.x + info.width / 2) as f64,
        (info.y + info.height / 2) as f64,
    )
    .await
    .unwrap();
    let asked = tokio::spawn({
        let frontend = frontend.clone();
        async move {
            let reply = frontend
                .call_method(
                    Some(BACKEND),
                    PATH,
                    Some("org.freedesktop.impl.portal.Screenshot"),
                    "PickColor",
                    &(
                        ObjectPath::try_from("/org/freedesktop/portal/desktop/request/1_1/pick")
                            .unwrap(),
                        "",
                        "",
                        HashMap::<String, OwnedValue>::new(),
                    ),
                )
                .await
                .unwrap();
            reply
                .body()
                .deserialize::<(u32, HashMap<String, OwnedValue>)>()
                .unwrap()
        }
    });
    wait_until(|| async {
        let pick: serde_json::Value = serde_json::from_str(&test.pick().await.unwrap()).unwrap();
        pick["mode"] == "colour"
    })
    .await;
    test.button(0x110, true).await.unwrap();
    test.button(0x110, false).await.unwrap();
    let (response, results) = asked.await.unwrap();
    assert_eq!(response, 0);
    let colour: (f64, f64, f64) = results["color"].try_clone().unwrap().try_into().unwrap();
    assert_eq!(colour, (1.0, 0.0, 0.0));
    assert!(!window.printed("button 272"));
}
