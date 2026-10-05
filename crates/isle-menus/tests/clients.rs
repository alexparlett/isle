//! The clients against menus exported as apps export them, on a private bus.

use std::{
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};

use isle_menus::{
    Activation, Node, Shortcut, Source, Toggle, dbusmenu,
    exporter::{ActionsExport, DBusMenuExport, GMenuExport},
    gmenu, registrar,
};

struct Bus {
    address: String,
    pid: i32,
}

impl Bus {
    fn start() -> Self {
        let out = Command::new("dbus-daemon")
            .args([
                "--session",
                "--fork",
                "--nopidfile",
                "--print-address=1",
                "--print-pid=1",
            ])
            .output()
            .expect("dbus-daemon");
        let text = String::from_utf8(out.stdout).unwrap();
        let mut lines = text.lines();
        Self {
            address: lines.next().unwrap().to_owned(),
            pid: lines.next().unwrap().trim().parse().unwrap(),
        }
    }

    async fn connect(&self) -> zbus::Connection {
        zbus::connection::Builder::address(self.address.as_str())
            .unwrap()
            .build()
            .await
            .unwrap()
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        unsafe { libc_kill(self.pid) };
    }
}

unsafe fn libc_kill(pid: i32) {
    let _ = Command::new("kill").arg(pid.to_string()).status();
}

fn recorder() -> (Arc<Mutex<Vec<String>>>, isle_menus::exporter::OnActivate) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    (
        seen,
        Arc::new(move |label| sink.lock().unwrap().push(label)),
    )
}

fn labels(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Item(i) => i.label.clone(),
            Node::Separator => "-".into(),
            Node::Section(s) => format!("[{s}]"),
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dbusmenu_tree_reads_with_its_shortcuts_toggles_and_a_submenu_filled_on_asking() {
    let bus = Bus::start();
    let app = bus.connect().await;
    let (seen, on_activate) = recorder();
    app.object_server()
        .at("/MenuBar/1", DBusMenuExport::new(on_activate))
        .await
        .unwrap();
    let service = app.unique_name().unwrap().to_string();
    let isle = bus.connect().await;

    let tree = dbusmenu::fetch(&isle, &service, "/MenuBar/1")
        .await
        .unwrap();
    let titles: Vec<&str> = tree.menus.iter().map(|m| m.label.as_str()).collect();
    assert_eq!(titles, ["File", "Edit", "Help"]);
    let file = &tree.menus[0];
    assert_eq!(
        labels(file.children.as_ref().unwrap()),
        ["New", "Open Recent", "-", "Quit"]
    );
    let new = tree.find(&["File", "New"]).unwrap();
    assert_eq!(
        new.shortcut,
        Some(Shortcut::Keys(vec![vec!["Control".into(), "n".into()]]))
    );
    assert!(!tree.find(&["Edit", "Paste"]).unwrap().enabled);
    assert_eq!(
        tree.find(&["Edit", "Word Wrap"]).unwrap().toggle,
        Toggle::Check(Some(true))
    );
    assert_eq!(
        tree.find(&["Edit", "Dark"]).unwrap().toggle,
        Toggle::Radio(false)
    );

    let recent = tree.find(&["File", "Open Recent"]).unwrap();
    assert!(recent.children.as_ref().unwrap().is_empty());
    let Some(Activation::DBusMenu { id }) = recent.fill else {
        panic!("a submenu to fill");
    };
    let filled = dbusmenu::fetch_submenu(&isle, &service, "/MenuBar/1", id)
        .await
        .unwrap();
    assert_eq!(labels(&filled), ["Recent One"]);

    let Activation::DBusMenu { id } = new.activation else {
        panic!()
    };
    dbusmenu::activate(&isle, &service, "/MenuBar/1", id)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(*seen.lock().unwrap(), ["New"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_gmenu_model_reads_with_its_sections_and_actions_by_prefix() {
    let bus = Bus::start();
    let app = bus.connect().await;
    let (seen, on_activate) = recorder();
    app.object_server()
        .at("/org/test/menus/menubar", GMenuExport)
        .await
        .unwrap();
    app.object_server()
        .at("/org/test", ActionsExport::app(on_activate.clone()))
        .await
        .unwrap();
    app.object_server()
        .at("/org/test/window/1", ActionsExport::window(on_activate))
        .await
        .unwrap();
    let source = Source::GMenu {
        service: app.unique_name().unwrap().to_string(),
        menubar: "/org/test/menus/menubar".into(),
        application: Some("/org/test".into()),
        window: Some("/org/test/window/1".into()),
        unity: None,
    };
    let isle = bus.connect().await;
    let tree = gmenu::fetch(&isle, &source).await.unwrap();
    let titles: Vec<&str> = tree.menus.iter().map(|m| m.label.as_str()).collect();
    assert_eq!(titles, ["File", "Edit"]);
    assert_eq!(
        labels(tree.menus[1].children.as_ref().unwrap()),
        ["Copy", "[View]", "Word Wrap"]
    );
    let copy = tree.find(&["Edit", "Copy"]).unwrap();
    assert_eq!(copy.shortcut, Some(Shortcut::Accel("<Primary>c".into())));
    assert_eq!(
        tree.find(&["Edit", "Word Wrap"]).unwrap().toggle,
        Toggle::Check(Some(true))
    );

    let Activation::GAction { path, name, target } = &copy.activation else {
        panic!()
    };
    assert_eq!(
        (path.as_str(), name.as_str()),
        ("/org/test/window/1", "copy")
    );
    let Source::GMenu { service, .. } = &source else {
        panic!()
    };
    gmenu::activate(&isle, service, path, name, target.as_ref())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(*seen.lock().unwrap(), ["copy"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_registrar_records_windows_drops_a_sender_that_leaves_and_keeps_its_name() {
    let bus = Bus::start();
    let isle = bus.connect().await;
    let registrations = registrar::own(&isle).await.unwrap().unwrap();

    let app = bus.connect().await;
    let sender = app.unique_name().unwrap().to_string();
    app.call_method(
        Some(registrar::NAME),
        registrar::PATH,
        Some(registrar::NAME),
        "RegisterWindow",
        &(42u32, zvariant::ObjectPath::try_from("/MenuBar/1").unwrap()),
    )
    .await
    .unwrap();
    let found = registrations.for_window(42).unwrap();
    assert_eq!(
        (found.sender.as_str(), found.path.as_str()),
        (sender.as_str(), "/MenuBar/1")
    );
    assert_eq!(
        registrar::pid_of(&isle, &sender).await,
        Some(std::process::id())
    );

    drop(app);
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while registrations.for_window(42).is_some() {
        assert!(
            std::time::Instant::now() < deadline,
            "the registration outlived its sender"
        );
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    // A second registrar finds the name taken and says by whom.
    let other = bus.connect().await;
    match registrar::own(&other).await.unwrap() {
        Err(registrar::Taken(owner)) => assert_eq!(owner, isle.unique_name().unwrap().to_string()),
        Ok(_) => panic!("the name changed hands"),
    }
}
