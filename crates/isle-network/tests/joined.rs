//! The network joined, read from a fake NetworkManager: found by the access point in use when
//! a stronger one has its name, and listed once however many saved connections share a name.

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
async fn the_joined_network_is_the_access_point_in_use_and_known_once() {
    let (address, pid) = private_bus();
    // One test in this binary, before any connection: the variable is set once.
    unsafe { std::env::set_var("DBUS_SYSTEM_BUS_ADDRESS", &address) };
    let ap = |ssid: &str, strength| Ap { ssid: ssid.into(), strength, secured: true };
    // The fake joins the first access point of a known name: here the weaker of two.
    let _nm = NetworkManager::start(
        &address,
        vec![ap("Home", 60), ap("Home", 90), ap("Café", 40)],
        vec![Saved::Wifi("Home".into()), Saved::Wifi("Home".into())],
    )
    .await
    .unwrap();

    let (publisher, mut rx) = isle_bus::snapshot::<isle_network::State>();
    tokio::spawn(isle_network::watch(publisher));
    let state = loop {
        rx.changed().await.unwrap();
        let snapshot = rx.borrow().clone();
        if snapshot.available && snapshot.state.wifi.is_some() && !snapshot.state.known.is_empty() {
            break snapshot.state;
        }
    };
    let wifi = state.wifi.unwrap();
    let current = wifi.current.expect("the joined network");
    assert_eq!((current.ssid.as_str(), current.strength), ("Home", 60));
    let others: Vec<&str> = wifi.networks.iter().map(|n| n.ssid.as_str()).collect();
    assert_eq!(others, ["Café"]);
    assert_eq!(state.known.len(), 1);
    assert_eq!(state.known[0].paths.len(), 2);
    assert_eq!(state.known[0].path, state.known[0].paths[0]);

    let _ = Command::new("kill").arg(pid.to_string()).status();
}
