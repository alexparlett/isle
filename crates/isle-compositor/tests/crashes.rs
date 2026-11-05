//! Apps that crash: core dump records read from a fake journal, the notice only for an app
//! that is gone, once a minute, its Reopen and its Report…. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::{fs, io::Write, path::Path, time::Duration};

use common::{Isle, wait_until};
use isle_ipc::{desktops::DesktopsProxy, notifications::NotificationsProxy};

fn entry(dir: &Path, id: &str, name: &str) {
    let client = env!("CARGO_BIN_EXE_isle-test-client");
    fs::write(
        dir.join(format!("data/applications/{id}.desktop")),
        format!("[Desktop Entry]\nType=Application\nName={name}\nExec={client} --app-id {id}\n"),
    )
    .unwrap();
}

/// Appends a core dump of `pid` to the fake journal, as `systemd-coredump` records one.
fn dump(isle: &Isle, pid: i32) {
    let uid = unsafe { libc::getuid() };
    let line = serde_json::json!({
        "MESSAGE_ID": "fc2e22bc6ee647b6b90729ab34a250b1",
        "COREDUMP_UID": uid.to_string(),
        "COREDUMP_PID": pid.to_string(),
        "COREDUMP_CGROUP": "/user.slice/elsewhere.scope",
        "COREDUMP_COMM": "isle-test-clien",
        "COREDUMP_SIGNAL_NAME": "SIGSEGV",
        "MESSAGE": "Process dumped core.\n\nStack trace of thread 1:\n#0 0x0 main",
    });
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(isle.dir.join("journal"))
        .unwrap();
    writeln!(file, "{line}").unwrap();
}

async fn notices(isle: &Isle) -> Vec<serde_json::Value> {
    let text = NotificationsProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["entries"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|e| {
            e["summary"]
                .as_str()
                .is_some_and(|s| s.ends_with("quit unexpectedly."))
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crashed_app_is_noticed_once_and_reopens_where_it_was() {
    let mut isle = Isle::start_env(
        "crashes",
        "",
        |dir| {
            fs::write(dir.join("journal"), "").unwrap();
            entry(dir, "isle.test.Crashy", "Crashy");
            entry(dir, "isle.test.Helper", "Helper");
        },
        |dir| {
            vec![
                (
                    "ISLE_CRASH_JOURNAL".into(),
                    dir.join("journal").display().to_string(),
                ),
                ("ISLE_RESTORE".into(), "manual".into()),
                (
                    "XDG_CONFIG_DIRS".into(),
                    dir.join("xdg-none").display().to_string(),
                ),
            ]
        },
    )
    .await;
    isle.test().await.restore_now().await.unwrap();
    let desktops = DesktopsProxy::new(&isle.bus).await.unwrap();
    assert_eq!(desktops.add("").await.unwrap(), 2);

    // A helper of a running app dumps core: its windows stay, and nothing is said.
    let helper = isle.client("isle.test.Helper", "22aa44");
    helper.wait_for("keyboard-enter").await;
    dump(&isle, helper.pid());
    tokio::time::sleep(Duration::from_millis(1600)).await;
    assert!(notices(&isle).await.is_empty());

    let crashy = isle.client("isle.test.Crashy", "3366ff");
    crashy.wait_for("keyboard-enter").await;
    let window = isle.wait_for_window("isle.test.Crashy").await;
    assert!(desktops.move_window(window.id, 2).await.unwrap());
    tokio::time::sleep(Duration::from_millis(100)).await;
    unsafe { libc::kill(crashy.pid(), libc::SIGKILL) };
    wait_until(|| async { crashy.exited() }).await;
    let recorded = std::time::Instant::now();
    dump(&isle, crashy.pid());
    wait_until(|| async { notices(&isle).await.len() == 1 }).await;
    assert!(
        recorded.elapsed() < Duration::from_secs(2),
        "within two seconds of the record"
    );
    let notice = notices(&isle).await.remove(0);
    assert_eq!(notice["summary"], "Crashy quit unexpectedly.");
    let keys: Vec<&str> = notice["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a[1].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["Reopen", "Report…"]);

    // Once a minute.
    dump(&isle, crashy.pid());
    tokio::time::sleep(Duration::from_millis(1600)).await;
    assert_eq!(notices(&isle).await.len(), 1);

    // Reopen brings its window back on the desktop it left.
    let id = notice["id"].as_u64().unwrap() as u32;
    let proxy = NotificationsProxy::new(&isle.bus).await.unwrap();
    desktops.switch("", 1).await.unwrap();
    assert!(proxy.invoke(id, "default").await.unwrap());
    let back = isle.wait_for_window("isle.test.Crashy").await;
    assert_eq!(back.desktop, 2);

    // Report… writes the record's summary and stack.
    unsafe { libc::kill(helper.pid(), libc::SIGKILL) };
    wait_until(|| async { helper.exited() }).await;
    dump(&isle, helper.pid());
    wait_until(|| async {
        notices(&isle)
            .await
            .iter()
            .any(|n| n["summary"] == "Helper quit unexpectedly.")
    })
    .await;
    let notice = notices(&isle)
        .await
        .into_iter()
        .find(|n| n["summary"] == "Helper quit unexpectedly.")
        .unwrap();
    assert!(
        proxy
            .invoke(notice["id"].as_u64().unwrap() as u32, "report")
            .await
            .unwrap()
    );
    let reports = isle.dir.join("state/isle/crashes/apps");
    wait_until(|| async {
        fs::read_dir(&reports).is_ok_and(|mut d| {
            d.next().is_some_and(|e| {
                fs::read_to_string(e.unwrap().path()).is_ok_and(|t| {
                    t.contains("Helper quit unexpectedly.") && t.contains("#0 0x0 main")
                })
            })
        })
    })
    .await;
}
