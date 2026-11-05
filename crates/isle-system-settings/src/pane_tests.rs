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

/// Clicks the control of the row labelled `caption` whose description starts `described`:
/// a described row centres its control on the label and description together.
fn click_described(test: &mut TestingRunner, caption: &str, described: &str) {
    let area = |test: &TestingRunner, wanted: &dyn Fn(&str) -> bool| {
        test.find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| wanted(&label.text))
                .map(|_| node.layout().area)
        })
        .unwrap_or_else(|| panic!("no label for {caption:?}: {:?}", labels(test)))
    };
    let top = area(test, &|t| t == caption).min_y();
    let bottom = area(test, &|t| t.starts_with(described)).max_y();
    let x = (900. - 220.) / 2. + 220. + 600. / 2. - 30.;
    test.click_cursor((x as f64, f64::from((top + bottom) / 2.)));
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

#[test]
fn advanced_shows_the_tokens_file_and_opens_it_in_the_editor() {
    let status = |ok: bool| isle_ipc::SettingsStatus {
        ok,
        file: "/home/a/.config/isle/tokens.kdl".into(),
        line: if ok { 0 } else { 3 },
        column: 0,
        message: if ok { String::new() } else { "unknown node `spce`".into() },
    };
    let (mut test, handles) = open("isle-settings://appearance", move |desktop, _| {
        let (mut st, mut text) = (desktop.tokens_status, desktop.tokens_text);
        st.set(Some(status(true)));
        text.set(String::new());
    });
    click(&mut test, "Advanced…");
    assert!(shows(&test, "The shipped tokens are in use."), "{:?}", labels(&test));
    click(&mut test, "Create in Editor…");
    assert!(handles.acts().contains(&Act::EditTokens {
        path: "/home/a/.config/isle/tokens.kdl".into(),
        shipped: false,
    }));
    click(&mut test, "Show Shipped Tokens");
    assert!(handles.acts().contains(&Act::EditTokens {
        path: String::new(),
        shipped: true,
    }));

    let (mut test, _) = open("isle-settings://appearance", move |desktop, _| {
        let (mut st, mut text) = (desktop.tokens_status, desktop.tokens_text);
        st.set(Some(status(true)));
        text.set("space { m 16 }\ncolour { dark { text \"#ffffff\" }; light { text \"#000000\" } }\n".into());
    });
    click(&mut test, "Advanced…");
    assert!(shows(&test, "Changes 2 tokens."), "{:?}", labels(&test));

    let (mut test, _) = open("isle-settings://appearance", move |desktop, _| {
        let mut st = desktop.tokens_status;
        st.set(Some(status(false)));
    });
    click(&mut test, "Advanced…");
    assert!(shows(&test, "Not loaded"), "{:?}", labels(&test));
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
            networks: vec![network("Office", 60), network("Café", 40)],
        }),
        vpns: Vec::new(),
        known: vec![
            Known {
                path: "/saved/home".into(),
                paths: vec!["/saved/home".into()],
                ssid: "Home".into(),
                auto_join: true,
            },
            Known {
                path: "/saved/office".into(),
                paths: vec!["/saved/office".into(), "/saved/office-2".into()],
                ssid: "Office".into(),
                auto_join: true,
            },
        ],
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
    // Home is the joined row alone; Office, known and in range, is listed once, under Known.
    let count = |name: &str| labels(&test).iter().filter(|l| *l == name).count();
    assert_eq!((count("Home"), count("Office")), (1, 1), "{:?}", labels(&test));
    assert!(
        handles.acts().contains(&Act::Scan("/dev/wlan0".into())),
        "{:?}",
        handles.acts()
    );
    click(&mut test, "Connect");
    assert!(
        handles.acts().iter().any(|a| matches!(a, Act::Join { device, network }
            if device == "/dev/wlan0" && network.ssid == "Office"
                && network.known.as_deref() == Some("/saved/office"))),
        "{:?}",
        handles.acts()
    );
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
        hdr_peak: 0.,
        hdr: false,
        sdr_white: 203.,
        bit_depth: 0,
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
        shows(&test, "Do you want to keep these display settings?"),
        "{:?}",
        labels(&test)
    );
    click(&mut test, "Keep");
    assert!(handles.acts().contains(&Act::Keep(7)));
    assert!(!shows(&test, "Do you want to keep these display settings?"));
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
    assert!(!shows(&test, "Do you want to keep these display settings?"));
}

