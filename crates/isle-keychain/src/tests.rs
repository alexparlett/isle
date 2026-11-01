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
    secrets: Option<SecretService>,
    pub nm: NetworkManager,
    pub polkit: Polkit,
    pub home: PathBuf,
    pub runtime: tokio::runtime::Runtime,
}

impl Fakes {
    pub fn secrets(&self) -> &SecretService {
        self.secrets.as_ref().expect("a Secret Service")
    }
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
pub fn fakes(name: &str, collections: impl Into<Option<Vec<CollectionSpec>>>) -> Fakes {
    let collections = collections.into();
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
        let secrets = match collections {
            Some(c) => Some(SecretService::start(&address, c).await.unwrap()),
            None => None,
        };
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
        runtime,
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
    let missing = std::env::var_os("KEYCHAIN_MISSING").is_some();
    let _fakes = fakes("shot", (!missing).then(keyring));
    let mut test = runner(
        std::env::var("KEYCHAIN_WIDTH")
            .ok()
            .and_then(|w| w.parse().ok())
            .unwrap_or(1000.),
    );
    settle_until(&mut test, if missing { "Cafe" } else { "github.com" });
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
    assert_eq!(fakes.secrets().fetched(), 0, "listing fetches nothing");

    click(&mut test, "github.com");
    poll(&mut test, 5);
    click(&mut test, "Show");
    settle_until(&mut test, "hunter2");
    assert_eq!(fakes.secrets().fetched(), 1);
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
    assert_eq!(fakes.secrets().fetched(), 2);
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
    assert_eq!(fakes.secrets().fetched(), 1, "Edit fetches the password, behind the gate");
    click(&mut test, "Done");
    poll(&mut test, 20);
    assert!(labels(&test).iter().any(|l| l == "Edit"), "Done with nothing changed ends Edit");
    assert_eq!(fakes.secrets().item("login", 0).unwrap().label, "github.com");

    command(&mut test, "delete");
    settle_until(&mut test, "Delete “github.com”?");
    assert!(labels(&test).iter().any(|l| l == "You can't undo this."));
    click(&mut test, "Delete");
    for _ in 0..100 {
        poll(&mut test, 1);
        if fakes.secrets().item("login", 0).is_none() {
            break;
        }
    }
    assert!(fakes.secrets().item("login", 0).is_none(), "the item is deleted");
    settle_until(&mut test, "Grafana");
    assert!(!labels(&test).iter().any(|l| l == "github.com"));

    click(&mut test, "Chrome Safe Storage");
    poll(&mut test, 5);
    command(&mut test, "delete");
    settle_until(&mut test, "Chrome will lose the passwords and sign-ins it keeps.");
    click(&mut test, "Cancel");
    poll(&mut test, 5);
    assert!(fakes.secrets().item("login", 2).is_some());

    command(&mut test, "new-login");
    settle_until(&mut test, "New Login");
    click(&mut test, "Save");
    for _ in 0..100 {
        poll(&mut test, 1);
        if fakes.secrets().len("login") > 4 {
            break;
        }
    }
    let made = fakes.secrets().item("login", 4).expect("a login is made");
    assert_eq!(made.attributes["xdg:schema"], "org.gnome.keyring.NetworkPassword");
    assert_eq!(fakes.secrets().fetched(), 1, "nothing more is fetched");
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

#[test]
fn a_locked_keyring_heads_the_list_and_unlocks_through_its_prompt() {
    let _turn = serial();
    let mut collections = keyring();
    collections.push(CollectionSpec {
        id: "work".into(),
        label: "Work".into(),
        locked: true,
        alias: None,
        items: vec![ItemSpec::new("Jira", "org.gnome.keyring.NetworkPassword", &[("user", "a"), ("server", "jira.work")], "j")],
    });
    let fakes = fakes("locked", collections);
    let mut test = runner(1000.);
    settle_until(&mut test, "Work is locked");
    assert!(!labels(&test).iter().any(|l| l == "Jira"));
    click(&mut test, "Unlock…");
    for _ in 0..200 {
        poll(&mut test, 1);
        if fakes.secrets().prompts_shown() > 0 {
            break;
        }
    }
    fakes.runtime.block_on(fakes.secrets().complete_prompt(true)).unwrap();
    settle_until(&mut test, "Jira");
    assert!(!labels(&test).iter().any(|l| l == "Work is locked"));
}

#[test]
fn without_a_secret_service_wifi_and_ssh_keys_work_on() {
    let _turn = serial();
    let _fakes = fakes("missing", None);
    let mut test = runner(1000.);
    settle_until(&mut test, "No keyring service is running. Choose one in Settings › General › Passwords.");
    settle_until(&mut test, "id_ed25519");
    let seen = labels(&test);
    assert!(seen.iter().any(|l| l == "Home") && seen.iter().any(|l| l == "id_ed25519"), "{seen:?}");
    click(&mut test, "Logins");
    settle_until(&mut test, "No keyring service is running. Choose one in Settings › General › Passwords.");
}

#[test]
fn a_frozen_service_says_so_in_time() {
    let _turn = serial();
    let fakes = fakes("frozen", keyring());
    fakes.secrets().freeze();
    let mut test = runner(1000.);
    let start = std::time::Instant::now();
    for _ in 0..600 {
        poll(&mut test, 1);
        if labels(&test).iter().any(|l| l.starts_with("The keyring isn't answering")) {
            break;
        }
    }
    assert!(labels(&test).iter().any(|l| l.starts_with("The keyring isn't answering")), "{:?}", labels(&test));
    assert!(start.elapsed() < Duration::from_secs(12));
    settle_until(&mut test, "Home");
}

fn many(n: usize, secret: &str) -> Vec<CollectionSpec> {
    let mut items: Vec<ItemSpec> = (0..n)
        .map(|i| {
            if i % 2 == 0 {
                ItemSpec::new(&format!("Item {i:04}"), "org.gnome.keyring.NetworkPassword", &[("user", "u"), ("server", &format!("s{i}.example"))], secret)
            } else {
                ItemSpec::new(&format!("Item {i:04}"), "org.freedesktop.Secret.Generic", &[("application", "tool")], secret)
            }
        })
        .collect();
    items.sort_by(|a, b| a.label.cmp(&b.label));
    vec![CollectionSpec { id: "login".into(), label: "Login".into(), locked: false, alias: Some("default".into()), items }]
}

fn press(test: &mut TestingRunner, key: Key, times: usize) {
    for _ in 0..times {
        test.press_key(key.clone());
    }
    poll(test, 2);
}

/// Choice 26: listing, searching, selecting and scrolling every kind of two thousand items
/// fetches nothing; Show then fetches one.
#[test]
fn nothing_is_fetched_across_two_thousand_items() {
    let _turn = serial();
    let fakes = fakes("many", many(2000, "s"));
    let start = std::time::Instant::now();
    let mut test = runner(1000.);
    settle_until(&mut test, "Item 0000");
    eprintln!("listed 2,000 items in {:?} (debug build)", start.elapsed());
    for row in ["Logins", "Apps", "Wi-Fi", "SSH keys", "Login", "All"] {
        click(&mut test, row);
        poll(&mut test, 3);
    }
    press(&mut test, Key::Named(NamedKey::ArrowDown), 60);
    click(&mut test, "Item 0002");
    poll(&mut test, 3);
    assert_eq!(fakes.secrets().fetched(), 0, "nothing is fetched to list, select or scroll");
    click(&mut test, "Show");
    for _ in 0..100 {
        poll(&mut test, 1);
        if fakes.secrets().fetched() > 0 {
            break;
        }
    }
    poll(&mut test, 5);
    assert_eq!(fakes.secrets().fetched(), 1, "Show fetches exactly one");
}

static LOGS: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());

struct Captured;

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        LOGS.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn files_holding(root: &std::path::Path, needle: &[u8]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else { return out };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            out.extend(files_holding(&path, needle));
        } else if std::fs::read(&path).is_ok_and(|b| b.windows(needle.len()).any(|w| w == needle)) {
            out.push(path);
        }
    }
    out
}

