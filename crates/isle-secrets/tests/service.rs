//! The Secret Service client against the fake: listing without a fetch, one fetch per ask,
//! writes, lock and unlock through the prompt, the signals, and a service missing or frozen.

use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::StreamExt;
use isle_fakes::secrets::{CollectionSpec, ItemSpec, SecretService};
use isle_secrets::{
    kinds::{self, Kind},
    service::{Change, Changed, Error, Keyring},
};
use zbus::zvariant::ObjectPath;
use zeroize::Zeroizing;

fn keyring() -> Vec<CollectionSpec> {
    let mut binary = ItemSpec::new(
        "Application key",
        "org.freedesktop.portal.Secret",
        &[("app_id", "org.gnome.Fractal")],
        "",
    );
    binary.secret = vec![0xff, 0x00, 0x01, 0x02];
    binary.content_type = "application/octet-stream".into();
    let mut go = ItemSpec::new(
        "gh:github.com ''",
        "org.freedesktop.Secret.Generic",
        &[("service", "gh:github.com"), ("username", "")],
        "gho_token",
    );
    go.content_type = "text/plain; charset=utf8".into();
    vec![
        CollectionSpec {
            id: "login".into(),
            label: "Login".into(),
            locked: false,
            alias: Some("default".into()),
            items: vec![
                ItemSpec::new(
                    "Chrome Safe Storage",
                    "chrome_libsecret_os_crypt_password_v2",
                    &[("application", "chrome")],
                    "storage-key",
                ),
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
                binary,
                go,
            ],
        },
        CollectionSpec {
            id: "work".into(),
            label: "Work".into(),
            locked: true,
            alias: None,
            items: vec![ItemSpec::new(
                "VPN",
                "org.freedesktop.Secret.Generic",
                &[],
                "vpn",
            )],
        },
    ]
}

