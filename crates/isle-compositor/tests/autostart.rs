#![cfg(feature = "test-hooks")]

mod common;

use std::fs;

use common::Isle;

#[tokio::test(flavor = "multi_thread")]
async fn login_items_start_as_the_generator_would_choose_them() {
    let client = env!("CARGO_BIN_EXE_isle-test-client");
    let entry = |extra: &str| {
        format!(
            "[Desktop Entry]\nType=Application\nName=Item\nExec={client} --app-id isle.test.Item --colour 22aa44\n{extra}"
        )
    };
    let isle = Isle::start_env(
        "autostart",
        "",
        |dir| {
            let (user, system) = (
                dir.join("xdg-config/autostart"),
                dir.join("xdg-system/autostart"),
            );
            fs::create_dir_all(&user).unwrap();
            fs::create_dir_all(&system).unwrap();
            fs::write(
                system.join("isle.test.Item.desktop"),
                entry("NoDisplay=true\n"),
            )
            .unwrap();
            fs::write(
                system.join("gnome-only.desktop"),
                entry("OnlyShowIn=GNOME;\n"),
            )
            .unwrap();
            fs::write(system.join("turned-off.desktop"), entry("")).unwrap();
            fs::write(user.join("turned-off.desktop"), entry("Hidden=true\n")).unwrap();
        },
        |dir| {
            vec![
                (
                    "XDG_CONFIG_HOME".into(),
                    dir.join("xdg-config").display().to_string(),
                ),
                (
                    "XDG_CONFIG_DIRS".into(),
                    dir.join("xdg-system").display().to_string(),
                ),
            ]
        },
    )
    .await;
    let started = isle.test().await.login_items().await.unwrap();
    assert_eq!(started, ["isle.test.Item"]);
    isle.wait_for_window("isle.test.Item").await;
}
