//! Measurements for P3's budgets, run by hand on the host:
//! `cargo test -p isle-compositor --features test-hooks --test bench -- --ignored --nocapture`.
#![cfg(feature = "test-hooks")]

use std::{
    fs,
    time::{Duration, Instant},
};

use isle_ipc::{SurfaceInfo, shell::ShellProxy, version::VersionProxy};

mod common;

use common::{Isle, wait_until};

/// RSS and PSS in MB, from `smaps_rollup`.
fn memory(pid: u32) -> (f64, f64) {
    let text = fs::read_to_string(format!("/proc/{pid}/smaps_rollup")).unwrap_or_default();
    let field = |name: &str| {
        text.lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0)
            / 1024.0
    };
    (field("Rss:"), field("Pss:"))
}

/// The process's memory on the GPU in MiB, from nvidia-smi's process table, which lists
/// graphics processes where its compute query does not.
fn vram(pid: u32) -> Option<u64> {
    let out = std::process::Command::new("nvidia-smi").output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.split_whitespace().any(|w| w == pid.to_string()))
        .and_then(|l| {
            l.split_whitespace()
                .find_map(|w| w.strip_suffix("MiB").and_then(|n| n.parse().ok()))
        })
}

async fn surface(isle: &Isle, name: &str) -> SurfaceInfo {
    ShellProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap()
}

async fn tap(isle: &Isle, code: u32) {
    let test = isle.test().await;
    test.key(code, true).await.unwrap();
    test.key(code, false).await.unwrap();
}

/// A 3440 by 1440 output as the only, and so main, one; returns where it is in the global space.
async fn ultrawide(isle: &Isle) -> (f64, f64) {
    let test = isle.test().await;
    test.add_output("BIG", "Isle", "Bench", "1", 3440, 1440)
        .await
        .unwrap();
    test.remove_output("HEADLESS-1").await.unwrap();
    wait_until(|| async { surface(isle, "probe-full").await.output == "BIG" }).await;
    let big = isle_ipc::displays::DisplaysProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
        .into_iter()
        .find(|d| d.name == "BIG")
        .unwrap();
    (big.x as f64, big.y as f64)
}

fn report(what: &str, s: &SurfaceInfo) {
    eprintln!(
        "{what}: {} draws, p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms",
        s.draws,
        s.draw_p50 as f64 / 1000.0,
        s.draw_p95 as f64 / 1000.0,
        s.draw_max as f64 / 1000.0
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn full_output_draw_and_memory() {
    let isle = Isle::start("bench").await;
    let pid = VersionProxy::new(&isle.bus).await.unwrap().pid().await.unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    let test = isle.test().await;
    ultrawide(&isle).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (rss, pss) = memory(pid);
    eprintln!("at rest, nothing open: RSS {rss:.1} MB, PSS {pss:.1} MB, VRAM {:?} MiB", vram(pid));

    test.measure(true).await.unwrap();
    shell.open("probe-full").await.unwrap();
    tokio::time::sleep(Duration::from_millis(600)).await;
    for code in [46, 24, 50, 49, 14, 14, 14, 14] {
        tap(&isle, code).await;
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
    let typing = surface(&isle, "probe-full").await;
    report("3440x1440, opening and typing", &typing);

    test.measure(true).await.unwrap();
    test.animate_probe_full(true).await.unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    let animating = surface(&isle, "probe-full").await;
    test.animate_probe_full(false).await.unwrap();
    report("3440x1440, 24 tiles fading", &animating);
    let (used, limit) = shell.cache().await.unwrap();
    eprintln!("Skia cache {:.1} of {:.1} MB", used as f64 / 1e6, limit as f64 / 1e6);
    let (rss, pss) = memory(pid);
    eprintln!("probe-full open: RSS {rss:.1} MB, PSS {pss:.1} MB, VRAM {:?} MiB", vram(pid));

    shell.close("probe-full").await.unwrap();
    shell.open("probe").await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (rss, pss) = memory(pid);
    eprintln!("probe open at rest: RSS {rss:.1} MB, PSS {pss:.1} MB, VRAM {:?} MiB", vram(pid));
    shell.close("probe").await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (rss, pss) = memory(pid);
    eprintln!("all closed again: RSS {rss:.1} MB, PSS {pss:.1} MB, VRAM {:?} MiB", vram(pid));

    assert!(typing.draw_p95 < 2000, "acceptance 5: p95 under 2 ms");
}

/// Opens, types in, animates, relayouts and closes both probes for `ISLE_SOAK_SECS` (an hour by
/// default), sampling memory each minute.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn soak() {
    let secs: u64 = std::env::var("ISLE_SOAK_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600);
    let isle = Isle::start("soak").await;
    let pid = VersionProxy::new(&isle.bus).await.unwrap().pid().await.unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    let test = isle.test().await;
    let (ox, oy) = ultrawide(&isle).await;
    let started = Instant::now();
    let mut samples: Vec<(u64, f64, f64, Option<u64>)> = Vec::new();
    let mut next = Instant::now();
    let mut cycles = 0u64;
    while started.elapsed() < Duration::from_secs(secs) {
        shell.open("probe").await.unwrap();
        for code in [30, 48, 46] {
            tap(&isle, code).await;
        }
        // The probe's Animate button: the bar animating relayouts the probe every frame.
        let (x, y) = (
            ox + (3440.0 - 480.0) / 2.0 + 150.0,
            oy + (1440.0 - 360.0) / 2.0 + 66.0,
        );
        test.pointer_to(x, y).await.unwrap();
        test.button(272, true).await.unwrap();
        test.button(272, false).await.unwrap();
        if cycles == 0 {
            wait_until(|| async { isle.test().await.probe().await.unwrap().2 }).await;
        }
        tokio::time::sleep(Duration::from_millis(700)).await;
        test.button(272, true).await.unwrap();
        test.button(272, false).await.unwrap();
        shell.close("probe").await.unwrap();
        shell.open("probe-full").await.unwrap();
        test.animate_probe_full(true).await.unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
        test.animate_probe_full(false).await.unwrap();
        shell.close("probe-full").await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        cycles += 1;
        if Instant::now() >= next {
            let (rss, pss) = memory(pid);
            samples.push((started.elapsed().as_secs(), rss, pss, vram(pid)));
            eprintln!(
                "{:>5} s, {cycles} cycles: RSS {rss:.1} MB, PSS {pss:.1} MB, VRAM {:?} MiB",
                started.elapsed().as_secs(),
                vram(pid)
            );
            next += Duration::from_secs(60);
        }
    }
    let first = samples.get(1).or(samples.first()).unwrap();
    let last = samples.last().unwrap();
    eprintln!("first minute RSS {:.1} MB, last {:.1} MB over {cycles} cycles", first.1, last.1);
    assert!(last.1 < first.1 * 1.2 + 20.0, "RSS grew from {:.1} to {:.1} MB", first.1, last.1);
}
