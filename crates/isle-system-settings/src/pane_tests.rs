//! The panes headless, against the defaults and state a test gives them.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use freya::prelude::*;
use freya_testing::{
    TestingRunner,
    prelude::{MouseEventName, PlatformEvent},
};
use isle_app::{Desktop, Request, Write};
use isle_bus::Snapshot;
use isle_settings::Value;

use crate::{
    frame::{Frame, MemoryDir},
    services::{Act, Services},
};

type Setup = Rc<RefCell<Option<Box<dyn Fn(Desktop, Services)>>>>;

/// What a test reads back: the desktop and the services the window was given.
#[derive(Default, Clone)]
struct Handles(Rc<RefCell<Option<(Desktop, Services)>>>);

impl Handles {
    fn writes(&self) -> Vec<Write> {
        self.0.borrow().as_ref().unwrap().0.writes.peek().clone()
    }

    fn acts(&self) -> Vec<Act> {
        self.0.borrow().as_ref().unwrap().1.acts.peek().clone()
    }

    fn services(&self) -> Services {
        self.0.borrow().as_ref().unwrap().1
    }
}

fn defaults() -> BTreeMap<String, Value> {
    let mut all = BTreeMap::new();
    isle_settings::Section::flatten(&isle_settings::schema::Settings::default(), "", &mut all);
    all
}

fn settle(test: &mut TestingRunner) {
    for _ in 0..8 {
        test.sync_and_update();
    }
}

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, element| Label::try_downcast(element).map(|l| l.text.to_string()))
}

fn shows(test: &TestingRunner, text: &str) -> bool {
    labels(test).iter().any(|l| l == text)
}

fn click(test: &mut TestingRunner, caption: &str) {
    let at = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == caption)
                .map(|_| node.layout().area.center())
        })
        .unwrap_or_else(|| panic!("nothing reads {caption:?}: {:?}", labels(test)));
    test.click_cursor((f64::from(at.x), f64::from(at.y)));
    settle(test);
}

/// The last label reading `caption`: a menu's item over a row of the same name.
fn click_last(test: &mut TestingRunner, caption: &str) {
    let at = test
        .find_many(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == caption)
                .map(|_| node.layout().area.center())
        })
        .pop()
        .unwrap_or_else(|| panic!("nothing reads {caption:?}: {:?}", labels(test)));
    test.click_cursor((f64::from(at.x), f64::from(at.y)));
    settle(test);
}

/// The control at the trailing end of the row labelled `caption`: its switch or pop-up.
fn click_control(test: &mut TestingRunner, caption: &str) {
    let row = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == caption)
                .map(|_| node.layout().area)
        })
        .unwrap_or_else(|| panic!("no row {caption:?}: {:?}", labels(test)));
    // The form's right edge, inside the row's padding, on the label's line.
    let x = (900. - 220.) / 2. + 220. + 600. / 2. - 30.;
    test.click_cursor((x as f64, f64::from(row.center().y)));
    settle(test);
}

/// Settings opened at `uri` with the defaults, after `setup` gives its state.
fn open(uri: &str, setup: impl Fn(Desktop, Services) + 'static) -> (TestingRunner, Handles) {
    let dir = std::env::temp_dir().join(format!(
        "isle-settings-panes-{}-{}",
        uri.replace(['/', ':'], "-"),
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let handles = Handles::default();
    let setup: Setup = Rc::new(RefCell::new(Some(Box::new(setup))));
    let (uri, kept) = (uri.to_owned(), handles.clone());
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| MemoryDir(dir.clone()));
            let services = use_provide_context(Services::still);
            let desktop = isle_app::provide_for_tests(defaults());
            if let Some(act) = setup.borrow_mut().take() {
                act(desktop, services);
                let mut requests = desktop.requests;
                requests.write().push(Request::Open {
                    uris: vec![uri.clone()],
                    token: None,
                });
                *kept.0.borrow_mut() = Some((desktop, services));
            }
            rect().expanded().child(Frame)
        },
        (900., 2400.).into(),
        |_| {},
        1.,
    );
    settle(&mut test);
    (test, handles)
}

#[test]
fn every_pane_opens_on_its_header_with_the_defaults() {
    for pane in crate::panes::PANES {
        let (test, _) = open(&format!("isle-settings://{}", pane.id), |_, _| {});
        assert!(
            shows(&test, pane.subtitle),
            "{}: {:?}",
            pane.id,
            labels(&test)
        );
    }
}

#[test]
fn a_switch_writes_its_key_and_slow_keys_brings_its_delay() {
    let (mut test, handles) = open("isle-settings://accessibility", |_, _| {});
    assert!(!shows(&test, "Acceptance delay"));
    click_control(&mut test, "Slow Keys");
    assert!(
        handles
            .writes()
            .contains(&Write::Set("keyboard.slow-keys".into(), Value::Int(500)))
    );
    assert!(shows(&test, "Acceptance delay"), "{:?}", labels(&test));
    click_control(&mut test, "Reduce motion");
    assert!(handles.writes().contains(&Write::Set(
        "appearance.reduce-motion".into(),
        Value::Bool(true)
    )));
}

