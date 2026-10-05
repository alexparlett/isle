#![cfg(feature = "test-hooks")]

mod common;

use std::fs;

use common::{Isle, wait_until};

#[tokio::test(flavor = "multi_thread")]
async fn an_entry_and_its_action_start_in_front_with_their_token() {
    let isle = Isle::start("launch").await;
    let client = env!("CARGO_BIN_EXE_isle-test-client");
    fs::write(
        isle.dir.join("data/applications/isle.test.Launched.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Launched\nExec={client} --app-id isle.test.Launched --colour 22aa44 %U\nActions=other;\n\n[Desktop Action other]\nName=Other\nExec={client} --app-id isle.test.Other --colour 4422aa\n"
        ),
    )
    .unwrap();
    let windows = isle.windows_proxy().await;
    wait_until(|| async { windows.launch("isle.test.Launched", "", &[]).await.unwrap() }).await;
    let launched = isle.wait_for_window("isle.test.Launched").await;
    wait_until(|| async { isle.window(launched.id).await.focused }).await;

    assert!(
        windows
            .launch("isle.test.Launched", "other", &[])
            .await
            .unwrap()
    );
    let other = isle.wait_for_window("isle.test.Other").await;
    wait_until(|| async { isle.window(other.id).await.focused }).await;

    assert!(!windows.launch("isle.test.Nothing", "", &[]).await.unwrap());
    assert!(
        !windows
            .launch("isle.test.Launched", "missing", &[])
            .await
            .unwrap()
    );
    assert!(isle.log().contains("launching"), "{}", isle.log());
}
