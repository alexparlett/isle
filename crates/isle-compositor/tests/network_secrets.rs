//! NetworkManager's secret agent: a secured network's password asked in Isle's prompt, asked
//! again when wrong; an 802.1X sign-in; a VPN's fields from its plugin's external UI. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::collections::HashMap;

use common::{Isle, wait_until};
use isle_fakes::network::{Ap, NetworkManager};
use zbus::zvariant::{OwnedValue, Str, Value};

const RETURN: u32 = 28;

/// Evdev codes for the letters typed in these tests.
fn code(c: char) -> u32 {
    match c {
        'a' => 30,
        'd' => 32,
        'e' => 18,
        'g' => 34,
        'i' => 23,
        'n' => 49,
        'o' => 24,
        'p' => 25,
        'r' => 19,
        's' => 31,
        'w' => 17,
        'x' => 45,
        _ => panic!("no code for {c}"),
    }
}

async fn type_text(isle: &Isle, text: &str) {
    let test = isle.test().await;
    for c in text.chars() {
        test.key(code(c), true).await.unwrap();
        test.key(code(c), false).await.unwrap();
    }
    test.key(RETURN, true).await.unwrap();
    test.key(RETURN, false).await.unwrap();
}

async fn prompts(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.prompts().await.unwrap()).unwrap()
}

async fn armed(isle: &Isle) -> serde_json::Value {
    wait_until(|| async { prompts(isle).await["armed"] == true }).await;
    prompts(isle).await
}

fn office() -> Vec<Ap> {
    vec![Ap {
        ssid: "Office".into(),
        strength: 60,
        secured: true,
    }]
}