#[test]
fn a_spotlight_source_turned_off_is_listed() {
    let (mut test, handles) = open("isle-settings://spotlight", |_, _| {});
    click_control(&mut test, "Menu items");
    assert!(
        handles.writes().contains(&Write::Set(
            "spotlight.sources-off".into(),
            Value::List(vec!["menu-items".into()])
        )),
        "{:?}",
        handles.writes()
    );
}

fn wifi_state() -> isle_network::State {
    use isle_network::{Known, Link, Network, Wifi};
    let network = |ssid: &str, strength| Network {
        path: format!("/ap/{ssid}"),
        ssid: ssid.into(),
        strength,
        secured: true,
        known: None,
    };
    isle_network::State {
        wifi_enabled: true,
        wired: None,
        wifi: Some(Wifi {
            path: "/dev/wlan0".into(),
            link: Link::Connected,
            current: Some(network("Home", 80)),
            networks: vec![network("Home", 80), network("Café", 40)],
        }),
        vpns: Vec::new(),
        known: vec![Known {
            path: "/saved/home".into(),
            ssid: "Home".into(),
            auto_join: true,
        }],
    }
}

#[test]
fn wifi_lists_known_and_nearby_networks_and_joins_one() {
    let (mut test, handles) = open("isle-settings://wi-fi", |_, services| {
        let mut network = services.network;
        network.set(Snapshot {
            available: true,
            state: wifi_state(),
        });
    });
    assert!(shows(&test, "Known networks"));
    assert!(shows(&test, "Café"));
    assert!(
        handles.acts().contains(&Act::Scan("/dev/wlan0".into())),
        "{:?}",
        handles.acts()
    );
    click(&mut test, "Connect");
    assert!(handles.acts().contains(&Act::Join {
        device: "/dev/wlan0".into(),
        ssid: "Café".into()
    }));
}

#[test]
fn a_missing_adapter_is_the_panes_state() {
    let (test, _) = open("isle-settings://bluetooth", |_, services| {
        let mut bluetooth = services.bluetooth;
        bluetooth.set(Snapshot {
            available: true,
            state: Default::default(),
        });
    });
    assert!(shows(&test, "No Bluetooth adapter is connected."));
    let (test, _) = open("isle-settings://wi-fi", |_, _| {});
    assert!(shows(&test, "No Wi-Fi device is connected."));
}

fn display() -> isle_ipc::DisplayInfo {
    isle_ipc::DisplayInfo {
        name: "DP-1".into(),
        make: "Dell".into(),
        model: "U2723".into(),
        serial: "1".into(),
        width: 2560,
        height: 1440,
        refresh: 143912,
        scale: 1.,
        x: 0,
        y: 0,
        vrr_capability: "on-demand".into(),
        vrr_setting: "off".into(),
        vrr_on: false,
        enabled: true,
        transform: "normal".into(),
        modes: [
            (2560, 1440, 143912),
            (2560, 1440, 59951),
            (1920, 1080, 60000),
        ]
        .iter()
        .map(|(w, h, r)| isle_ipc::DisplayMode {
            width: *w,
            height: *h,
            refresh: *r,
        })
        .collect(),
        width_mm: 597,
        height_mm: 336,
        min_hz: 48,
        max_hz: 144,
        brightness: -1.,
    }
}

