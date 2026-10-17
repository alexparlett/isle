//! A device renamed and forgotten, and the adapter's own name, against a fake BlueZ on a
//! private system bus.

use std::process::Command;

use isle_fakes::bluez::{BlueZ, Device};

fn private_bus() -> (String, i32) {
    let out = Command::new("dbus-daemon")
        .args(["--session", "--fork", "--nopidfile", "--print-address=1", "--print-pid=1"])
        .output()
        .expect("dbus-daemon");
    let text = String::from_utf8(out.stdout).unwrap();
    let mut lines = text.lines();
    (lines.next().unwrap().to_owned(), lines.next().unwrap().trim().parse().unwrap())
}

async fn state(rx: &mut tokio::sync::watch::Receiver<isle_bus::Snapshot<isle_bluetooth::Bluetooth>>, done: impl Fn(&isle_bluetooth::Bluetooth) -> bool) -> isle_bluetooth::Bluetooth {
    loop {
        {
            let snapshot = rx.borrow();
            if snapshot.available && done(&snapshot.state) {
                return snapshot.state.clone();
            }
        }
        tokio::time::timeout(std::time::Duration::from_secs(5), rx.changed())
            .await
            .expect("a change")
            .unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_device_is_renamed_and_forgotten() {
    let (address, pid) = private_bus();
    // One test in this binary, before any connection: the variable is set once.
    unsafe { std::env::set_var("DBUS_SYSTEM_BUS_ADDRESS", &address) };
    let _bluez = BlueZ::start(
        &address,
        vec![Device {
            id: "00_11_22_33_44_55".into(),
            name: "MX Master".into(),
            icon: "input-mouse".into(),
            paired: true,
            connected: true,
            battery: Some(80),
        }],
    )
    .await
    .unwrap();
    let (publisher, mut rx) = isle_bus::snapshot::<isle_bluetooth::Bluetooth>();
    tokio::spawn(isle_bluetooth::watch(publisher));
    let now = state(&mut rx, |s| !s.devices.is_empty()).await;
    let adapter = now.adapter.clone().unwrap();
    assert_eq!(adapter.name, "Isle Test");
    let mouse = now.devices[0].path.clone();

    isle_bluetooth::rename(&mouse, "Desk Mouse").await.unwrap();
    state(&mut rx, |s| s.devices.first().is_some_and(|d| d.name == "Desk Mouse")).await;

    isle_bluetooth::forget(&adapter.path, &mouse).await.unwrap();
    state(&mut rx, |s| s.devices.is_empty()).await;
    let _ = Command::new("kill").arg(pid.to_string()).status();
}
