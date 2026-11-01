//! The greeter mode: the lock and nothing else, with no Wayland socket, no names on the
//! session bus and nothing written under the home. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};

async fn status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.lock_status().await.unwrap()).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_greeter_is_the_lock_and_nothing_else() {
    let isle = Isle::start_greeter("greeter-only", |_| {}, |_| Vec::new()).await;
    assert_eq!(status(&isle).await["locked"], true);
    wait_until(|| async {
        let kinds = isle.test().await.frame_kinds("HEADLESS-1").await.unwrap();
        kinds.iter().any(|k| k == "shell")
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.keep_shot("greeter").await;

    // No Wayland socket for clients.
    let sockets: Vec<_> = std::fs::read_dir(isle.dir.join("runtime"))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("wayland-"))
        .collect();
    assert!(sockets.is_empty(), "{sockets:?}");

    // None of a session's names.
    let dbus = zbus::fdo::DBusProxy::new(&isle.bus).await.unwrap();
    for name in [
        "org.freedesktop.Notifications",
        "org.kde.StatusNotifierWatcher",
        "org.freedesktop.impl.portal.desktop.isle",
        "org.freedesktop.ScreenSaver",
        "com.canonical.AppMenu.Registrar",
    ] {
        let name = zbus::names::BusName::try_from(name).unwrap();
        assert!(
            !dbus.name_has_owner(name.clone()).await.unwrap(),
            "{name} is owned"
        );
    }

    // The greeter's lock never ends.
    isle.test().await.unlock().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(status(&isle).await["locked"], true);

    // Nothing written in the home or the user's configuration.
    for dir in ["home", "config"] {
        let left: Vec<_> = std::fs::read_dir(isle.dir.join(dir)).unwrap().collect();
        assert!(left.is_empty(), "{dir}: {left:?}");
    }
}

const RETURN: u32 = 28;
const ESCAPE: u32 = 1;
const RIGHT: u32 = 106;

/// evdev codes for lowercase letters and digits typed on a US layout.
fn code(c: char) -> u32 {
    let row = |s: &str, first: u32| s.find(c).map(|i| first + i as u32);
    row("qwertyuiop", 16)
        .or_else(|| row("asdfghjkl", 30))
        .or_else(|| row("zxcvbnm", 44))
        .or_else(|| row("1234567890", 2))
        .unwrap_or_else(|| panic!("no key for {c}"))
}

async fn type_text(isle: &Isle, text: &str) {
    let test = isle.test().await;
    for c in text.chars() {
        test.key(code(c), true).await.unwrap();
        test.key(code(c), false).await.unwrap();
    }
}

async fn press(isle: &Isle, key: u32) {
    let test = isle.test().await;
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
}

/// A greeter with `users` in AccountsService and greetd, and Isle's and Hyprland's
/// session entries.
async fn greeter(
    name: &str,
    users: &[(&str, &str, &str)],
) -> (
    Isle,
    isle_fakes::greetd::Greetd,
    isle_fakes::accounts::Accounts,
) {
    let socket =
        std::env::temp_dir().join(format!("isle-greetd-{name}-{}.sock", std::process::id()));
    let fake = isle_fakes::greetd::Greetd::start(
        &socket,
        users
            .iter()
            .map(|(login, _, password)| isle_fakes::greetd::User::new(login, password))
            .collect(),
    );
    let isle = Isle::start_greeter(
        name,
        |dir| {
            let sessions = dir.join("data-system/wayland-sessions");
            std::fs::create_dir_all(&sessions).unwrap();
            std::fs::write(
                sessions.join("isle.desktop"),
                "[Desktop Entry]\nName=Isle\nExec=isle-session\nDesktopNames=Isle\n",
            )
            .unwrap();
            std::fs::write(
                sessions.join("hyprland.desktop"),
                "[Desktop Entry]\nName=Hyprland\nExec=/usr/bin/start-hyprland\nDesktopNames=Hyprland\n",
            )
            .unwrap();
        },
        |_| vec![("GREETD_SOCK".into(), socket.to_string_lossy().into_owned())],
    )
    .await;
    let accounts = isle_fakes::accounts::Accounts::start(
        &isle.system_address(),
        users
            .iter()
            .enumerate()
            .map(|(i, (login, real, _))| {
                isle_fakes::accounts::User::new(1000 + i as u64, login, real)
            })
            .collect(),
    )
    .await
    .unwrap();
    let logins: Vec<String> = users.iter().map(|(l, _, _)| l.to_string()).collect();
    wait_until(|| async { status(&isle).await["greeter"]["users"] == serde_json::json!(logins) })
        .await;
    (isle, fake, accounts)
}

#[tokio::test(flavor = "multi_thread")]
async fn one_user_signs_in_to_isle_and_the_greeter_exits() {
    let (mut isle, fake, _accounts) =
        greeter("greeter-sign-in", &[("alex", "Alex Example", "hunter2")]).await;
    wait_until(|| async { status(&isle).await["greeter"]["prompt"] == "Password: " }).await;
    assert_eq!(status(&isle).await["greeter"]["session"], "isle");

    // A wrong password first.
    type_text(&isle, "nope").await;
    press(&isle, RETURN).await;
    wait_until(|| async { status(&isle).await["status"] == "wrong" }).await;
    wait_until(|| async { status(&isle).await["greeter"]["prompt"] == "Password: " }).await;
    assert_eq!(status(&isle).await["failures"], 1);

    type_text(&isle, "hunter2").await;
    press(&isle, RETURN).await;
    wait_until(|| async { !fake.started().is_empty() }).await;
    let started = &fake.started()[0];
    assert_eq!(started.user, "alex");
    assert_eq!(started.cmd, ["isle-session"]);
    assert!(started.env.contains(&"XDG_CURRENT_DESKTOP=Isle".to_owned()));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while isle.compositor_exit().is_none() {
        assert!(
            std::time::Instant::now() < deadline,
            "the greeter did not exit"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let memory = std::fs::read_to_string(isle.dir.join("state/isle/greeter.json")).unwrap();
    assert!(memory.contains("\"last_user\": \"alex\""), "{memory}");
    assert!(!memory.contains("hunter2"));
    assert!(!isle.log().contains("hunter2"));
}

#[tokio::test(flavor = "multi_thread")]
async fn several_users_are_a_list_and_each_keeps_their_session() {
    let (isle, fake, _accounts) = greeter(
        "greeter-list",
        &[("alex", "Alex Example", "a"), ("sam", "Sam Example", "s")],
    )
    .await;
    let greeter = || async { status(&isle).await["greeter"].clone() };
    assert_eq!(greeter().await["chosen"], serde_json::Value::Null);
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.keep_shot("greeter-list").await;

    // Right and Return choose Sam; Escape goes back to the list.
    press(&isle, RIGHT).await;
    wait_until(|| async { greeter().await["selected"] == 1 }).await;
    press(&isle, RETURN).await;
    wait_until(|| async { greeter().await["chosen"] == 1 }).await;
    wait_until(|| async { greeter().await["prompt"] == "Password: " }).await;
    press(&isle, ESCAPE).await;
    wait_until(|| async { greeter().await["chosen"] == serde_json::Value::Null }).await;

    // Sam signs in to Hyprland from the session menu.
    let test = isle.test().await;
    test.greeter_act("choose-user", 1).await.unwrap();
    wait_until(|| async { greeter().await["prompt"] == "Password: " }).await;
    test.greeter_act("session-menu", 1).await.unwrap();
    assert_eq!(greeter().await["menu_open"], true);
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.keep_shot("greeter-menu").await;
    test.greeter_act("choose-session", 0).await.unwrap();
    assert_eq!(greeter().await["session"], "hyprland");
    assert_eq!(greeter().await["menu_open"], false);
    type_text(&isle, "s").await;
    press(&isle, RETURN).await;
    wait_until(|| async { !fake.started().is_empty() }).await;
    let started = &fake.started()[0];
    assert_eq!(
        (started.user.as_str(), started.cmd.as_slice()),
        ("sam", &["/usr/bin/start-hyprland".to_owned()][..])
    );
    let memory = std::fs::read_to_string(isle.dir.join("state/isle/greeter.json")).unwrap();
    assert!(memory.contains("\"sam\": \"hyprland\""), "{memory}");
}
