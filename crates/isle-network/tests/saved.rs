//! Saved networks against a fake NetworkManager on a private system bus: listed with their
//! auto-join, forgotten, set not to join with their password kept, and a device's details.

use std::process::Command;

use isle_fakes::network::{Ap, NetworkManager, Saved};

fn private_bus() -> (String, i32) {
    let out = Command::new("dbus-daemon")
        .args(["--session", "--fork", "--nopidfile", "--print-address=1", "--print-pid=1"])
        .output()
        .expect("dbus-daemon");
    let text = String::from_utf8(out.stdout).unwrap();
    let mut lines = text.lines();
    (lines.next().unwrap().to_owned(), lines.next().unwrap().trim().parse().unwrap())
}

#[tokio::test(flavor = "multi_thread")]
async fn saved_networks_are_listed_forgotten_and_set_not_to_join() {
    let (address, pid) = private_bus();
    // One test in this binary, before any connection: the variable is set once.
    unsafe { std::env::set_var("DBUS_SYSTEM_BUS_ADDRESS", &address) };
    let nm = NetworkManager::start(
        &address,
        vec![Ap { ssid: "Home".into(), strength: 80, secured: true }],
        vec![Saved::Wifi("Home".into()), Saved::Wifi("Office".into()), Saved::Vpn("Work".into())],
    )
    .await
    .unwrap();

    let (publisher, mut rx) = isle_bus::snapshot::<isle_network::State>();
    tokio::spawn(isle_network::watch(publisher));
    let state = loop {
        rx.changed().await.unwrap();
        let snapshot = rx.borrow().clone();
        if snapshot.available && !snapshot.state.known.is_empty() {
            break snapshot.state;
        }
    };
    let names: Vec<&str> = state.known.iter().map(|k| k.ssid.as_str()).collect();
    assert_eq!(names, ["Home", "Office"]);
    assert!(state.known.iter().all(|k| k.auto_join));

    let office = state.known.iter().find(|k| k.ssid == "Office").unwrap().path.clone();
    isle_network::set_auto_join(&office, false).await.unwrap();
    let update = nm.updated(1).expect("an update");
    assert_eq!(
        update["802-11-wireless-security"]["psk"],
        zbus::zvariant::OwnedValue::try_from(zbus::zvariant::Value::from("password")).unwrap(),
        "the password goes back with the change"
    );

    isle_network::forget(&office).await.unwrap();
    assert!(nm.deleted(1));

    let details = isle_network::details("/org/freedesktop/NetworkManager/Devices/2").await.unwrap();
    assert_eq!(details.address.as_deref(), Some("192.168.1.20"));
    assert_eq!(details.subnet.as_deref(), Some("255.255.255.0"));
    assert_eq!(details.router.as_deref(), Some("192.168.1.1"));
    assert_eq!(details.dns, ["192.168.1.1"]);
    assert_eq!(details.hardware.as_deref(), Some("aa:bb:cc:dd:ee:02"));

    let _ = Command::new("kill").arg(pid.to_string()).status();
}
