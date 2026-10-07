//! The system services' path to the loop: snapshots from a daemon, bursts, a daemon going and
//! returning, and resume from sleep.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_fakes::logind::Logind;
use isle_ipc::{island::IslandProxy, services::ServicesProxy};

async fn status(isle: &Isle) -> serde_json::Value {
    let text = ServicesProxy::new(&isle.bus)
        .await
        .unwrap()
        .status()
        .await
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

async fn login(isle: &Isle) -> serde_json::Value {
    status(isle).await["login"].clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemons_change_reaches_isle_services() {
    let isle = Isle::start("services-change").await;
    assert_eq!(login(&isle).await["available"], false);
    let mut logind = Logind::start(&isle.system_address()).await.unwrap();
    wait_until(|| async { login(&isle).await["available"] == true }).await;
    assert_eq!(login(&isle).await["state"]["locked_hint"], false);
    logind.set_locked_hint(true).await.unwrap();
    wait_until(|| async { login(&isle).await["state"]["locked_hint"] == true }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_burst_reaches_the_loop_once_a_frame_with_its_last_value() {
    let isle = Isle::start("services-burst").await;
    let mut logind = Logind::start(&isle.system_address()).await.unwrap();
    wait_until(|| async { login(&isle).await["available"] == true }).await;
    let before = login(&isle).await["applied"].as_u64().unwrap();
    let flips = 200;
    for i in 0..flips {
        logind.set_locked_hint(i % 2 == 0).await.unwrap();
    }
    // The last flip set it off.
    logind.set_locked_hint(true).await.unwrap();
    wait_until(|| async { login(&isle).await["state"]["locked_hint"] == true }).await;
    let applied = login(&isle).await["applied"].as_u64().unwrap() - before;
    assert!(
        applied < flips / 2,
        "{applied} snapshots reached the loop for {flips} changes"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_that_goes_is_unavailable_and_followed_again_when_it_returns() {
    let isle = Isle::start("services-vanish").await;
    let mut logind = Logind::start(&isle.system_address()).await.unwrap();
    wait_until(|| async { login(&isle).await["available"] == true }).await;
    logind.set_locked_hint(true).await.unwrap();
    wait_until(|| async { login(&isle).await["state"]["locked_hint"] == true }).await;

    let quiet = isle.log().len();
    logind.vanish().await;
    wait_until(|| async { login(&isle).await["available"] == false }).await;
    // The last state stays to be shown greyed.
    assert_eq!(login(&isle).await["state"]["locked_hint"], true);

    logind.restore().await.unwrap();
    wait_until(|| async { login(&isle).await["available"] == true }).await;
    logind.set_locked_hint(false).await.unwrap();
    wait_until(|| async { login(&isle).await["state"]["locked_hint"] == false }).await;
    let stalls = isle.job_stalls_since(quiet);
    assert!(stalls.is_empty(), "{stalls:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_resume_from_sleep_reads_the_clock_again() {
    let isle = Isle::start("services-resume").await;
    let mut logind = Logind::start(&isle.system_address()).await.unwrap();
    wait_until(|| async { login(&isle).await["available"] == true }).await;
    let island = IslandProxy::new(&isle.bus).await.unwrap();
    let resumes = || async {
        let status: serde_json::Value =
            serde_json::from_str(&island.status().await.unwrap()).unwrap();
        status["resumes"].as_u64().unwrap()
    };
    assert_eq!(resumes().await, 0);
    logind.prepare_for_sleep(true).await.unwrap();
    wait_until(|| async { login(&isle).await["state"]["preparing_for_sleep"] == true }).await;
    assert_eq!(resumes().await, 0);
    logind.prepare_for_sleep(false).await.unwrap();
    wait_until(|| async { resumes().await == 1 }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_gallery_draws_the_new_controls_and_rows() {
    let isle = Isle::start("services-gallery").await;
    let shell = isle_ipc::shell::ShellProxy::new(&isle.bus).await.unwrap();
    assert!(shell.open("gallery").await.unwrap());
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    let shot = isle.shot("HEADLESS-1", "gallery").await;
    if let Ok(dir) = std::env::var("ISLE_SHOT_DIR") {
        std::fs::copy(isle.dir.join("gallery.png"), format!("{dir}/gallery.png")).unwrap();
    }
    // The switch that is on is filled with the accent.
    let accent = [0x2f, 0x6f, 0xe4];
    let found = (0..shot.height)
        .flat_map(|y| (0..shot.width).map(move |x| (x, y)))
        .any(|(x, y)| {
            let p = shot.at(x, y);
            (0..3).all(|i| (p[i] as i32 - accent[i] as i32).abs() < 12)
        });
    assert!(found, "no accent anywhere in the gallery");
}