async fn office_row(isle: &Isle) -> String {
    isle.rows()
        .await
        .iter()
        .find(|r| r["label"] == "Other networks")
        .and_then(|g| {
            g["rows"]
                .as_array()?
                .iter()
                .find(|r| r["label"] == "Office")
                .map(|r| r["id"].as_str().unwrap().to_owned())
        })
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_secured_network_asks_and_asks_again_when_wrong() {
    let isle = Isle::start("secrets-wifi").await;
    let nm = NetworkManager::start(&isle.system_address(), office(), vec![])
        .await
        .unwrap();
    wait_until(|| async { nm.agent_registered() }).await;
    isle.open_panel(Some("wifi")).await;
    isle.panel(serde_json::json!({ "expand": ["wifi", "other"] }))
        .await;
    wait_until(|| async {
        isle.rows()
            .await
            .iter()
            .any(|r| r["label"] == "Other networks" && r["open"] == true)
    })
    .await;
    let row = office_row(&isle).await;
    isle.panel(serde_json::json!({ "choose": ["wifi", row] }))
        .await;
    let shown = armed(&isle).await;
    assert_eq!(shown["title"], "“Office” needs a password", "{shown}");
    type_text(&isle, "wrong").await;
    wait_until(|| async {
        prompts(&isle).await["status"]
            .as_str()
            .is_some_and(|s| s.contains("Check the password"))
    })
    .await;
    armed(&isle).await;
    type_text(&isle, "password").await;
    wait_until(|| async { nm.current().as_deref() == Some("Office") }).await;
}

fn settings(
    id: &str,
    extra: Vec<(&str, Vec<(&str, OwnedValue)>)>,
) -> HashMap<String, HashMap<String, OwnedValue>> {
    let mut all = HashMap::new();
    all.insert(
        "connection".to_owned(),
        HashMap::from([
            ("id".to_owned(), OwnedValue::from(Str::from(id.to_owned()))),
            ("uuid".to_owned(), OwnedValue::from(Str::from("1234"))),
        ]),
    );
    for (setting, items) in extra {
        all.insert(
            setting.to_owned(),
            items.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
        );
    }
    all
}

#[tokio::test(flavor = "multi_thread")]
async fn an_enterprise_network_asks_for_its_sign_in() {
    let isle = Isle::start("secrets-8021x").await;
    let nm = NetworkManager::start(&isle.system_address(), vec![], vec![])
        .await
        .unwrap();
    wait_until(|| async { nm.agent_registered() }).await;
    let nm = std::sync::Arc::new(nm);
    let asking = nm.clone();
    let reply = tokio::spawn(async move {
        asking
            .ask_agent(
                settings(
                    "Campus",
                    vec![(
                        "802-11-wireless",
                        vec![(
                            "ssid",
                            OwnedValue::try_from(Value::from(b"Campus".to_vec())).unwrap(),
                        )],
                    )],
                ),
                "802-1x",
                vec!["identity".into(), "password".into()],
                0x1,
            )
            .await
    });
    let shown = armed(&isle).await;
    assert_eq!(shown["title"], "Sign in to “Campus”", "{shown}");
    // The user name first, then Tab to the password.
    let test = isle.test().await;
    for c in "sam".chars() {
        let k = match c {
            's' => 31,
            'a' => 30,
            _ => 50,
        };
        test.key(k, true).await.unwrap();
        test.key(k, false).await.unwrap();
    }
    test.key(15, true).await.unwrap();
    test.key(15, false).await.unwrap();
    type_text(&isle, "pass").await;
    let secrets = reply.await.unwrap().unwrap();
    let sign_in = &secrets["802-1x"];
    assert_eq!(
        String::try_from(sign_in["identity"].clone()).unwrap(),
        "sam"
    );
    assert_eq!(
        String::try_from(sign_in["password"].clone()).unwrap(),
        "pass"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_vpn_plugins_fields_are_asked_in_isles_prompt() {
    let isle = Isle::start_env(
        "secrets-vpn",
        "",
        |dir| {
            let plugins = dir.join("vpn");
            std::fs::create_dir_all(&plugins).unwrap();
            let dialog = plugins.join("auth-dialog");
            std::fs::write(
                &dialog,
                "#!/bin/sh\n\
                 while read -r line; do [ \"$line\" = DONE ] && break; done\n\
                 printf '[VPN Plugin UI]\\nVersion=2\\nDescription=Sign in to the test VPN.\\nTitle=Authenticate\\n\\n\
                 [password]\\nValue=\\nLabel=Password:\\nIsSecret=true\\nShouldAsk=true\\n\\n\
                 [username]\\nValue=sam\\nLabel=User name:\\nIsSecret=false\\nShouldAsk=false\\n'\n",
            )
            .unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dialog, std::fs::Permissions::from_mode(0o755)).unwrap();
            std::fs::write(
                plugins.join("test.name"),
                format!(
                    "[VPN Connection]\nname=test\nservice=org.isle.test.vpn\n\n[GNOME]\nauth-dialog={}\nsupports-external-ui-mode=true\n",
                    dialog.display()
                ),
            )
            .unwrap();
        },
        |dir| vec![("NM_VPN_PLUGIN_DIR".into(), dir.join("vpn").display().to_string())],
    )
    .await;
    let nm = NetworkManager::start(&isle.system_address(), vec![], vec![])
        .await
        .unwrap();
    wait_until(|| async { nm.agent_registered() }).await;
    let nm = std::sync::Arc::new(nm);
    let asking = nm.clone();
    let reply = tokio::spawn(async move {
        asking
            .ask_agent(
                settings(
                    "Work",
                    vec![(
                        "vpn",
                        vec![(
                            "service-type",
                            OwnedValue::from(Str::from("org.isle.test.vpn")),
                        )],
                    )],
                ),
                "vpn",
                vec![],
                0x1,
            )
            .await
    });
    let shown = armed(&isle).await;
    assert_eq!(shown["title"], "“Work” needs a password", "{shown}");
    type_text(&isle, "pass").await;
    let secrets = reply.await.unwrap().unwrap();
    let vpn = HashMap::<String, String>::try_from(secrets["vpn"]["secrets"].clone()).unwrap();
    assert_eq!(vpn["password"], "pass");
    assert_eq!(
        vpn["username"], "sam",
        "a value the plugin gave goes straight back"
    );
}
