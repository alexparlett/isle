//! The Secret Service fake read and written by `oo7::dbus` itself: an encrypted session,
//! listing, a fetch, create, edit, delete, lock and unlock through a prompt, and the signals.

use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use isle_fakes::secrets::{CollectionSpec, ItemSpec, SecretService};
use oo7::{
    Key, Secret,
    dbus::api::{DBusSecret, Service},
};

fn keyring() -> Vec<CollectionSpec> {
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
                "vpn-secret",
            )],
        },
    ]
}

#[tokio::test(flavor = "multi_thread")]
async fn oo7_reads_and_writes_the_fake() {
    let bus = isle_fakes::Bus::start();
    let fake = SecretService::start(&bus.address, keyring()).await.unwrap();
    let conn = isle_fakes::connect(&bus.address).await.unwrap();
    let service = Service::new(&conn).await.unwrap();

    let private = Key::generate_private_key().unwrap();
    let public = Key::generate_public_key(&private).unwrap();
    let (server, session) = service.open_session(Some(public)).await.unwrap();
    let aes = Arc::new(Key::generate_aes_key(&private, &server.unwrap()).unwrap());
    let session = Arc::new(session);

    let collections = service.collections().await.unwrap();
    assert_eq!(collections.len(), 2);
    let login = &collections[0];
    assert_eq!(login.label().await.unwrap(), "Login");
    assert!(!login.is_locked().await.unwrap());
    let items = login.items().await.unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[1].label().await.unwrap(), "github.com");
    assert_eq!(items[1].attributes().await.unwrap()["user"], "alex");
    assert_eq!(fake.fetched(), 0, "listing fetches nothing");

    let secret = items[1].secret(&session).await.unwrap();
    assert_ne!(
        secret.value(),
        b"hunter2",
        "the value crosses the bus encrypted"
    );
    assert_eq!(secret.decrypt(Some(&aes)).unwrap(), Secret::text("hunter2"));
    assert_eq!(fake.fetched(), 1);

    let created = login.receive_item_created().await.unwrap();
    let mut created = std::pin::pin!(created);
    let new = DBusSecret::new_encrypted(session.clone(), "s3cret", &aes).unwrap();
    let item = login
        .create_item(
            "example.org",
            &[("user", "me"), ("server", "example.org")],
            &new,
            false,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        created.next().await.unwrap().inner().path(),
        item.inner().path()
    );
    assert_eq!(fake.secret("login", 2).unwrap(), b"s3cret");

    let changed = login.receive_item_changed().await.unwrap();
    let mut changed = std::pin::pin!(changed);
    item.set_label("Example").await.unwrap();
    assert_eq!(
        changed.next().await.unwrap().inner().path(),
        item.inner().path()
    );
    item.set_secret(&DBusSecret::new_encrypted(session.clone(), "n3w", &aes).unwrap())
        .await
        .unwrap();
    assert_eq!(fake.item("login", 2).unwrap().label, "Example");
    assert_eq!(fake.secret("login", 2).unwrap(), b"n3w");

    let deleted = login.receive_item_deleted().await.unwrap();
    let mut deleted = std::pin::pin!(deleted);
    item.delete(None).await.unwrap();
    assert_eq!(
        deleted.next().await.unwrap().as_str(),
        item.inner().path().as_str()
    );
    assert!(fake.item("login", 2).is_none());

    let work = &collections[1];
    assert!(work.is_locked().await.unwrap());
    assert!(
        work.items().await.unwrap().is_empty(),
        "a locked keyring lists nothing"
    );
    let fake = Arc::new(fake);
    let completer = fake.clone();
    tokio::spawn(async move {
        while completer.prompts_shown() == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        completer.complete_prompt(true).await.unwrap();
    });
    let unlocked = tokio::time::timeout(
        Duration::from_secs(5),
        service.unlock(&[work.inner().path().to_owned()], None),
    )
    .await
    .expect("the prompt completes")
    .unwrap();
    assert_eq!(unlocked.len(), 1);
    assert!(!work.is_locked().await.unwrap());
    assert_eq!(work.items().await.unwrap().len(), 1);

    service
        .lock(&[work.inner().path().to_owned()], None)
        .await
        .unwrap();
    assert!(work.is_locked().await.unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plain_only_service_refuses_the_encrypted_session() {
    let bus = isle_fakes::Bus::start();
    let fake = SecretService::start(&bus.address, keyring()).await.unwrap();
    fake.plain_only();
    let conn = isle_fakes::connect(&bus.address).await.unwrap();
    let service = Service::new(&conn).await.unwrap();
    let private = Key::generate_private_key().unwrap();
    let public = Key::generate_public_key(&private).unwrap();
    assert!(service.open_session(Some(public)).await.is_err());
    let (_, session) = service.open_session(None).await.unwrap();
    let items = service.collections().await.unwrap()[0]
        .items()
        .await
        .unwrap();
    let secret = items[0].secret(&session).await.unwrap();
    assert_eq!(secret.decrypt(None).unwrap(), Secret::text("storage-key"));
}
