//! A display change applied at once and held until kept: the deadline, Revert and the asking
//! client leaving the bus put back what was. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use std::{fs, time::Duration};

use isle_ipc::{DisplayInfo, displays::DisplaysProxy};

mod common;

use common::{Isle, wait_until};

/// A deadline short enough to wait out.
async fn start(name: &str) -> Isle {
    Isle::start_prepared(name, "wallpaper {\n    source \"#1c1c1f\"\n}\n", |dir| {
        fs::write(dir.join("config/tokens.kdl"), "version 1\ntiming { display-revert-ms 800 }\n").unwrap();
    })
    .await
}

fn identity(info: &DisplayInfo) -> String {
    format!("{} {} {}", info.make, info.model, info.serial)
}

async fn first(displays: &DisplaysProxy<'_>) -> DisplayInfo {
    displays.list().await.unwrap().into_iter().find(|d| d.name == "HEADLESS-1").unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_applies_at_once_and_goes_back_at_its_deadline() {
    let isle = start("display-deadline").await;
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    let before = first(&displays).await;
    let change = format!(r#"{{"{}": {{"scale": 2.0}}}}"#, identity(&before));
    displays.configure(&change).await.unwrap();
    assert_eq!(first(&displays).await.scale, 2.0);
    wait_until(|| async { first(&displays).await.scale == before.scale }).await;
    let file = fs::read_to_string(isle.dir.join("config/settings.kdl")).unwrap();
    assert!(!file.contains("scale"), "nothing written: {file}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_kept_change_is_written_and_stays() {
    let isle = start("display-keep").await;
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    let before = first(&displays).await;
    let change = format!(r#"{{"{}": {{"mode": "1280x720", "scale": 1.25}}}}"#, identity(&before));
    let id = displays.configure(&change).await.unwrap();
    wait_until(|| async { first(&displays).await.width == 1280 }).await;
    assert!(displays.keep(id).await.unwrap());
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let now = first(&displays).await;
    assert_eq!((now.width, now.scale), (1280, 1.25));
    let file = fs::read_to_string(isle.dir.join("config/settings.kdl")).unwrap();
    assert!(file.contains("mode \"1280x720\"") && file.contains("scale 1.25"), "{file}");
    assert!(!displays.keep(id).await.unwrap(), "kept once");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_change_goes_back_when_its_caller_leaves() {
    let isle = Isle::start("display-caller").await;
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    let before = first(&displays).await;
    let caller = zbus::connection::Builder::address(isle.bus_address().as_str())
        .unwrap()
        .build()
        .await
        .unwrap();
    let theirs = DisplaysProxy::new(&caller).await.unwrap();
    let change = format!(r#"{{"{}": {{"scale": 1.5}}}}"#, identity(&before));
    theirs.configure(&change).await.unwrap();
    assert_eq!(first(&displays).await.scale, 1.5);
    drop(theirs);
    drop(caller);
    wait_until(|| async { first(&displays).await.scale == before.scale }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_display_turns_off_and_back_and_the_last_one_cannot() {
    let isle = Isle::start("display-off").await;
    let test = isle.test().await;
    test.add_output("HEADLESS-2", "Isle", "Headless", "2", 1280, 720).await.unwrap();
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    let only = format!(r#"{{"{}": {{"enabled": false}}, "Isle Headless 2": {{"enabled": false}}}}"#, identity(&first(&displays).await));
    assert!(displays.configure(&only).await.is_err(), "not every display off");

    let id = displays.configure(r#"{"Isle Headless 2": {"enabled": false}}"#).await.unwrap();
    let listed = displays.list().await.unwrap();
    let second = listed.iter().find(|d| d.name == "HEADLESS-2").unwrap();
    assert!(!second.enabled, "listed while off: {listed:?}");
    assert_eq!(isle.windows_proxy().await.list().await.unwrap().len(), 0);
    assert!(displays.revert(id).await.unwrap());
    wait_until(|| async {
        displays.list().await.unwrap().iter().any(|d| d.name == "HEADLESS-2" && d.enabled)
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn identify_names_each_screen_and_goes() {
    let isle = start("displays-identify").await;
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    let shell = isle_ipc::shell::ShellProxy::new(&isle.bus).await.unwrap();
    let open = || async {
        shell
            .list()
            .await
            .unwrap()
            .into_iter()
            .any(|s| s.name == "identify" && s.open)
    };
    assert!(!open().await);
    displays.identify().await.unwrap();
    wait_until(|| async { open().await }).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    wait_until(|| async { !open().await }).await;
}