#[test]
fn hdr_shows_for_a_screen_that_takes_it_and_asks_to_keep_the_change() {
    let (mut test, handles) = open("isle-settings://displays", |_, services| {
        let mut displays = services.displays;
        displays.set(Snapshot {
            available: true,
            state: vec![display()],
        });
    });
    assert!(!shows(&test, "High Dynamic Range"));

    let mut displays = handles.services().displays;
    displays.set(Snapshot {
        available: true,
        state: vec![isle_ipc::DisplayInfo { hdr_peak: 302., ..display() }],
    });
    settle(&mut test);
    assert!(shows(&test, "High Dynamic Range"), "{:?}", labels(&test));
    assert!(!shows(&test, "SDR Brightness"));
    click_described(&mut test, "High Dynamic Range", "Shows HDR games");
    let configured = handles.acts().into_iter().find_map(|a| match a {
        Act::Configure(json) => Some(json),
        _ => None,
    });
    assert_eq!(configured.as_deref(), Some(r#"{"Dell U2723 1":{"hdr":true}}"#));

    displays.set(Snapshot {
        available: true,
        state: vec![isle_ipc::DisplayInfo { hdr_peak: 302., hdr: true, bit_depth: 8, ..display() }],
    });
    settle(&mut test);
    assert!(shows(&test, "SDR Brightness"), "{:?}", labels(&test));
    assert!(shows(&test, "SDR Colour"), "{:?}", labels(&test));
    assert!(
        labels(&test).iter().any(|l| l.ends_with("Running at 8-bit.")),
        "{:?}",
        labels(&test)
    );
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
            modifiers: Modifiers::empty(),
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
            modifiers: Modifiers::empty(),
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

fn centre_of(test: &TestingRunner, caption: &str) -> (f64, f64) {
    let at = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == caption)
                .map(|_| node.layout().area.center())
        })
        .unwrap_or_else(|| panic!("nothing reads {caption:?}"));
    (f64::from(at.x), f64::from(at.y))
}

#[test]
fn a_dragged_slider_sends_its_last_value_once() {
    let (mut test, handles) = open("isle-settings://appearance", |_, _| {});
    let (clear, y) = centre_of(&test, "Clear");
    let (tinted, _) = centre_of(&test, "Tinted");
    // Along the track between the two words, from its start to its end.
    let (from, to) = (clear + 30., tinted - 30.);
    test.press_cursor((from, y));
    for i in 0..=40 {
        test.move_cursor((from + (to - from) * f64::from(i) / 40., y));
    }
    test.release_cursor((to, y));
    settle(&mut test);
    let glass: Vec<Write> = handles
        .writes()
        .into_iter()
        .filter(|w| w.key() == "appearance.glass")
        .collect();
    assert_eq!(glass.len(), 1, "{glass:?}");
    assert!(matches!(&glass[0], Write::Set(_, Value::Float(v)) if *v > 0.9), "{glass:?}");
}

#[test]
fn stage_manager_turns_on_for_the_desktop_with_focus() {
    let (mut test, handles) = open("isle-settings://desktop", |_, services| {
        let mut desktops = services.desktops;
        let desktop = |index, focus| isle_ipc::DesktopInfo {
            output: "DP-4".into(),
            index,
            current: focus,
            kind: "desktop".into(),
            window: 0,
            windows: 1,
            stage: false,
            focus,
        };
        desktops.set(Snapshot {
            available: true,
            state: vec![desktop(1, false), desktop(2, true)],
        });
    });
    // The group's heading reads the same; the switch's row is the last.
    let row = test
        .find_many(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == "Stage Manager")
                .map(|_| node.layout().area)
        })
        .pop()
        .unwrap();
    let x = (900. - 220.) / 2. + 220. + 600. / 2. - 30.;
    test.click_cursor((x as f64, f64::from(row.center().y)));
    settle(&mut test);
    assert!(handles.acts().contains(&Act::SetStage {
        output: "DP-4".into(),
        index: 2,
        on: true,
    }));
}

fn login_item(id: &str, name: &str, on: bool) -> isle_apps::autostart::LoginItem {
    isle_apps::autostart::LoginItem {
        id: id.into(),
        name: name.into(),
        icon: None,
        source: isle_apps::autostart::Source::Entry,
        on,
        locked: false,
        only_in: Vec::new(),
    }
}

