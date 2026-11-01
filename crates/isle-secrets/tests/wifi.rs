//! Saved Wi-Fi against fake NetworkManager and Secret Service: system-owned passwords from
//! NetworkManager, agent-owned ones from the keyring, nothing for one not saved, and a refusal.

use isle_fakes::{
    network::{NetworkManager, Profile, Saved},
    secrets::{CollectionSpec, ItemSpec, SecretService},
};
use isle_network::Keeps;
use isle_secrets::{
    service::{Error, Keyring},
    wifi,
};

fn profile(id: &str, uuid: &str, key_mgmt: &str, flags: u32, password: &str) -> Saved {
    Saved::Profile(Profile {
        id: id.into(),
        uuid: uuid.into(),
        ssid: id.into(),
        key_mgmt: key_mgmt.into(),
        flags,
        password: password.into(),
        identity: (key_mgmt == "wpa-eap").then(|| "alex@corp".into()),
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn each_profile_gives_its_password_from_where_it_keeps_it() {
    let bus = isle_fakes::Bus::start();
    let nm = NetworkManager::start(
        &bus.address,
        Vec::new(),
        vec![
            profile("Home", "u-home", "wpa-psk", 0, "home-pass"),
            profile("Cafe", "u-cafe", "sae", 1, "agent-held"),
            profile("Guest", "u-guest", "wpa-psk", 2, ""),
            profile("Corp", "u-corp", "wpa-eap", 0, "corp-pass"),
            profile("Library", "u-lib", "", 0, ""),
            Saved::Vpn("Work".into()),
        ],
    )
    .await
    .unwrap();
    let secrets = SecretService::start(
        &bus.address,
        vec![CollectionSpec {
            id: "login".into(),
            label: "Login".into(),
            locked: false,
            alias: Some("default".into()),
            items: vec![ItemSpec::new(
                "Network secret for Cafe/802-11-wireless-security/psk",
                "org.freedesktop.NetworkManager.Connection",
                &[
                    ("connection-uuid", "u-cafe"),
                    ("setting-name", "802-11-wireless-security"),
                    ("setting-key", "psk"),
                ],
                "agent-held",
            )],
        }],
    )
    .await
    .unwrap();
    let conn = isle_fakes::connect(&bus.address).await.unwrap();
    let keyring = Keyring::open(conn.clone()).await.unwrap();
    let sources = keyring.list().await.unwrap();

    let saved = wifi::saved(&conn).await.unwrap();
    let names: Vec<&str> = saved.iter().map(|w| w.id.as_str()).collect();
    assert_eq!(
        names,
        ["Cafe", "Corp", "Guest", "Home", "Library"],
        "VPNs are not Wi-Fi"
    );
    let by = |id: &str| saved.iter().find(|w| w.id == id).unwrap();
    assert_eq!(by("Home").keeps, Keeps::System);
    assert_eq!(by("Cafe").keeps, Keeps::Agent);
    assert_eq!(by("Guest").keeps, Keeps::NotSaved);
    assert_eq!(by("Library").keeps, Keeps::Nothing);
    assert_eq!(by("Corp").identity.as_deref(), Some("alex@corp"));
    assert_eq!(by("Corp").setting, "802-1x");
    assert_eq!(nm.secrets_asked(), 0, "listing asks for no secret");
    assert_eq!(secrets.fetched(), 0);

    let read = |id: &'static str| {
        let w = by(id).clone();
        let item = wifi::agent_item(&sources, &w);
        let (conn, keyring) = (&conn, &keyring);
        async move { wifi::password(conn, Some(keyring), item.as_ref(), &w).await }
    };
    assert_eq!(&*read("Home").await.unwrap().unwrap().text(), "home-pass");
    assert_eq!(&*read("Corp").await.unwrap().unwrap().text(), "corp-pass");
    assert_eq!(nm.secrets_asked(), 2);
    assert_eq!(&*read("Cafe").await.unwrap().unwrap().text(), "agent-held");
    assert_eq!(
        nm.secrets_asked(),
        2,
        "an agent-owned password is not asked of NetworkManager"
    );
    assert_eq!(secrets.fetched(), 1);
    assert!(read("Guest").await.unwrap().is_none());
    assert!(read("Library").await.unwrap().is_none());

    nm.refuse_secrets(true);
    assert_eq!(read("Home").await.unwrap_err(), Error::Refused);
}
