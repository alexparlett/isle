//! The reveal gate's check against the polkit fake: Keychain's own process as the subject,
//! asked interactively for Isle's action, and the answer as given.

use isle_fakes::polkit::{Answer, Polkit};
use isle_secrets::gate;

#[tokio::test(flavor = "multi_thread")]
async fn asks_for_its_own_process_and_takes_the_answer() {
    let bus = isle_fakes::Bus::start();
    let polkit = Polkit::start(&bus.address).await.unwrap();
    let conn = isle_fakes::connect(&bus.address).await.unwrap();

    polkit.answer(Answer::Refuse);
    assert!(!gate::ask(&conn).await.unwrap());
    polkit.answer(Answer::Allow);
    assert!(gate::ask(&conn).await.unwrap());

    let checks = polkit.checks();
    assert_eq!(checks.len(), 2);
    assert!(
        checks
            .iter()
            .all(|c| c.action == gate::ACTION && c.subject_kind == "unix-process" && c.interactive)
    );
}
