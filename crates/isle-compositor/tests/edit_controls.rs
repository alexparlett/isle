//! Control Centre completed: the design's default arrangement, edit mode and its gallery
//! writing the arrangement, Accessibility's switches, and a keyboard-only walk.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, wait_until};
use isle_ipc::settings::SettingsProxy;
use serde_json::Value;

const ESC: u32 = 1;
const TAB: u32 = 15;
const ENTER: u32 = 28;
const HOME: u32 = 102;

async fn panel(isle: &Isle) -> Value {
    isle.island_output().await["panel"].clone()
}

async fn ids(isle: &Isle) -> Vec<String> {
    panel(isle).await["modules"]
        .as_array()
        .map(|m| {
            m.iter()
                .map(|m| m["id"].as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

async fn arrangement(settings: &SettingsProxy<'_>) -> Vec<String> {
    let value = settings.get("control-centre.modules").await.unwrap();
    Vec::<String>::try_from(value).unwrap_or_default()
}

async fn set_modules(isle: &Isle, modules: &[&str]) {
    let settings = SettingsProxy::new(&isle.bus).await.unwrap();
    settings
        .set(
            "control-centre.modules",
            &zvariant::Value::from(modules.to_vec()),
        )
        .await
        .unwrap();
}

async fn tap(isle: &Isle, key: u32) {
    isle.chord(&[key]).await;
    tokio::time::sleep(Duration::from_millis(60)).await;
}

/// The label of the island's focused node.
async fn focused(isle: &Isle) -> String {
    let shell = isle_ipc::shell::ShellProxy::new(&isle.bus).await.unwrap();
    let tree: Value = serde_json::from_str(&shell.tree("island").await.unwrap()).unwrap();
    tree["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["focused"] == true)
        .and_then(|n| n["label"].as_str())
        .unwrap_or_default()
        .to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_mode_adds_resizes_moves_and_removes_modules_and_writes_them() {
    let isle = Isle::start("edit-controls").await;
    let settings = SettingsProxy::new(&isle.bus).await.unwrap();
    let default = arrangement(&settings).await;
    assert!(default.contains(&"night-light:s".to_owned()));
    assert!(
        !default.iter().any(|m| m.starts_with("dark-mode")),
        "{default:?}"
    );

    isle.open_panel(None).await;
    isle.panel_settled().await;
    isle.keep_shot("control-centre-default").await;
    assert!(ids(&isle).await.contains(&"focus".to_owned()));

    isle.panel(serde_json::json!({ "edit": true })).await;
    wait_until(|| async { !panel(&isle).await["edit"].is_null() }).await;
    let gallery = panel(&isle).await["edit"]["gallery"].clone();
    let find = |id: &str| {
        gallery
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["id"] == id)
            .cloned()
            .unwrap_or_default()
    };
    assert_eq!(find("accessibility")["why"], Value::Null);
    assert_eq!(find("accessibility")["group"], "Accessibility");
    assert_eq!(find("phone")["why"], "No phone provider is set up.");
    assert!(find("night-light").is_null(), "in the grid, so not offered");
    isle.panel_settled().await;
    isle.keep_shot("control-centre-edit").await;

    isle.panel(serde_json::json!({ "edit-add": "accessibility" }))
        .await;
    wait_until(|| async {
        arrangement(&settings)
            .await
            .contains(&"accessibility:m".into())
    })
    .await;
    wait_until(|| async { ids(&isle).await.contains(&"accessibility".to_owned()) }).await;

    isle.panel(serde_json::json!({ "edit-size": ["accessibility", "s"] }))
        .await;
    wait_until(|| async {
        arrangement(&settings)
            .await
            .contains(&"accessibility:s".into())
    })
    .await;
    isle.panel(serde_json::json!({ "edit-size": ["focus", "w"] }))
        .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        arrangement(&settings).await.contains(&"focus:m".into()),
        "a size the module does not come in is refused"
    );

    isle.panel(serde_json::json!({ "edit-move": ["accessibility", 0] }))
        .await;
    wait_until(|| async { ids(&isle).await.first().map(String::as_str) == Some("accessibility") })
        .await;
    // Before Now Playing, the first shown; absent modules keep their places.
    let order = arrangement(&settings).await;
    let at = |id: &str| order.iter().position(|m| m.starts_with(id)).unwrap();
    assert!(at("accessibility") < at("now-playing"), "{order:?}");

    // Escape closes a module's menu first, then leaves edit mode; the panel stays.
    isle.panel(serde_json::json!({ "edit-menu": "focus" }))
        .await;
    wait_until(|| async { panel(&isle).await["edit"]["menu"] == "focus" }).await;
    isle.panel_settled().await;
    isle.keep_shot("control-centre-edit-menu").await;
    tap(&isle, ESC).await;
    wait_until(|| async { panel(&isle).await["edit"]["menu"].is_null() }).await;
    assert!(!panel(&isle).await["edit"].is_null());
    tap(&isle, ESC).await;
    wait_until(|| async { panel(&isle).await["edit"].is_null() }).await;
    assert!(!panel(&isle).await.is_null());

    isle.panel(serde_json::json!({ "edit": true })).await;
    isle.panel(serde_json::json!({ "edit-remove": "focus" }))
        .await;
    wait_until(|| async { !ids(&isle).await.contains(&"focus".to_owned()) }).await;
    assert!(
        !arrangement(&settings)
            .await
            .iter()
            .any(|m| m.starts_with("focus"))
    );
    wait_until(|| async {
        panel(&isle).await["edit"]["gallery"]
            .as_array()
            .is_some_and(|g| g.iter().any(|g| g["id"] == "focus"))
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn accessibility_switches_write_their_settings() {
    let isle = Isle::start("accessibility-module").await;
    set_modules(&isle, &["accessibility:m"]).await;
    isle.open_panel(None).await;
    wait_until(|| async { ids(&isle).await == ["accessibility"] }).await;
    isle.panel(serde_json::json!({ "page": "accessibility" }))
        .await;
    wait_until(|| async { isle.rows().await.len() == 3 }).await;
    isle.panel_settled().await;
    isle.keep_shot("accessibility-page").await;
    isle.panel(serde_json::json!({ "switch": ["accessibility", "reduce-motion", true] }))
        .await;
    let settings = SettingsProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async {
        bool::try_from(settings.get("appearance.reduce-motion").await.unwrap()).unwrap_or(false)
    })
    .await;
    wait_until(|| async { isle.module("accessibility").await["state"] == "1 setting on" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_keyboard_alone_reaches_every_module_page_row_and_tray_item() {
    let mut isle = Isle::start("control-centre-keys").await;
    set_modules(
        &isle,
        &["now-playing:l", "focus:m", "accessibility:m", "dark-mode:s"],
    )
    .await;
    let app = isle.client("isle.test.Tray", "3366ff");
    app.wait_for("keyboard-enter").await;
    app.send("tray");
    app.wait_for("tray registered").await;

    isle.open_panel(None).await;
    wait_until(|| async { ids(&isle).await.len() == 4 }).await;
    wait_until(|| async { !panel(&isle).await["tray"].as_array().unwrap().is_empty() }).await;
    isle.panel_settled().await;

    // Pinned, the keyboard starts on the first module; the arrows walk the grid.
    wait_until(|| async { focused(&isle).await.starts_with("Now Playing") }).await;
    let mut seen = vec![focused(&isle).await];
    for _ in 0..3 {
        tap(&isle, common::keys::RIGHT).await;
        seen.push(focused(&isle).await);
    }
    for name in ["Now Playing", "Focus", "Accessibility", "Dark Mode"] {
        assert!(
            seen.iter().any(|l| l.starts_with(name)),
            "{name} in {seen:?}"
        );
    }

    // Tab leaves the grid as one stop for the tray, then the footer.
    tap(&isle, TAB).await;
    assert_eq!(focused(&isle).await, "Test Tray");
    tap(&isle, TAB).await;
    assert_eq!(focused(&isle).await, "Edit controls");

    // Back to the grid, to Accessibility, and Return opens its page.
    isle.chord(&[common::keys::SHIFT, TAB]).await;
    isle.chord(&[common::keys::SHIFT, TAB]).await;
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert!(
        focused(&isle).await.starts_with("Dark Mode"),
        "back where it left"
    );
    tap(&isle, HOME).await;
    assert!(focused(&isle).await.starts_with("Now Playing"));
    tap(&isle, common::keys::RIGHT).await;
    tap(&isle, common::keys::RIGHT).await;
    assert!(focused(&isle).await.starts_with("Accessibility"));
    tap(&isle, ENTER).await;
    wait_until(|| async { panel(&isle).await["page"] == "accessibility" }).await;
    isle.panel_settled().await;
    let mut labels = Vec::new();
    for _ in 0..6 {
        tap(&isle, TAB).await;
        labels.push(focused(&isle).await);
    }
    for row in ["Increase contrast", "Reduce motion", "Reduce transparency"] {
        assert!(
            labels.iter().any(|l| l.contains(row)),
            "{row} in {labels:?}"
        );
    }
    tap(&isle, ESC).await;
    wait_until(|| async { panel(&isle).await["page"].is_null() }).await;

    // E on the grid is edit mode; Escape leaves it.
    // Back on the grid, the keyboard is on the module whose page it left.
    wait_until(|| async { focused(&isle).await.starts_with("Accessibility") }).await;
    isle.chord(&[18]).await;
    wait_until(|| async { !panel(&isle).await["edit"].is_null() }).await;
    tap(&isle, ESC).await;
    wait_until(|| async { panel(&isle).await["edit"].is_null() }).await;
}

/// The default arrangement with every source present, and each module's page, kept as
/// screenshots under `ISLE_SHOT_DIR` to compare with `p1/surfaces-b.html`.
#[tokio::test(flavor = "multi_thread")]
async fn the_default_arrangement_and_every_page_with_their_sources() {
    use isle_fakes::{bluez, logind::Logind, mpris::Mpris, network, upower};
    let mut isle = Isle::start("control-centre-pages").await;
    let system = isle.system_address();
    let _nm = network::NetworkManager::start(
        &system,
        vec![
            network::Ap {
                ssid: "Home".into(),
                strength: 80,
                secured: true,
            },
            network::Ap {
                ssid: "Cafe".into(),
                strength: 50,
                secured: false,
            },
        ],
        vec![
            network::Saved::Wifi("Home".into()),
            network::Saved::Vpn("Work".into()),
        ],
    )
    .await
    .unwrap();
    let _bluez = bluez::BlueZ::start(
        &system,
        vec![bluez::Device {
            id: "00_11_22_33_44_55".into(),
            name: "WH-1000XM6".into(),
            icon: "audio-headset".into(),
            paired: true,
            connected: true,
            battery: Some(70),
        }],
    )
    .await
    .unwrap();
    let _upower = upower::UPower::start(
        &system,
        vec![upower::Device {
            id: "mouse".into(),
            kind: 5,
            power_supply: false,
            model: "MX Master 3S".into(),
            percentage: 64.0,
            state: 2,
        }],
    )
    .await
    .unwrap();
    let _profiles = upower::PowerProfiles::start(&system).await.unwrap();
    let _logind = Logind::start(&system).await.unwrap();
    isle.test()
        .await
        .fake_screen("HEADLESS-1", 60, 100)
        .await
        .unwrap();
    let _player = Mpris::start(&isle.bus_address(), "test", true)
        .await
        .unwrap();
    let audio = isle.start_audio().await;

    isle.open_panel(None).await;
    let wanted = [
        "wifi",
        "now-playing",
        "bluetooth",
        "focus",
        "vpn",
        "display",
        "energy-mode",
        "batteries",
    ];
    wait_until(|| async {
        let ids = ids(&isle).await;
        wanted.iter().all(|w| ids.iter().any(|i| i == w))
    })
    .await;
    if audio {
        wait_until(|| async { ids(&isle).await.contains(&"sound".to_owned()) }).await;
    }
    isle.panel_settled().await;
    isle.keep_shot("page-grid").await;
    for page in ids(&isle).await {
        isle.panel(serde_json::json!({ "page": page })).await;
        wait_until(|| async { panel(&isle).await["page"] == page.as_str() }).await;
        isle.panel_settled().await;
        isle.keep_shot(&format!("page-{page}")).await;
        isle.panel(serde_json::json!({ "page": null })).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_reads_the_catalogue_and_opens_edit_mode() {
    let isle = Isle::start("edit-from-settings").await;
    let island = isle_ipc::island::IslandProxy::new(&isle.bus).await.unwrap();
    let modules = island.modules().await.unwrap();
    let wifi = modules.iter().find(|m| m.0 == "wifi").expect("Wi-Fi in the catalogue");
    assert_eq!((wifi.4.clone(), wifi.5.as_str()), (vec!["s".to_owned(), "m".to_owned()], "wi-fi"));
    island.edit_controls().await.unwrap();
    wait_until(|| async { !panel(&isle).await["edit"].is_null() }).await;
}