async fn open(bus: &isle_fakes::Bus) -> Keyring {
    let conn = zbus::connection::Builder::address(bus.address.as_str())
        .unwrap()
        .method_timeout(isle_secrets::service::ANSWER)
        .build()
        .await
        .unwrap();
    Keyring::open(conn).await.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn lists_without_a_fetch_and_fetches_once_per_ask() {
    let bus = isle_fakes::Bus::start();
    let fake = SecretService::start(&bus.address, keyring()).await.unwrap();
    let keyring = open(&bus).await;
    assert!(keyring.encrypted());

    let sources = keyring.list().await.unwrap();
    assert_eq!(sources.len(), 2);
    let login = &sources[0];
    assert!(login.default && !login.locked);
    assert_eq!(login.items.len(), 4);
    assert!(sources[1].locked && sources[1].items.is_empty());
    let kinds: Vec<Kind> = login
        .items
        .iter()
        .map(|e| kinds::classify(&e.label, &e.attributes).kind)
        .collect();
    assert_eq!(kinds, [Kind::App, Kind::Login, Kind::App, Kind::App]);
    assert_eq!(fake.fetched(), 0);

    let secret = keyring.fetch(&login.items[1].path).await.unwrap();
    assert_eq!(&*secret.text(), "hunter2");
    assert_eq!(fake.fetched(), 1);
    let binary = keyring.fetch(&login.items[2].path).await.unwrap();
    assert!(!binary.is_text());
    assert_eq!(&*binary.text(), "/wABAg==");
    let go = keyring.fetch(&login.items[3].path).await.unwrap();
    assert_eq!(
        &*go.text(),
        "gho_token",
        "a content type oo7 0.6 refuses still reads"
    );
    assert_eq!(fake.fetched(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn writes_follow_the_signals() {
    let bus = isle_fakes::Bus::start();
    let fake = SecretService::start(&bus.address, keyring()).await.unwrap();
    let keyring = open(&bus).await;
    let changes = isle_secrets::service::changes(keyring.connection())
        .await
        .unwrap();
    let mut changes = std::pin::pin!(changes);
    let login = keyring.list().await.unwrap().remove(0);

    let attrs = kinds::login_attributes("me", "https://example.org", "note");
    let item = keyring
        .create(&login.path, "Example", &attrs, "s3cret")
        .await
        .unwrap();
    let Some(Changed::Item {
        item: created,
        gone: false,
        ..
    }) = changes.next().await
    else {
        panic!("no ItemCreated");
    };
    assert_eq!(created, item);
    assert_eq!(fake.secret("login", 4).unwrap(), b"s3cret");
    assert_eq!(
        fake.item("login", 4).unwrap().attributes["server"],
        "example.org"
    );

    keyring
        .update(
            &item,
            Change {
                label: Some("Renamed".into()),
                secret: Some(Zeroizing::new("n3w".into())),
                ..Change::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(fake.item("login", 4).unwrap().label, "Renamed");
    assert_eq!(fake.secret("login", 4).unwrap(), b"n3w");

    fake.relabel("login", 1, "GitHub").await.unwrap();
    let relabelled = loop {
        if let Some(Changed::Item {
            item, gone: false, ..
        }) = changes.next().await
            && item.as_str().ends_with("/1")
        {
            break item;
        }
    };
    let entry = keyring.item(&login.path, relabelled).await.unwrap();
    assert_eq!(entry.label, "GitHub");

    keyring.delete(&item).await.unwrap();
    let gone = loop {
        if let Some(Changed::Item {
            item: i,
            gone: true,
            ..
        }) = changes.next().await
        {
            break i;
        }
    };
    assert_eq!(gone, item);
    assert!(fake.item("login", 4).is_none());
    assert_eq!(fake.fetched(), 0, "writing fetches nothing");
}

#[tokio::test(flavor = "multi_thread")]
async fn unlocks_through_the_prompt_and_locks() {
    let bus = isle_fakes::Bus::start();
    let fake = Arc::new(SecretService::start(&bus.address, keyring()).await.unwrap());
    let keyring = open(&bus).await;
    let work = keyring.list().await.unwrap().remove(1);
    assert!(work.locked);

    let completer = fake.clone();
    let answer = tokio::spawn(async move {
        while completer.prompts_shown() == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        completer.complete_prompt(false).await.unwrap();
    });
    assert_eq!(keyring.unlock(&work.path).await, Err(Error::Dismissed));
    answer.await.unwrap();

    let completer = fake.clone();
    tokio::spawn(async move {
        while completer.prompts_shown() == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        completer.complete_prompt(true).await.unwrap();
    });
    keyring.unlock(&work.path).await.unwrap();
    let work = keyring.list().await.unwrap().remove(1);
    assert!(!work.locked);
    assert_eq!(work.items[0].label, "VPN");

    keyring.lock(&work.path).await.unwrap();
    assert_eq!(
        keyring.fetch(&work.items[0].path).await.unwrap_err(),
        Error::Locked
    );
    assert!(keyring.list().await.unwrap()[1].locked);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plain_service_is_used_plainly() {
    let bus = isle_fakes::Bus::start();
    let fake = SecretService::start(&bus.address, keyring()).await.unwrap();
    fake.plain_only();
    let keyring = open(&bus).await;
    assert!(!keyring.encrypted());
    let login = keyring.list().await.unwrap().remove(0);
    assert_eq!(
        &*keyring.fetch(&login.items[1].path).await.unwrap().text(),
        "hunter2"
    );
    let mut attrs = HashMap::new();
    attrs.insert("a".to_owned(), "b".to_owned());
    keyring
        .create(&login.path, "x", &attrs, "plain")
        .await
        .unwrap();
    assert_eq!(fake.secret("login", 4).unwrap(), b"plain");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_or_frozen_service_says_so_in_time() {
    let bus = isle_fakes::Bus::start();
    let conn = isle_fakes::connect(&bus.address).await.unwrap();
    assert_eq!(Keyring::open(conn).await.unwrap_err(), Error::Missing);

    let fake = SecretService::start(&bus.address, keyring()).await.unwrap();
    let keyring = open(&bus).await;
    let login = keyring.list().await.unwrap().remove(0);
    fake.freeze();
    let start = std::time::Instant::now();
    assert_eq!(keyring.list().await.unwrap_err(), Error::Frozen);
    assert_eq!(
        keyring
            .fetch(&ObjectPath::try_from(login.items[1].path.as_str()).unwrap())
            .await
            .unwrap_err(),
        Error::Frozen
    );
    assert!(start.elapsed() < isle_secrets::service::ANSWER * 3);
}
