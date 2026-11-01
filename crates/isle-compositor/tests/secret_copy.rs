//! A password copied the way Keychain copies it: kept out of history, cleared from the
//! clipboard when its source is withdrawn, a later copy by another client left alone, and
//! the island's line from `Shell1.Copied`. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};

#[tokio::test(flavor = "multi_thread")]
async fn a_withdrawn_secret_leaves_the_clipboard_and_never_history() {
    let mut isle = Isle::start("secret-copy").await;
    let a = isle.client("isle.test.Keychain", "3366ff");
    a.wait_for("keyboard-enter").await;
    a.send("copy-secret hunter2");
    a.wait_for("copied").await;
    a.clear();
    a.send("paste");
    a.wait_for("pasted hunter2").await;

    a.clear();
    a.send("withdraw");
    a.wait_for("withdrawn").await;
    a.wait_for("selection none").await;
    a.clear();
    a.send("paste");
    a.wait_for("pasted nothing").await;
    let index = isle.dir.join("state/isle/clipboard/index.json");
    assert!(
        !std::fs::read_to_string(&index)
            .unwrap_or_default()
            .contains("hunter2")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn withdrawing_leaves_a_later_copy_alone() {
    let mut isle = Isle::start("secret-copy-later").await;
    let a = isle.client("isle.test.Keychain", "3366ff");
    a.wait_for("keyboard-enter").await;
    a.send("copy-secret hunter2");
    a.wait_for("copied").await;

    let b = isle.client("isle.test.Editor", "33ff66");
    b.wait_for("keyboard-enter").await;
    b.send("copy mine");
    b.wait_for("copied").await;
    a.send("withdraw");
    a.wait_for("withdrawn").await;
    b.clear();
    b.send("paste");
    b.wait_for("pasted mine").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_apps_password_copy_posts_its_line() {
    let isle = Isle::start("secret-copy-island").await;
    let shell = isle_ipc::shell::ShellProxy::new(&isle.bus).await.unwrap();
    shell.copied("password").await.unwrap();
    wait_until(|| async { isle.island_output().await["text"] == "Password copied" }).await;
}
