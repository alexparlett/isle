//! Wake for network access against a fake NetworkManager on a private system bus: wired and
//! Wi-Fi profiles set to wake on a magic packet and back, a profile already so left alone, a
//! profile added seen by its path, and secrets kept through the update.

use std::{process::Command, time::Duration};

use isle_bus::Snapshot;
use isle_fakes::network::{Ap, NetworkManager, Saved};
use isle_network::{State, WakeProfile};
use tokio::sync::watch::Receiver;
use zbus::zvariant::{OwnedValue, Value};

fn private_bus() -> (String, i32) {
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
    (
        lines.next().unwrap().to_owned(),
        lines.next().unwrap().trim().parse().unwrap(),
    )
}

async fn until(rx: &mut Receiver<Snapshot<State>>, ok: impl Fn(&State) -> bool) -> State {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            {
                let snapshot = rx.borrow_and_update();
                if snapshot.available && ok(&snapshot.state) {
                    return snapshot.state.clone();
                }
            }
            rx.changed().await.unwrap();
        }
    })
    .await
    .expect("the state in time")
}

fn wakes(state: &State) -> Vec<bool> {
    state.wake.profiles.iter().map(|p| p.wakes).collect()
}

fn path(i: usize) -> String {
    format!("/org/freedesktop/NetworkManager/Settings/{i}")
}

#[tokio::test(flavor = "multi_thread")]
async fn wired_and_wifi_profiles_wake_on_a_magic_packet_and_go_back() {
    let (address, pid) = private_bus();
    // One test in this binary, before any connection: the variable is set once.
    unsafe { std::env::set_var("DBUS_SYSTEM_BUS_ADDRESS", &address) };
    let nm = NetworkManager::start(
        &address,
        vec![Ap {
            ssid: "Home".into(),
            strength: 80,
            secured: true,
        }],
        vec![
            Saved::Wired("Desk".into()),
            Saved::Wifi("Home".into()),
            Saved::Wifi("Café".into()),
            Saved::Vpn("Work".into()),
        ],
    )
    .await
    .unwrap();
    nm.set_wake(2, 0x8);

    let (publisher, mut rx) = isle_bus::snapshot::<State>();
    tokio::spawn(isle_network::watch(publisher));
    let state = until(&mut rx, |s| s.wake.profiles.len() == 3).await;
    assert_eq!(
        state.wake.profiles,
        [
            WakeProfile {
                path: path(0),
                wakes: false
            },
            WakeProfile {
                path: path(1),
                wakes: false
            },
            WakeProfile {
                path: path(2),
                wakes: true
            },
        ],
        "wired and Wi-Fi profiles, no VPN"
    );

    // On: magic on the wire and on Wi-Fi; the profile already so is not updated.
    isle_network::set_wake(true).await.unwrap();
    assert_eq!((nm.wake(0), nm.wake(1), nm.wake(2)), (0x40, 0x8, 0x8));
    assert!(
        nm.updated(2).is_none(),
        "a profile already waking is left alone"
    );
    assert!(nm.updated(3).is_none(), "a VPN is not touched");
    assert_eq!(nm.updates(), 2);
    let wifi = nm.updated(1).unwrap();
    let text = |s: &str| OwnedValue::try_from(Value::from(s)).unwrap();
    assert_eq!(
        wifi["802-11-wireless-security"]["psk"],
        text("password"),
        "the password goes back with the change"
    );
    assert_eq!(
        wifi["802-11-wireless"]["ssid"],
        OwnedValue::try_from(Value::from(b"Home".to_vec())).unwrap()
    );
    assert_eq!(
        nm.updated(0).unwrap()["connection"]["id"],
        text("Desk"),
        "the rest of the profile kept"
    );
    until(&mut rx, |s| wakes(s) == [true, true, true]).await;

    // A profile added is a path not seen before; applying again updates only it.
    let added = nm.add(Saved::Wired("Dock".into())).await.unwrap();
    let state = until(&mut rx, |s| s.wake.profiles.len() == 4).await;
    assert_eq!(
        state.wake.profiles[3],
        WakeProfile {
            path: path(added),
            wakes: false
        }
    );
    isle_network::set_wake(true).await.unwrap();
    assert_eq!(nm.wake(added), 0x40);
    assert_eq!(nm.updates(), 3);

    // Off: every profile back to the default, and again changes nothing.
    isle_network::set_wake(false).await.unwrap();
    assert_eq!(
        (nm.wake(0), nm.wake(1), nm.wake(2), nm.wake(added)),
        (0x1, 0x1, 0x1, 0x1)
    );
    assert_eq!(nm.updates(), 7);
    until(&mut rx, |s| wakes(s) == [false, false, false, false]).await;
    isle_network::set_wake(false).await.unwrap();
    assert_eq!(nm.updates(), 7, "a profile at the default is left alone");

    let _ = Command::new("kill").arg(pid.to_string()).status();
}
