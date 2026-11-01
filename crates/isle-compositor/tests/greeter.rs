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

fn login() -> String {
    String::from_utf8(
        std::process::Command::new("id")
            .arg("-un")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_publishes_what_the_greeter_draws_and_nothing_else() {
    let looks = std::env::temp_dir().join(format!("isle-looks-{}", std::process::id()));
    let mine = looks.join(login());
    std::fs::create_dir_all(&mine).unwrap();
    let picture = looks.join("picture.png");
    image::RgbaImage::from_pixel(64, 48, image::Rgba([200, 40, 40, 255]))
        .save(&picture)
        .unwrap();
    let settings = format!(
        "appearance {{\n    mode \"light\"\n    glass 0.5\n}}\nkeyboard {{\n    layouts \"gb\" \"us\"\n}}\n\
         windows {{\n    edge-tiling #false\n}}\nwallpaper {{\n    source \"{}\"\n}}\n",
        picture.display()
    );
    let looks_env = looks.clone();
    let isle = Isle::start_env(
        "greeter-publish",
        &settings,
        |_| {},
        move |_| {
            vec![(
                "ISLE_GREETER_LOOKS".into(),
                looks_env.to_string_lossy().into_owned(),
            )]
        },
    )
    .await;
    let look = mine.join("look.kdl");
    wait_until(|| async {
        std::fs::read_to_string(&look).is_ok_and(|t| t.contains("wallpaper.png"))
    })
    .await;
    let text = std::fs::read_to_string(&look).unwrap();
    assert!(text.contains("mode \"light\""), "{text}");
    assert!(text.contains("layouts \"gb\" \"us\""), "{text}");
    assert!(
        !text.contains("glass") && !text.contains("windows"),
        "{text}"
    );
    let published = image::open(mine.join("wallpaper.png")).unwrap();
    assert_eq!((published.width(), published.height()), (1920, 1080));
    drop(isle);
    let _ = std::fs::remove_dir_all(&looks);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_greeter_draws_each_users_look_and_the_shipped_one_for_a_broken_file() {
    let looks = std::env::temp_dir().join(format!("isle-looks-greeter-{}", std::process::id()));
    let picture = looks.join("alex/wallpaper.png");
    std::fs::create_dir_all(looks.join("alex")).unwrap();
    image::RgbaImage::from_pixel(64, 48, image::Rgba([40, 120, 200, 255]))
        .save(&picture)
        .unwrap();
    let alex = format!(
        "appearance {{\n    mode \"light\"\n}}\nkeyboard {{\n    layouts \"gb\"\n}}\n\
         wallpaper {{\n    source \"{}\"\n}}\n",
        picture.display()
    );
    for (user, text) in [
        ("alex", alex.as_str()),
        ("sam", "appearance {\n    mode \"light\"\n"),
    ] {
        std::fs::create_dir_all(looks.join(user)).unwrap();
        std::fs::write(looks.join(user).join("look.kdl"), text).unwrap();
    }
    let socket = std::env::temp_dir().join(format!("isle-greetd-look-{}.sock", std::process::id()));
    let _fake = isle_fakes::greetd::Greetd::start(
        &socket,
        vec![
            isle_fakes::greetd::User::new("alex", "a"),
            isle_fakes::greetd::User::new("sam", "s"),
        ],
    );
    let looks_env = looks.clone();
    let isle = Isle::start_greeter(
        "greeter-look",
        |_| {},
        move |_| {
            vec![
                ("GREETD_SOCK".into(), socket.to_string_lossy().into_owned()),
                (
                    "ISLE_GREETER_LOOKS".into(),
                    looks_env.to_string_lossy().into_owned(),
                ),
            ]
        },
    )
    .await;
    let _accounts = isle_fakes::accounts::Accounts::start(
        &isle.system_address(),
        vec![
            isle_fakes::accounts::User::new(1000, "alex", "Alex Example"),
            isle_fakes::accounts::User::new(1001, "sam", "Sam Example"),
        ],
    )
    .await
    .unwrap();
    let greeter = || async { status(&isle).await["greeter"].clone() };
    wait_until(|| async { greeter().await["users"] == serde_json::json!(["alex", "sam"]) }).await;
    wait_until(|| async { greeter().await["dark"] == false }).await;
    assert_eq!(greeter().await["layouts"], serde_json::json!(["gb"]));
    wait_until(|| async { isle.log().contains("alex/wallpaper.png") }).await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.keep_shot("greeter-look").await;

    // Sam's file does not parse: the shipped look, dark, no layouts.
    press(&isle, RIGHT).await;
    wait_until(|| async { greeter().await["dark"] == true }).await;
    assert_eq!(greeter().await["layouts"], serde_json::json!([]));
    drop(isle);
    let _ = std::fs::remove_dir_all(&looks);
}
