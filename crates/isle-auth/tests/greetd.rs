//! The greeter's greetd client against a fake greetd: PAM's conversations as greetd relays
//! them, and the session started.

use isle_auth::greetd::{Greetd, Step};
use isle_fakes::greetd::{Greetd as Fake, Say, User};

fn socket(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("isle-greetd-{}-{name}.sock", std::process::id()))
}

fn ask(prompt: &str, secret: bool, messages: &[&str]) -> Step {
    Step::Ask {
        prompt: prompt.into(),
        secret,
        messages: messages.iter().map(|m| m.to_string()).collect(),
    }
}

#[tokio::test]
async fn a_password_signs_in_and_starts_the_session() {
    let fake = Fake::start(&socket("plain"), vec![User::new("alex", "hunter2")]);
    let mut greetd = Greetd::connect(&fake.socket).await.unwrap();
    assert_eq!(
        greetd.create("alex").await.unwrap(),
        ask("Password: ", true, &[])
    );
    assert_eq!(
        greetd.answer(Some("hunter2")).await.unwrap(),
        Step::Ready { messages: vec![] }
    );
    let cmd = vec!["isle-session".to_owned()];
    let env = vec!["XDG_CURRENT_DESKTOP=Isle".to_owned()];
    assert_eq!(greetd.start(&cmd, &env).await.unwrap(), Ok(()));
    let started = fake.started();
    assert_eq!(started.len(), 1);
    assert_eq!(
        (started[0].user.as_str(), &started[0].cmd, &started[0].env),
        ("alex", &cmd, &env)
    );
    assert_eq!(fake.answers(), 1);
}

#[tokio::test]
async fn a_wrong_password_is_refused_and_the_next_try_starts_afresh() {
    let fake = Fake::start(&socket("wrong"), vec![User::new("alex", "hunter2")]);
    let mut greetd = Greetd::connect(&fake.socket).await.unwrap();
    greetd.create("alex").await.unwrap();
    let refused = greetd.answer(Some("nope")).await.unwrap();
    assert!(matches!(refused, Step::Refused { .. }), "{refused:?}");
    assert_eq!(refused.lockout(), None);
    // Refused, the session is cancelled, so the next create is not "already configured".
    assert_eq!(
        greetd.create("alex").await.unwrap(),
        ask("Password: ", true, &[])
    );
    assert!(matches!(
        greetd.answer(Some("hunter2")).await.unwrap(),
        Step::Ready { .. }
    ));
    assert!(fake.requests().contains(&"cancel_session".to_owned()));
}

#[tokio::test]
async fn info_lines_are_kept_and_a_second_question_is_asked() {
    let mut user = User::new("alex", "old");
    user.script = vec![
        Say::Info("Welcome".into()),
        Say::Password,
        Say::Error("Your password has expired".into()),
        Say::Secret("New password: ".into()),
        Say::Visible("Token: ".into()),
    ];
    let fake = Fake::start(&socket("expired"), vec![user]);
    let mut greetd = Greetd::connect(&fake.socket).await.unwrap();
    assert_eq!(
        greetd.create("alex").await.unwrap(),
        ask("Password: ", true, &["Welcome"])
    );
    assert_eq!(
        greetd.answer(Some("old")).await.unwrap(),
        ask("New password: ", true, &["Your password has expired"])
    );
    assert_eq!(
        greetd.answer(Some("new")).await.unwrap(),
        ask("Token: ", false, &[])
    );
    assert!(matches!(
        greetd.answer(Some("123")).await.unwrap(),
        Step::Ready { .. }
    ));
}

#[tokio::test]
async fn faillocks_messages_read_as_a_lockout() {
    let mut user = User::new("alex", "hunter2");
    user.on_wrong = vec![
        Say::Error("The account is locked due to 3 failed logins.".into()),
        Say::Info("(5 minutes left to unlock)".into()),
    ];
    let fake = Fake::start(&socket("faillock"), vec![user]);
    let mut greetd = Greetd::connect(&fake.socket).await.unwrap();
    greetd.create("alex").await.unwrap();
    let refused = greetd.answer(Some("nope")).await.unwrap();
    assert_eq!(refused.lockout(), Some(5));
}

#[tokio::test]
async fn a_session_cancelled_midway_can_be_begun_for_another_user() {
    let fake = Fake::start(
        &socket("cancel"),
        vec![User::new("alex", "a"), User::new("sam", "s")],
    );
    let mut greetd = Greetd::connect(&fake.socket).await.unwrap();
    greetd.create("alex").await.unwrap();
    greetd.cancel().await.unwrap();
    greetd.create("sam").await.unwrap();
    assert!(matches!(
        greetd.answer(Some("s")).await.unwrap(),
        Step::Ready { .. }
    ));
}

#[tokio::test]
async fn starting_before_signing_in_is_greetds_error() {
    let fake = Fake::start(&socket("early"), vec![User::new("alex", "a")]);
    let mut greetd = Greetd::connect(&fake.socket).await.unwrap();
    greetd.create("alex").await.unwrap();
    assert!(greetd.start(&["x".into()], &[]).await.unwrap().is_err());
    assert!(fake.started().is_empty());
}