/// The − under a list whose last row reads `last`.
fn click_list_remove(test: &mut TestingRunner, last: &str) {
    let row = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == last)
                .map(|_| node.layout().area)
        })
        .unwrap_or_else(|| panic!("no row {last:?}: {:?}", labels(test)));
    let left = (900. - 220.) / 2. + 220. - 600. / 2.;
    test.click_cursor((left as f64 + 28. + 14., f64::from(row.center().y) + 34.));
    settle(test);
}

#[test]
fn login_items_show_both_groups_and_act_on_them() {
    use isle_apps::autostart::{LoginItems, Source};
    let (mut test, handles) = open("isle-settings://general/open-at-login", |_, services| {
        let mut items = services.login_items;
        let unit = |id: &str, name: &str, locked: bool| isle_apps::autostart::LoginItem {
            source: Source::Unit,
            locked,
            ..login_item(id, name, true)
        };
        items.set(Snapshot {
            available: true,
            state: LoginItems {
                open: vec![login_item("org.app.Mail", "Mail", true)],
                background: vec![
                    login_item("gnome-keyring", "Keyring", true),
                    isle_apps::autostart::LoginItem {
                        only_in: vec!["GNOME".into()],
                        ..login_item("gnomey", "Gnomey", true)
                    },
                    unit("isle-dev-sync.service", "Isle dev sync", true),
                    unit("xremap.service", "xremap", false),
                ],
            },
        });
        let mut apps = services.apps;
        apps.set(Snapshot {
            available: true,
            state: vec![crate::services::App {
                id: "org.app.Chat".into(),
                name: "Chat".into(),
                icon: None,
                listed: true,
                game: false,
            }],
        });
    });
    for text in ["Open at Login", "Mail", "Allow in the Background", "Starts only in GNOME."] {
        assert!(shows(&test, text), "{text}: {:?}", labels(&test));
    }
    click_control(&mut test, "Keyring");
    click_control(&mut test, "xremap");
    click_control(&mut test, "Isle dev sync");
    assert_eq!(
        handles.acts(),
        [
            Act::LoginItem { id: "gnome-keyring".into(), unit: false, on: false },
            Act::LoginItem { id: "xremap.service".into(), unit: true, on: false },
        ],
        "a locked item does not switch"
    );

    click(&mut test, "Mail");
    click_list_remove(&mut test, "Mail");
    assert!(
        handles.acts().contains(&Act::OpenAtLogin { app: "org.app.Mail".into(), on: false }),
        "{:?}",
        handles.acts()
    );
}

#[test]
fn login_items_add_an_app_from_the_picker_and_say_when_there_are_none() {
    let (mut test, handles) = open("isle-settings://general/open-at-login", |_, services| {
        let mut apps = services.apps;
        apps.set(Snapshot {
            available: true,
            state: vec![crate::services::App {
                id: "org.app.Chat".into(),
                name: "Chat".into(),
                icon: None,
                listed: true,
                game: false,
            }],
        });
        let mut items = services.login_items;
        items.set(Snapshot { available: true, state: Default::default() });
    });
    assert!(shows(&test, "No login items"));
    assert!(!shows(&test, "Allow in the Background"));
    let left = (900. - 220.) / 2. + 220. - 600. / 2.;
    let row = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == "No login items")
                .map(|_| node.layout().area)
        })
        .unwrap();
    test.click_cursor((left as f64 + 14., f64::from(row.center().y) + 34.));
    settle(&mut test);
    assert!(shows(&test, "Open at login"), "{:?}", labels(&test));
    click(&mut test, "Chat");
    click(&mut test, "Add");
    assert!(handles.acts().contains(&Act::OpenAtLogin { app: "org.app.Chat".into(), on: true }));
}

#[test]
fn desktop_has_close_windows_and_reopen_apps() {
    let (mut test, handles) = open("isle-settings://desktop", |_, _| {});
    click_described(&mut test, "Close windows when quitting an application", "When on");
    click_described(&mut test, "Reopen apps when logging in", "The apps that were open");
    let writes = handles.writes();
    assert!(writes.contains(&Write::Set(
        "windows.close-windows-when-quitting".into(),
        Value::Bool(true)
    )));
    assert!(writes.contains(&Write::Set("windows.reopen-apps".into(), Value::Bool(false))));
}