#[test]
fn displays_shows_the_selected_screens_rows_and_asks_to_keep_a_change() {
    let (mut test, handles) = open("isle-settings://displays", |_, services| {
        let mut displays = services.displays;
        displays.set(Snapshot {
            available: true,
            state: vec![display()],
        });
    });
    assert!(shows(&test, "2560 × 1440 (default)"), "{:?}", labels(&test));
    assert!(shows(&test, "144 Hz"));
    assert!(shows(&test, "This display supports 48–144 Hz."));
    assert!(shows(&test, "Brightness can't be set on this display."));

    click(&mut test, "144 Hz");
    click(&mut test, "60 Hz");
    let configured = handles.acts().into_iter().find_map(|a| match a {
        Act::Configure(json) => Some(json),
        _ => None,
    });
    assert_eq!(
        configured.as_deref(),
        Some(r#"{"Dell U2723 1":{"mode":"2560x1440@59.951"}}"#)
    );

    let mut change = handles.services().display_change;
    change.set(Some(7));
    settle(&mut test);
    assert!(
        shows(&test, "Keep these display settings?"),
        "{:?}",
        labels(&test)
    );
    click(&mut test, "Keep");
    assert!(handles.acts().contains(&Act::Keep(7)));
    assert!(!shows(&test, "Keep these display settings?"));
}

#[test]
fn a_scale_applies_at_once_without_asking() {
    let (mut test, handles) = open("isle-settings://displays", |_, services| {
        let mut displays = services.displays;
        displays.set(Snapshot {
            available: true,
            state: vec![display()],
        });
    });
    click(&mut test, "100% (default)");
    click(&mut test, "150%");
    assert!(handles.writes().contains(&Write::Set(
        "output[Dell U2723 1].scale".into(),
        Value::Float(1.5)
    )));
    assert!(!shows(&test, "Keep these display settings?"));
}

#[test]
fn sound_lists_devices_and_makes_one_the_default() {
    let (mut test, handles) = open("isle-settings://sound", |_, services| {
        let mut audio = services.audio;
        audio.set(Snapshot {
            available: true,
            state: Default::default(),
        });
    });
    assert!(shows(&test, "Alert sound"));
    click(&mut test, "Message");
    click(&mut test, "Bell");
    assert!(handles.writes().contains(&Write::Set(
        "notifications.alert-sound".into(),
        Value::Str("bell".into())
    )));
    assert!(handles.acts().contains(&Act::PlaySound("bell".into())));
}

#[test]
fn a_search_result_inside_a_sheet_opens_the_sheet() {
    let (test, _) = open("isle-settings://displays/warmth", |_, services| {
        let mut displays = services.displays;
        displays.set(Snapshot {
            available: true,
            state: vec![display()],
        });
    });
    assert!(shows(&test, "Warmth"), "{:?}", labels(&test));
    assert!(shows(&test, "Less warm"));
}

#[test]
fn control_centre_shows_each_module_with_its_size() {
    let (mut test, handles) = open("isle-settings://control-centre", |_, services| {
        let mut modules = services.modules;
        modules.set(Snapshot {
            available: true,
            state: vec![crate::services::Module {
                id: "dark-mode".into(),
                name: "Dark Mode".into(),
                glyph: "circle-half".into(),
                sizes: vec!["s".into(), "m".into()],
            }],
        });
    });
    assert!(shows(&test, "Dark Mode"));
    click(&mut test, "Don't show");
    click(&mut test, "Small");
    let modules = handles.writes().into_iter().rev().find_map(|w| match w {
        Write::Set(key, Value::List(list)) if key == "control-centre.modules" => Some(list),
        _ => None,
    });
    assert_eq!(
        modules.unwrap().last().map(String::as_str),
        Some("dark-mode:s")
    );
}

#[test]
fn an_apps_notifications_page_writes_its_entry() {
    let (mut test, handles) = open("isle-settings://notifications", |_, services| {
        let mut apps = services.notify_apps;
        apps.set(Snapshot {
            available: true,
            state: vec![("org.mozilla.firefox".into(), "Firefox".into())],
        });
    });
    assert!(
        shows(&test, "Temporary banners, sound, badges"),
        "{:?}",
        labels(&test)
    );
    click(&mut test, "Firefox");
    assert!(shows(&test, "How Firefox's notifications reach you."));
    click_control(&mut test, "Allow notifications");
    assert!(handles.writes().contains(&Write::Set(
        "notification-app[org.mozilla.firefox].allow".into(),
        Value::Bool(false)
    )));
}

#[test]
fn a_hot_corner_is_chosen_in_its_sheet() {
    let (mut test, handles) = open("isle-settings://desktop", |_, _| {});
    click(&mut test, "Hot Corners…");
    assert!(shows(&test, "Apps"), "{:?}", labels(&test));
    click_last(&mut test, "Apps");
    click_last(&mut test, "Lock Screen");
    let writes = handles.writes();
    assert!(
        writes.contains(&Write::Set(
            "hot-corners.bottom-right".into(),
            Value::Str("lock-screen".into())
        )),
        "{writes:?}"
    );
    assert!(writes.contains(&Write::Set(
        "hot-corners.bottom-right-modifiers".into(),
        Value::Str(String::new())
    )));
}

#[test]
fn a_changed_row_resets_to_its_default_from_its_context_menu() {
    let (mut test, handles) = open("isle-settings://accessibility", |desktop, _| {
        desktop.set("appearance.reduce-motion", Value::Bool(true));
    });
    let row = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == "Reduce motion")
                .map(|_| node.layout().area.center())
        })
        .unwrap();
    let cursor = (f64::from(row.x), f64::from(row.y));
    for name in [MouseEventName::MouseDown, MouseEventName::MouseUp] {
        test.send_event(PlatformEvent::Mouse {
            name,
            cursor: cursor.into(),
            button: Some(MouseButton::Right),
        });
    }
    settle(&mut test);
    click(&mut test, "Reset to Default");
    assert!(
        handles
            .writes()
            .contains(&Write::Reset("appearance.reduce-motion".into())),
        "{:?}",
        handles.writes()
    );

    // An unchanged row offers nothing.
    let (mut test, _) = open("isle-settings://accessibility", |_, _| {});
    for name in [MouseEventName::MouseDown, MouseEventName::MouseUp] {
        test.send_event(PlatformEvent::Mouse {
            name,
            cursor: cursor.into(),
            button: Some(MouseButton::Right),
        });
    }
    settle(&mut test);
    assert!(!shows(&test, "Reset to Default"));
}

#[test]
fn identify_asks_the_compositor() {
    let (mut test, handles) = open("isle-settings://displays", |_, services| {
        let mut displays = services.displays;
        displays.set(Snapshot {
            available: true,
            state: vec![display()],
        });
    });
    click(&mut test, "Identify");
    assert!(handles.acts().contains(&Act::Identify));
}
