//! The window drawn headless against the fakes on a private bus: a keyring with logins and app
//! secrets, NetworkManager with saved networks, polkit, and keys in a temporary `~/.ssh`.

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_fakes::{
    network::{NetworkManager, Profile, Saved},
    polkit::{Answer, Polkit},
    secrets::{CollectionSpec, ItemSpec, SecretService},
};

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, element| Label::try_downcast(element).map(|l| l.text.to_string()))
}

fn poll(test: &mut TestingRunner, frames: usize) {
    for _ in 0..frames {
        test.poll(Duration::from_millis(16), Duration::from_millis(20));
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn settle_until(test: &mut TestingRunner, wanted: &str) {
    for _ in 0..300 {
        poll(test, 1);
        if labels(test).iter().any(|l| l == wanted) {
            return;
        }
    }
    panic!("never read {wanted:?}: {:?}", labels(test));
}

fn click(test: &mut TestingRunner, caption: &str) {
    let at = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|l| l.text == caption)
                .map(|_| node.layout().area.center())
        })
        .unwrap_or_else(|| panic!("nothing reads {caption:?}: {:?}", labels(test)));
    test.click_cursor((f64::from(at.x), f64::from(at.y)));
}

/// The window's tests set the process's buses and home, so they take turns.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct Fakes {
    pub _bus: isle_fakes::Bus,
    pub secrets: SecretService,
    pub nm: NetworkManager,
    pub polkit: Polkit,
    pub home: PathBuf,
    _runtime: tokio::runtime::Runtime,
}

