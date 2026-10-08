//! The lock's password through `isle-auth` and real PAM, on a service of the test's own that
//! accepts one password and counts failures with `pam_faillock`. Needs `--features
//! test-hooks` and `isle-auth` built beside the compositor.
#![cfg(feature = "test-hooks")]

mod common;

use std::path::{Path, PathBuf};

use common::{Isle, wait_until};
use isle_ipc::session::SessionProxy;

const RETURN: u32 = 28;
const I: u32 = 23;
const S: u32 = 31;
const L: u32 = 38;
const E: u32 = 18;
const X: u32 = 45;

fn helper() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_isle-compositor")).with_file_name("isle-auth");
    if !path.exists() {
        eprintln!("skipped: build isle-auth (cargo test --workspace)");
        return None;
    }
    Some(path)
}

/// `isle` is the password; three failures lock the account out for ten minutes.
fn pam_service(dir: &Path) {
    let pam = dir.join("pam");
    std::fs::create_dir_all(pam.join("tally")).unwrap();
    let check = pam.join("check.sh");
    std::fs::write(
        &check,
        "#!/bin/sh\npw=$(cat | tr -d '\\000')\n[ \"$pw\" = isle ]\n",
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&check, std::fs::Permissions::from_mode(0o755)).unwrap();
    let tally = pam.join("tally");
    let faillock = |kind: &str| {
        format!(
            "pam_faillock.so {kind} dir={} deny=3 unlock_time=600",
            tally.display()
        )
    };
    std::fs::write(
        pam.join("isle"),
        format!(
            "auth required {}\n\
             auth [success=1 default=ignore] pam_exec.so expose_authtok quiet {}\n\
             auth [default=die] {}\n\
             auth required {}\n\
             account required pam_permit.so\n",
            faillock("preauth"),
            check.display(),
            faillock("authfail"),
            faillock("authsucc"),
        ),
    )
    .unwrap();
}

async fn start(name: &str) -> Option<Isle> {
    let helper = helper()?;
    Some(
        Isle::start_env(name, "", pam_service, |dir| {
            vec![
                (
                    "ISLE_PAM_CONFDIR".into(),
                    dir.join("pam").display().to_string(),
                ),
                ("ISLE_AUTH_HELPER".into(), helper.display().to_string()),
            ]
        })
        .await,
    )
}

async fn status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.lock_status().await.unwrap()).unwrap()
}

async fn type_keys(isle: &Isle, keys: &[u32]) {
    let test = isle.test().await;
    for &key in keys {
        test.key(key, true).await.unwrap();
        test.key(key, false).await.unwrap();
    }
}

async fn lock(isle: &Isle) {
    SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .lock()
        .await
        .unwrap();
    wait_until(|| async { status(isle).await["locked"] == true }).await;
}

/// The helper the compositor started, by its parent.
fn helper_pid(isle: &Isle) -> Option<i32> {
    let parent = isle.compositor_pid();
    std::fs::read_dir("/proc")
        .ok()?
        .filter_map(Result::ok)
        .find_map(|entry| {
            let pid: i32 = entry.file_name().to_str()?.parse().ok()?;
            let stat = std::fs::read_to_string(entry.path().join("stat")).ok()?;
            let (name, rest) = stat.split_once(") ")?;
            let ppid: i32 = rest.split_whitespace().nth(1)?.parse().ok()?;
            (ppid == parent && name.ends_with("(isle-auth")).then_some(pid)
        })
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wrong_password_counts_and_the_right_one_unlocks() {
    let Some(mut isle) = start("auth-unlock").await else {
        return;
    };
    let window = isle.client("isle.test.Behind", "3366ff");
    window.wait_for("keyboard-enter").await;
    lock(&isle).await;

    type_keys(&isle, &[X, X]).await;
    assert_eq!(status(&isle).await["dots"], 2);
    type_keys(&isle, &[RETURN]).await;
    wait_until(|| async { status(&isle).await["status"] == "wrong" }).await;
    let after = status(&isle).await;
    assert_eq!(after["failures"], 1, "{after}");
    assert_eq!(
        after["dots"], 0,
        "the field is cleared after a wrong password"
    );
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.keep_shot("lock-wrong").await;

    window.clear();
    type_keys(&isle, &[I, S, L, E, RETURN]).await;
    wait_until(|| async { status(&isle).await["locked"] == false }).await;
    window.wait_for("keyboard-enter").await;
    assert!(!window.printed("key 23"), "a typed key reached the window");
    assert!(helper_pid(&isle).is_none(), "the helper outlived the lock");
}

#[tokio::test(flavor = "multi_thread")]
async fn three_failures_lock_the_account_out_and_typing_waits() {
    let Some(isle) = start("auth-lockout").await else {
        return;
    };
    lock(&isle).await;
    for failures in 1..=3 {
        type_keys(&isle, &[X, RETURN]).await;
        wait_until(|| async { status(&isle).await["failures"] == failures }).await;
    }
    type_keys(&isle, &[X, RETURN]).await;
    wait_until(|| async { status(&isle).await["status"] == "locked-out" }).await;
    let out = status(&isle).await;
    assert_eq!(out["minutes"], 10, "{out}");
    type_keys(&isle, &[I, S, L, E]).await;
    assert_eq!(
        status(&isle).await["dots"],
        0,
        "the field takes nothing while locked out"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_killed_helper_is_an_error_and_a_new_one_serves_the_next_attempt() {
    let Some(isle) = start("auth-killed").await else {
        return;
    };
    lock(&isle).await;
    type_keys(&isle, &[X, RETURN]).await;
    wait_until(|| async { status(&isle).await["status"] == "wrong" }).await;
    let pid = helper_pid(&isle).expect("the helper runs while locked");
    unsafe { libc::kill(pid, libc::SIGKILL) };
    type_keys(&isle, &[X, RETURN]).await;
    wait_until(|| async { status(&isle).await["status"] == "error" }).await;
    type_keys(&isle, &[I, S, L, E, RETURN]).await;
    wait_until(|| async { status(&isle).await["locked"] == false }).await;
}
