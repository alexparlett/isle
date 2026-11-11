//! Wake for network access when NetworkManager refuses a profile's secrets: the others are
//! still set and the refusal is returned.

use std::process::Command;

use isle_fakes::network::{NetworkManager, Saved};

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

#[tokio::test(flavor = "multi_thread")]
async fn a_refused_profile_does_not_stop_the_others() {
    let (address, pid) = private_bus();
    // One test in this binary, before any connection: the variable is set once.
    unsafe { std::env::set_var("DBUS_SYSTEM_BUS_ADDRESS", &address) };
    let nm = NetworkManager::start(
        &address,
        Vec::new(),
        vec![Saved::Wifi("Home".into()), Saved::Wired("Desk".into())],
    )
    .await
    .unwrap();
    nm.refuse_secrets(true);

    assert!(isle_network::set_wake(true).await.is_err());
    assert_eq!(
        nm.wake(0),
        0x1,
        "a profile whose secrets are refused is not updated without them"
    );
    assert_eq!(nm.wake(1), 0x40);

    let _ = Command::new("kill").arg(pid.to_string()).status();
}