impl Drop for Fakes {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

pub fn keyring() -> Vec<CollectionSpec> {
    vec![CollectionSpec {
        id: "login".into(),
        label: "Login".into(),
        locked: false,
        alias: Some("default".into()),
        items: vec![
            ItemSpec::new(
                "github.com",
                "org.gnome.keyring.NetworkPassword",
                &[
                    ("user", "alex"),
                    ("server", "github.com"),
                    ("protocol", "https"),
                ],
                "hunter2",
            ),
            ItemSpec::new(
                "Grafana",
                "org.gnome.keyring.NetworkPassword",
                &[("user", "admin"), ("server", "grafana.lan")],
                "graf-pass",
            ),
            ItemSpec::new(
                "Chrome Safe Storage",
                "chrome_libsecret_os_crypt_password_v2",
                &[("application", "chrome")],
                "storage-key",
            ),
            ItemSpec::new(
                "Network secret for Cafe/802-11-wireless-security/psk",
                "org.freedesktop.NetworkManager.Connection",
                &[
                    ("connection-uuid", "u-cafe"),
                    ("setting-name", "802-11-wireless-security"),
                    ("setting-key", "psk"),
                ],
                "cafe-pass",
            ),
        ],
    }]
}

/// The fakes on a bus of their own, the process's buses and home pointed at them.
pub fn fakes(name: &str, collections: Vec<CollectionSpec>) -> Fakes {
    let bus = isle_fakes::Bus::start();
    let home = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/tmp")
        .join(format!("keychain-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    isle_secrets::ssh::generate(
        &home.join(".ssh"),
        "id_ed25519",
        isle_secrets::ssh::NewKind::Ed25519,
        "alex@isle",
        "",
    )
    .unwrap();
    // SAFETY: set while holding the tests' lock, before the window's threads read them.
    unsafe {
        std::env::set_var("DBUS_SESSION_BUS_ADDRESS", &bus.address);
        std::env::set_var("DBUS_SYSTEM_BUS_ADDRESS", &bus.address);
        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_STATE_HOME", home.join(".state"));
        std::env::set_var("XDG_DATA_HOME", home.join(".local/share"));
        std::env::remove_var("SSH_AUTH_SOCK");
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let address = bus.address.clone();
    let (secrets, nm, polkit) = runtime.block_on(async move {
        let secrets = SecretService::start(&address, collections).await.unwrap();
        let profile = |id: &str, uuid: &str, flags: u32| {
            Saved::Profile(Profile {
                id: id.into(),
                uuid: uuid.into(),
                ssid: id.into(),
                key_mgmt: "wpa-psk".into(),
                flags,
                password: format!("{}-pass", id.to_lowercase()),
                identity: None,
            })
        };
        let nm = NetworkManager::start(
            &address,
            Vec::new(),
            vec![profile("Home", "u-home", 0), profile("Cafe", "u-cafe", 1)],
        )
        .await
        .unwrap();
        let polkit = Polkit::start(&address).await.unwrap();
        polkit.answer(Answer::Allow);
        (secrets, nm, polkit)
    });
    Fakes {
        _bus: bus,
        secrets,
        nm,
        polkit,
        home,
        _runtime: runtime,
    }
}

thread_local! {
    static DESKTOP: std::cell::Cell<Option<isle_app::Desktop>> = const { std::cell::Cell::new(None) };
}

/// Chooses a command as its menu or key would.
fn command(test: &mut TestingRunner, id: &str) {
    let desktop = DESKTOP.with(|d| d.get()).expect("the window has drawn");
    let mut queue = desktop.commands;
    queue.write().push(id.to_owned());
    poll(test, 5);
}

fn runner(width: f32) -> TestingRunner {
    let (test, _) = TestingRunner::new(
        move || {
            let desktop = isle_app::provide_for_tests(BTreeMap::new());
            DESKTOP.with(|d| d.set(Some(desktop)));
            crate::window::KeychainWindow {
                shared: crate::Shared::default(),
            }
            .render()
            .into_element()
        },
        (width, 640.).into(),
        |_| {},
        1.,
    );
    test
}

/// Draws the window to `$KEYCHAIN_SHOT` once it has listed, after clicking each caption in
/// `$KEYCHAIN_CLICK` (separated by `|`), for a look.
#[test]
#[ignore]
fn shot() {
    let _turn = serial();
    let Some(path) = std::env::var_os("KEYCHAIN_SHOT") else {
        return;
    };
    let _fakes = fakes("shot", keyring());
    let mut test = runner(
        std::env::var("KEYCHAIN_WIDTH")
            .ok()
            .and_then(|w| w.parse().ok())
            .unwrap_or(1000.),
    );
    settle_until(&mut test, "github.com");
    for id in std::env::var("KEYCHAIN_COMMAND").unwrap_or_default().split('|').filter(|c| !c.is_empty()) {
        poll(&mut test, 10);
        command(&mut test, id);
        poll(&mut test, 20);
    }
    for caption in std::env::var("KEYCHAIN_CLICK")
        .unwrap_or_default()
        .split('|')
        .filter(|c| !c.is_empty())
    {
        click(&mut test, caption);
        poll(&mut test, 20);
    }
    poll(&mut test, 30);
    if std::env::var_os("KEYCHAIN_DEBUG").is_some() {
        let areas: Vec<(String, f32, f32)> = test.find_many(|node, element| {
            Label::try_downcast(element).map(|l| {
                (
                    l.text.to_string(),
                    node.layout().area.min_y(),
                    node.layout().area.height(),
                )
            })
        });
        eprintln!("{areas:?}");
        let rects: Vec<(f32, f32, f32, f32)> = test.find_many(|node, element| {
            Rect::try_downcast(element).and_then(|_| {
                let a = node.layout().area;
                (a.min_x() > 540. && a.min_y() > 130. && a.min_y() < 330.)
                    .then(|| (a.min_x(), a.min_y(), a.width(), a.height()))
            })
        });
        eprintln!("RECTS {rects:?}");
    }
    test.render_to_file(PathBuf::from(path));
}

#[test]
fn every_kind_lists_without_a_secret_and_show_fetches_one() {
    let _turn = serial();
    let fakes = fakes("list", keyring());
    let mut test = runner(1000.);
    settle_until(&mut test, "github.com");
    let seen = labels(&test);
    for wanted in [
        "Grafana",
        "Chrome Safe Storage",
        "Home",
        "Cafe",
        "id_ed25519",
        "Login",
    ] {
        assert!(
            seen.iter().any(|l| l == wanted),
            "{wanted} is listed: {seen:?}"
        );
    }
    assert!(
        !seen.iter().any(|l| l.starts_with("Network secret for")),
        "a Wi-Fi network's password is not an item of its own: {seen:?}"
    );
    assert!(!seen.iter().any(|l| l.contains("hunter2")));
    assert_eq!(fakes.secrets.fetched(), 0, "listing fetches nothing");

    click(&mut test, "github.com");
    poll(&mut test, 5);
    click(&mut test, "Show");
    settle_until(&mut test, "hunter2");
    assert_eq!(fakes.secrets.fetched(), 1);
    assert_eq!(fakes.polkit.checks().len(), 1, "the gate asked once");
    click(&mut test, "Hide");
    poll(&mut test, 5);
    assert!(!labels(&test).iter().any(|l| l == "hunter2"));
    click(&mut test, "Show");
    settle_until(&mut test, "hunter2");
    assert_eq!(
        fakes.polkit.checks().len(),
        1,
        "the grant covers the next click"
    );
    assert_eq!(fakes.secrets.fetched(), 2);
}

#[test]
fn a_login_is_edited_made_and_deleted() {
    let _turn = serial();
    let fakes = fakes("edit", keyring());
    let mut test = runner(1000.);
    settle_until(&mut test, "github.com");
    click(&mut test, "github.com");
    poll(&mut test, 5);
    click(&mut test, "Edit");
    settle_until(&mut test, "Done");
    assert_eq!(fakes.secrets.fetched(), 1, "Edit fetches the password, behind the gate");
    click(&mut test, "Done");
    poll(&mut test, 20);
    assert!(labels(&test).iter().any(|l| l == "Edit"), "Done with nothing changed ends Edit");
    assert_eq!(fakes.secrets.item("login", 0).unwrap().label, "github.com");

    command(&mut test, "delete");
    settle_until(&mut test, "Delete “github.com”?");
    assert!(labels(&test).iter().any(|l| l == "You can't undo this."));
    click(&mut test, "Delete");
    for _ in 0..100 {
        poll(&mut test, 1);
        if fakes.secrets.item("login", 0).is_none() {
            break;
        }
    }
    assert!(fakes.secrets.item("login", 0).is_none(), "the item is deleted");
    settle_until(&mut test, "Grafana");
    assert!(!labels(&test).iter().any(|l| l == "github.com"));

    click(&mut test, "Chrome Safe Storage");
    poll(&mut test, 5);
    command(&mut test, "delete");
    settle_until(&mut test, "Chrome will lose the passwords and sign-ins it keeps.");
    click(&mut test, "Cancel");
    poll(&mut test, 5);
    assert!(fakes.secrets.item("login", 2).is_some());

    command(&mut test, "new-login");
    settle_until(&mut test, "New Login");
    click(&mut test, "Save");
    for _ in 0..100 {
        poll(&mut test, 1);
        if fakes.secrets.len("login") > 4 {
            break;
        }
    }
    let made = fakes.secrets.item("login", 4).expect("a login is made");
    assert_eq!(made.attributes["xdg:schema"], "org.gnome.keyring.NetworkPassword");
    assert_eq!(fakes.secrets.fetched(), 1, "nothing more is fetched");
}

#[test]
fn a_key_is_generated_added_and_trashed_then_put_back() {
    let _turn = serial();
    let fakes = fakes("ssh", keyring());
    let socket = std::env::temp_dir().join(format!("isle-kc-agent-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&socket);
    let mut agent = std::process::Command::new("ssh-agent")
        .args(["-D", "-a"])
        .arg(&socket)
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("ssh-agent");
    let start = std::time::Instant::now();
    while !socket.exists() {
        assert!(start.elapsed() < Duration::from_secs(5), "ssh-agent did not start");
        std::thread::sleep(Duration::from_millis(10));
    }
    // SAFETY: set while holding the tests' lock, before the window's store starts.
    unsafe { std::env::set_var("SSH_AUTH_SOCK", &socket) };
    let mut test = runner(1000.);
    settle_until(&mut test, "id_ed25519");
    click(&mut test, "SSH keys");
    poll(&mut test, 5);
    command(&mut test, "new-key");
    settle_until(&mut test, "New SSH Key");
    click(&mut test, "Save");
    settle_until(&mut test, "id_ed25519_2");
    let made = fakes.home.join(".ssh/id_ed25519_2");
    assert!(made.exists() && fakes.home.join(".ssh/id_ed25519_2.pub").exists());

    click(&mut test, "id_ed25519_2");
    poll(&mut test, 5);
    click(&mut test, "Add to agent");
    settle_until(&mut test, "Remove from agent");
    let listed = std::process::Command::new("ssh-add").arg("-l").env("SSH_AUTH_SOCK", &socket).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&listed.stdout).lines().count(), 1);

    command(&mut test, "delete");
    for _ in 0..100 {
        poll(&mut test, 1);
        if !made.exists() {
            break;
        }
    }
    assert!(!made.exists(), "the key went to the Trash");
    let listed = std::process::Command::new("ssh-add").arg("-l").env("SSH_AUTH_SOCK", &socket).output().unwrap();
    assert!(!listed.status.success() || listed.stdout.is_empty(), "and out of the agent");
    command(&mut test, "undo");
    for _ in 0..100 {
        poll(&mut test, 1);
        if made.exists() {
            break;
        }
    }
    assert!(made.exists(), "Undo puts it back");
    let _ = agent.kill();
    let _ = agent.wait();
    let _ = std::fs::remove_file(&socket);
    // SAFETY: as above.
    unsafe { std::env::remove_var("SSH_AUTH_SOCK") };
}

#[test]
fn a_wifi_password_shows_from_where_it_is_kept_and_a_refusal_shows_nothing() {
    let _turn = serial();
    let fakes = fakes("wifi", keyring());
    let mut test = runner(1000.);
    settle_until(&mut test, "Home");
    click(&mut test, "Home");
    poll(&mut test, 5);
    click(&mut test, "Show");
    settle_until(&mut test, "home-pass");
    assert_eq!(fakes.nm.secrets_asked(), 1);
    click(&mut test, "Cafe");
    poll(&mut test, 5);
    click(&mut test, "Show");
    settle_until(&mut test, "cafe-pass");
    assert_eq!(fakes.nm.secrets_asked(), 1, "an agent-owned password comes from the keyring");

    fakes.nm.refuse_secrets(true);
    click(&mut test, "Home");
    poll(&mut test, 5);
    click(&mut test, "Show");
    poll(&mut test, 30);
    assert!(!labels(&test).iter().any(|l| l == "home-pass"), "refused, the field stays dots");
    assert!(labels(&test).iter().any(|l| l == "Show"));
}