/// Choice 26's canary: shown, copied, edited and generated beside, it reaches no log and no
/// file Keychain writes.
#[test]
fn the_canary_leaves_no_trace() {
    let _turn = serial();
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(|| Captured)
        .try_init();
    const CANARY: &str = "CANARY-7f3a91";
    let fakes = fakes("canary", many(4, CANARY));
    let mut test = runner(1000.);
    settle_until(&mut test, "Item 0000");
    click(&mut test, "Item 0000");
    poll(&mut test, 3);
    click(&mut test, "Show");
    settle_until(&mut test, CANARY);
    click(&mut test, "Copy");
    poll(&mut test, 10);
    click(&mut test, "Edit");
    settle_until(&mut test, "Done");
    command(&mut test, "generate");
    poll(&mut test, 10);
    click(&mut test, "Done");
    poll(&mut test, 10);
    click(&mut test, "Done");
    poll(&mut test, 20);
    drop(test);
    let logs = String::from_utf8_lossy(&LOGS.lock().unwrap()).into_owned();
    assert!(!logs.contains(CANARY), "the canary is in the logs");
    let found = files_holding(&fakes.home, CANARY.as_bytes());
    assert!(found.is_empty(), "the canary is in {found:?}");
}

/// Choice 29's bars, on a release build: the list of 2,000 items drawn, and a Show drawn once
/// the grant is held. `cargo test --release -p isle-keychain -- --ignored bars --nocapture`.
#[test]
#[ignore]
fn bars() {
    let _turn = serial();
    let _fakes = fakes("bars", many(2000, "bar-secret"));
    let mut runs = Vec::new();
    for _ in 0..5 {
        let start = std::time::Instant::now();
        let mut test = runner(1000.);
        settle_until(&mut test, "Item 0000");
        runs.push(start.elapsed());
        drop(test);
    }
    runs.sort();
    eprintln!("opening, 2,000 items: median {:?}, worst {:?}", runs[2], runs[4]);
    let mut test = runner(1000.);
    settle_until(&mut test, "Item 0000");
    click(&mut test, "Item 0000");
    poll(&mut test, 3);
    click(&mut test, "Show");
    settle_until(&mut test, "bar-secret");
    click(&mut test, "Hide");
    poll(&mut test, 3);
    let mut reveals = Vec::new();
    for _ in 0..5 {
        let start = std::time::Instant::now();
        click(&mut test, "Show");
        while !labels(&test).iter().any(|l| l == "bar-secret") {
            test.poll(Duration::from_millis(1), Duration::from_millis(1));
        }
        reveals.push(start.elapsed());
        click(&mut test, "Hide");
        poll(&mut test, 3);
    }
    reveals.sort();
    eprintln!("reveal with the grant held: median {:?}, worst {:?}", reveals[2], reveals[4]);
}
