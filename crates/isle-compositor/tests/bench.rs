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

/// RSS, and the process's own memory (anonymous pages, resident or swapped), in MB, from
/// `smaps_rollup`. RSS rises and falls as a busy host evicts and swaps pages; the second is what
/// grows if the process leaks.
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
    (field("Rss:"), field("Anonymous:") + field("Swap:"))
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
    let pid = VersionProxy::new(&isle.bus)
        .await
        .unwrap()
        .pid()
        .await
        .unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    let test = isle.test().await;
    ultrawide(&isle).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (rss, own) = memory(pid);
    eprintln!(
        "at rest, nothing open: RSS {rss:.1} MB, own {own:.1} MB, VRAM {:?} MiB",
        vram(pid)
    );

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
    eprintln!(
        "Skia cache {:.1} of {:.1} MB",
        used as f64 / 1e6,
        limit as f64 / 1e6
    );
    let (rss, own) = memory(pid);
    eprintln!(
        "probe-full open: RSS {rss:.1} MB, own {own:.1} MB, VRAM {:?} MiB",
        vram(pid)
    );

    shell.close("probe-full").await.unwrap();
    shell.open("probe").await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (rss, own) = memory(pid);
    eprintln!(
        "probe open at rest: RSS {rss:.1} MB, own {own:.1} MB, VRAM {:?} MiB",
        vram(pid)
    );
    shell.close("probe").await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (rss, own) = memory(pid);
    eprintln!(
        "all closed again: RSS {rss:.1} MB, own {own:.1} MB, VRAM {:?} MiB",
        vram(pid)
    );

    assert!(typing.draw_p95 < 2000, "acceptance 5: p95 under 2 ms");
}

/// The probe's glass blurring again as a window behind it comes and goes, at 3440x1440.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn glass_blur_cost() {
    let mut isle = Isle::start("bench-glass").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    let test = isle.test().await;
    ultrawide(&isle).await;
    let _base = isle.client_with(&[
        "--app-id",
        "isle.test.Base",
        "--colour",
        "3366ff",
        "--size",
        "1600x900",
    ]);
    isle.wait_for_window("isle.test.Base").await;
    shell.open("probe").await.unwrap();
    tokio::time::sleep(Duration::from_millis(600)).await;
    test.measure(true).await.unwrap();
    let windows = isle_ipc::windows::WindowsProxy::new(&isle.bus)
        .await
        .unwrap();
    for n in 0..20 {
        let app = format!("isle.test.Flash{n}");
        let _flash =
            isle.client_with(&["--app-id", &app, "--colour", "ff6600", "--size", "900x500"]);
        let window = isle.wait_for_window(&app).await;
        tokio::time::sleep(Duration::from_millis(80)).await;
        windows.close(window.id).await.unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
    let s = surface(&isle, "probe").await;
    eprintln!(
        "glass 480x80 with its margin at 3440x1440: {} blurs, p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms",
        s.glass_runs,
        s.glass_p50 as f64 / 1000.0,
        s.glass_p95 as f64 / 1000.0,
        s.glass_max as f64 / 1000.0
    );
    assert!(s.glass_p95 < 1000, "the plan's bar: under 1 ms");
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
    let pid = VersionProxy::new(&isle.bus)
        .await
        .unwrap()
        .pid()
        .await
        .unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    let test = isle.test().await;
    let (ox, oy) = ultrawide(&isle).await;
    let started = Instant::now();
    let mut samples: Vec<(u64, f64, f64, Option<u64>)> = Vec::new();
    let mut next = Instant::now();
    let mut cycles = 0u64;
    // `probe` or `full` soaks one surface alone, to find which one grows.
    let only = std::env::var("ISLE_SOAK_ONLY").unwrap_or_default();
    while started.elapsed() < Duration::from_secs(secs) {
        if only != "full" {
            shell.open("probe").await.unwrap();
            // Typed and deleted again: the probe keeps its field's text across opens, and a text
            // that grew every cycle would grow Freya's paragraph cache with it.
            for code in [30, 48, 46, 14, 14, 14] {
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
        }
        if only != "probe" {
            shell.open("probe-full").await.unwrap();
            test.animate_probe_full(true).await.unwrap();
            tokio::time::sleep(Duration::from_millis(500)).await;
            test.animate_probe_full(false).await.unwrap();
            shell.close("probe-full").await.unwrap();
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        cycles += 1;
        if Instant::now() >= next {
            let (rss, own) = memory(pid);
            samples.push((started.elapsed().as_secs(), own, rss, vram(pid)));
            eprintln!(
                "{:>5} s, {cycles} cycles: RSS {rss:.1} MB, own {own:.1} MB, VRAM {:?} MiB",
                started.elapsed().as_secs(),
                vram(pid)
            );
            eprintln!(
                "    allocated and not freed: {:.1} MB",
                test.malloc_in_use().await.unwrap() as f64 / 1e6
            );
            next += Duration::from_secs(60);
        }
    }
    // Judged on the process's own memory. The baseline is the highest of the run's first
    // quarter, once allocators have warmed up; the margin is shell.md acceptance 4's.
    let quarter = samples.len().div_ceil(4).max(1);
    let baseline = samples[..quarter].iter().map(|s| s.1).fold(0.0, f64::max);
    let last = samples.last().unwrap();
    eprintln!(
        "own memory: first quarter's highest {baseline:.1} MB, last {:.1} MB over {cycles} cycles",
        last.1
    );
    assert!(
        last.1 <= (baseline * 1.1).max(baseline + 20.0),
        "own memory grew from {baseline:.1} to {:.1} MB",
        last.1
    );
}

/// What a title bar per window costs: the compositor's memory and VRAM with thirty barred
/// windows against thirty without.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn title_bar_cost() {
    async fn open(isle: &mut Isle, bars: bool, n: usize) {
        for i in 0..n {
            let app = format!("isle.test.Cost{bars}{i}");
            let mut args = vec!["--app-id", app.as_str(), "--size", "600x400"];
            if bars {
                args.extend(["--do", "decoration server"]);
            }
            isle.client_with(&args);
            isle.wait_for_window(&app).await;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    for bars in [false, true] {
        let mut isle = Isle::start(if bars { "cost-bars" } else { "cost-plain" }).await;
        let pid = VersionProxy::new(&isle.bus)
            .await
            .unwrap()
            .pid()
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
        let test = isle.test().await;
        let (rss0, own0) = memory(pid);
        let heap0 = test.malloc_in_use().await.unwrap();
        open(&mut isle, bars, 30).await;
        let (rss, own) = memory(pid);
        let heap = test.malloc_in_use().await.unwrap();
        let cache = ShellProxy::new(&isle.bus)
            .await
            .unwrap()
            .cache()
            .await
            .unwrap();
        println!(
            "bars {bars}: RSS {rss0:.1} -> {rss:.1} MB, own {own0:.1} -> {own:.1} MB, heap +{:.1} MB, skia {:.1} MB, VRAM {:?} MiB",
            (heap - heap0) as f64 / 1e6,
            cache.0 as f64 / 1e6,
            vram(pid)
        );
    }
}

/// P6: the island at rest, Control Centre open and the menu bar down with a menu open, on
/// a 3440 by 1440 output: draw times and what each adds to memory and the GPU.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn island_and_menu_bar_cost() {
    const CTRL: u32 = 29;
    const F8: u32 = 66;
    const F2: u32 = 60;
    const ESC: u32 = 1;
    let mut isle = Isle::start("cost-island").await;
    let pid = VersionProxy::new(&isle.bus).await.unwrap().pid().await.unwrap();
    let test = isle.test().await;
    test.add_output("BIG", "Isle", "Bench", "1", 3440, 1440).await.unwrap();
    test.remove_output("HEADLESS-1").await.unwrap();
    let _client = isle.client_with(&["--app-id", "isle.test.Qt", "--size", "1600x900", "--appmenu"]);
    isle.wait_for_window("isle.test.Qt").await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    test.measure(true).await.unwrap();
    let chord = |keys: &'static [u32]| {
        let isle = &isle;
        async move {
            isle.chord(keys).await;
            tokio::time::sleep(Duration::from_millis(800)).await;
        }
    };
    let snap = |what: &str| {
        let (rss, own) = memory(pid);
        println!("{what}: RSS {rss:.0} MB, own {own:.0} MB, VRAM {:?} MiB", vram(pid));
    };
    snap("rest");
    for _ in 0..20 {
        chord(&[CTRL, F8]).await;
        chord(&[ESC]).await;
    }
    report("island with Control Centre, 20 opens", &surface(&isle, "island").await);
    chord(&[CTRL, F8]).await;
    snap("Control Centre open");
    chord(&[ESC]).await;
    for _ in 0..10 {
        chord(&[CTRL, F2]).await;
        chord(&[ESC]).await;
        chord(&[ESC]).await;
    }
    snap("after 10 reveals");
    report("menu bar, 10 reveals", &surface(&isle, "menu-bar").await);
    chord(&[CTRL, F2]).await;
    snap("menu bar down, system menu open");
    chord(&[ESC]).await;
    chord(&[ESC]).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    snap("rest again");
    assert!(!isle.log().contains("panicked"), "a surface panicked");
}

/// The switcher held over twenty apps on a 3440 by 1440 output, its window row open, while
/// every app redraws: the panel's draws, the previews' updates, and what it adds to memory
/// and the GPU.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn switcher_cost() {
    const CMD: u32 = 125;
    const TAB: u32 = 15;
    let mut isle = Isle::start("cost-switcher").await;
    let pid = VersionProxy::new(&isle.bus).await.unwrap().pid().await.unwrap();
    let test = isle.test().await;
    test.add_output("BIG", "Isle", "Bench", "1", 3440, 1440).await.unwrap();
    test.remove_output("HEADLESS-1").await.unwrap();
    let mut clients = Vec::new();
    for i in 0..20 {
        let app = format!("isle.test.App{i:02}");
        let size = if i % 3 == 0 { "3440x1400" } else { "1600x900" };
        clients.push(isle.client_with(&["--app-id", &app, "--size", size, "--colour", "3366ff"]));
        isle.wait_for_window(&app).await;
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    let snap = |what: &str| {
        let (rss, own) = memory(pid);
        println!("{what}: RSS {rss:.0} MB, own {own:.0} MB, VRAM {:?} MiB", vram(pid));
    };
    snap("rest, twenty apps");
    test.measure(true).await.unwrap();
    for round in 0..5 {
        test.key(CMD, true).await.unwrap();
        tap(&isle, TAB).await;
        // Previews are in the window row only.
        tap(&isle, 108).await;
        tokio::time::sleep(Duration::from_millis(400)).await;
        if round == 0 {
            snap("held");
        }
        for frame in 0..30 {
            for (i, c) in clients.iter().enumerate() {
                c.send(&format!("colour {:06x}", (frame * 8 + i * 12) % 0xff * 0x010101));
            }
            tokio::time::sleep(Duration::from_millis(16)).await;
        }
        test.key(CMD, false).await.unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    snap("released");
    report("switcher, five holds", &surface(&isle, "switcher").await);
    let mut updates = test.switcher_updates().await.unwrap();
    updates.sort_unstable();
    let at = |q: f64| updates[((updates.len() - 1) as f64 * q) as usize] as f64 / 1000.0;
    eprintln!(
        "previews: {} updates, p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms",
        updates.len(),
        at(0.5),
        at(0.95),
        at(1.0)
    );
    tokio::time::sleep(Duration::from_secs(2)).await;
    snap("rest again");
    assert!(!isle.log().contains("panicked"), "a surface panicked");
}

/// Isle's own child processes, by name: an idle session should have none but Xwayland.
fn children(pid: u32) -> Vec<String> {
    let Ok(dir) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    dir.flatten()
        .filter_map(|e| {
            let stat = fs::read_to_string(e.path().join("stat")).ok()?;
            let (name, rest) = stat.split_once(") ")?;
            let ppid: u32 = rest.split_whitespace().nth(1)?.parse().ok()?;
            (ppid == pid).then(|| name.split_once(" (").map_or(name, |(_, n)| n).to_owned())
        })
        .collect()
}

/// P8's measure on a 3440 by 1440 output with every service live: Control Centre's draw on
/// every page, banners' draw and their latency from Notify, memory, and what runs and draws
/// at rest.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn services_cost() {
    use isle_fakes::{bluez, logind::Logind, mpris::Mpris, network, upower};
    let mut isle = Isle::start("cost-services").await;
    let pid = VersionProxy::new(&isle.bus).await.unwrap().pid().await.unwrap();
    let system = isle.system_address();
    let _nm = network::NetworkManager::start(
        &system,
        vec![
            network::Ap { ssid: "Home".into(), strength: 80, secured: true },
            network::Ap { ssid: "Cafe".into(), strength: 50, secured: false },
        ],
        vec![network::Saved::Wifi("Home".into()), network::Saved::Vpn("Work".into())],
    )
    .await
    .unwrap();
    let _bluez = bluez::BlueZ::start(
        &system,
        vec![bluez::Device {
            id: "00_11_22_33_44_55".into(),
            name: "WH-1000XM6".into(),
            icon: "audio-headset".into(),
            paired: true,
            connected: true,
            battery: Some(70),
        }],
    )
    .await
    .unwrap();
    let _upower = upower::UPower::start(
        &system,
        vec![upower::Device {
            id: "mouse".into(),
            kind: 5,
            power_supply: false,
            model: "MX Master 3S".into(),
            percentage: 64.0,
            state: 2,
        }],
    )
    .await
    .unwrap();
    let _profiles = upower::PowerProfiles::start(&system).await.unwrap();
    let _logind = Logind::start(&system).await.unwrap();
    let _player = Mpris::start(&isle.bus_address(), "bench", true).await.unwrap();
    let audio = isle.start_audio().await;
    let test = isle.test().await;
    test.add_output("BIG", "Isle", "Bench", "1", 3440, 1440).await.unwrap();
    test.remove_output("HEADLESS-1").await.unwrap();
    test.fake_screen("BIG", 60, 100).await.unwrap();
    let app = isle.client_with(&["--app-id", "isle.test.Chat", "--size", "1600x900"]);
    app.wait_for("keyboard-enter").await;
    app.send("tray");
    app.wait_for("tray registered").await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let snap = |what: &str| {
        let (rss, own) = memory(pid);
        println!("{what}: RSS {rss:.0} MB, own {own:.0} MB, VRAM {:?} MiB", vram(pid));
    };
    println!("PipeWire: {audio}");
    snap("rest, every service live");

    test.measure(true).await.unwrap();
    isle.chord(&[29, 66]).await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let modules: Vec<String> = isle.island_output().await["panel"]["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap().to_owned())
        .collect();
    println!("modules: {modules:?}");
    snap("Control Centre open");
    for _ in 0..3 {
        for page in &modules {
            isle.panel(serde_json::json!({ "page": page })).await;
            tokio::time::sleep(Duration::from_millis(500)).await;
            isle.panel(serde_json::json!({ "page": null })).await;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }
    snap("after every page three times");
    report("island with Control Centre, every page", &surface(&isle, "island").await);
    isle.chord(&[1]).await;
    tokio::time::sleep(Duration::from_millis(800)).await;

    test.measure(false).await.unwrap();
    test.measure(true).await.unwrap();
    // The test client opens its bus on its first notification; that is not Isle's time.
    app.send("notify summary=Warm transient=true");
    app.wait_for("notified 1").await;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let mut latencies = Vec::new();
    for i in 0..10 {
        // One card per app, so an arrival shows as its summary on the card.
        let shown = |text: &str| {
            serde_json::from_str::<serde_json::Value>(text).unwrap()["cards"]
                .as_array()
                .is_some_and(|c| c.iter().any(|c| c["summary"] == format!("Message {i}")))
        };
        let sent = Instant::now();
        app.send(&format!("notify summary=Message_{i} body=Hello transient=true"));
        loop {
            let now = isle.test().await.banners().await.unwrap();
            if shown(&now) || sent.elapsed() > Duration::from_secs(2) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        latencies.push(sent.elapsed().as_secs_f64() * 1000.0);
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }
    println!("banner latencies in order, ms: {latencies:.1?}");
    latencies.sort_by(f64::total_cmp);
    println!(
        "banner latency from the client's Notify to the stack: p50 {:.1} ms, max {:.1} ms",
        latencies[latencies.len() / 2],
        latencies[latencies.len() - 1]
    );
    tokio::time::sleep(Duration::from_secs(8)).await;
    report("banners, ten arrivals", &surface(&isle, "banners").await);
    snap("after the banners");

    // At rest: nothing drawn, no helper running.
    test.measure(false).await.unwrap();
    let before: Vec<(String, u64)> = ShellProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|s| (s.name, s.draws))
        .collect();
    tokio::time::sleep(Duration::from_secs(10)).await;
    let after = ShellProxy::new(&isle.bus).await.unwrap().list().await.unwrap();
    for s in after {
        let was = before.iter().find(|(n, _)| *n == s.name).map_or(0, |(_, d)| *d);
        if s.draws != was {
            println!("at rest, {} drew {} times in 10 s", s.name, s.draws - was);
        }
    }
    println!("children at rest: {:?}", children(pid));
    snap("rest again");
    assert!(!isle.log().contains("panicked"), "a surface panicked");
}

/// P9's measures at 3440x1440 over the frost wallpaper: the time from a lock request to every
/// screen showing it, the lock's face and a prompt as they are typed in, memory and VRAM
/// locked, and what draws at rest locked and unlocked.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn lock_and_prompts_cost() {
    use isle_fakes::logind::Logind;
    use isle_ipc::session::SessionProxy;
    let mut isle = Isle::start_with("cost-lock", "wallpaper {\n    source \"frost\"\n}\n").await;
    let pid = VersionProxy::new(&isle.bus).await.unwrap().pid().await.unwrap();
    let _logind = Logind::start(&isle.system_address()).await.unwrap();
    let test = isle.test().await;
    let session = SessionProxy::new(&isle.bus).await.unwrap();
    ultrawide(&isle).await;
    let app = isle.client_with(&["--app-id", "isle.test.Work", "--size", "1600x900"]);
    app.wait_for("keyboard-enter").await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let snap = |what: &str| {
        let (rss, own) = memory(pid);
        println!("{what}: RSS {rss:.0} MB, own {own:.0} MB, VRAM {:?} MiB", vram(pid));
    };
    let lock = || async {
        serde_json::from_str::<serde_json::Value>(&test.lock_status().await.unwrap()).unwrap()
    };
    let draws = || async {
        ShellProxy::new(&isle.bus)
            .await
            .unwrap()
            .list()
            .await
            .unwrap()
            .into_iter()
            .map(|s| (s.name, s.draws))
            .collect::<Vec<_>>()
    };
    let at_rest = |what: &'static str, before: Vec<(String, u64)>| {
        let bus = isle.bus.clone();
        async move {
            let after = ShellProxy::new(&bus).await.unwrap().list().await.unwrap();
            let mut quiet = true;
            for s in after {
                let was = before.iter().find(|(n, _)| *n == s.name).map_or(0, |(_, d)| *d);
                if s.draws != was {
                    quiet = false;
                    println!("{what}: {} drew {} times in 10 s", s.name, s.draws - was);
                }
            }
            if quiet {
                println!("{what}: nothing drew in 10 s");
            }
        }
    };
    snap("rest, unlocked");

    let mut shown = Vec::new();
    for _ in 0..5 {
        session.lock().await.unwrap();
        wait_until(|| async { !lock().await["shown_ms"].is_null() }).await;
        shown.push(lock().await["shown_ms"].as_f64().unwrap());
        test.unlock().await.unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    println!("lock request to every screen locked, ms: {shown:.1?}");

    test.measure(true).await.unwrap();
    session.lock().await.unwrap();
    tokio::time::sleep(Duration::from_millis(800)).await;
    snap("locked");
    for code in [30, 48, 46, 32, 18, 33, 34, 35, 14, 14, 14, 14].repeat(3) {
        tap(&isle, code).await;
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
    report("the lock's face, typing", &surface(&isle, "lock").await);
    let face = surface(&isle, "lock").await;
    test.measure(false).await.unwrap();
    // The face dozes, then nothing but the clock's minute should draw.
    tokio::time::sleep(Duration::from_secs(12)).await;
    let before = draws().await;
    tokio::time::sleep(Duration::from_secs(10)).await;
    at_rest("locked, at rest", before).await;
    test.unlock().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    snap("unlocked again");

    test.measure(true).await.unwrap();
    test.prompt_ask(
        r#"{"title": "Isle needs your password", "message": "To install updates.", "fields": [{"id": "password", "label": "Password", "secret": true}], "action": "Allow"}"#,
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(900)).await;
    snap("a prompt showing");
    for code in [30, 48, 46, 32, 18, 33, 34, 35, 14, 14, 14, 14].repeat(3) {
        tap(&isle, code).await;
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
    let prompt = surface(&isle, "prompt").await;
    report("a prompt, typing", &prompt);
    tap(&isle, 1).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    test.measure(false).await.unwrap();
    let before = draws().await;
    tokio::time::sleep(Duration::from_secs(10)).await;
    at_rest("unlocked, at rest", before).await;
    snap("after");

    assert!(face.draw_p95 < 2000, "the face under 2 ms p95");
    assert!(prompt.draw_p95 < 2000, "a prompt under 2 ms p95");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn spotlight_cost() {
    // The host's own apps and icons, and a home of a few thousand files.
    let mut isle = Isle::start_prepared("cost-spotlight", "wallpaper {\n    source \"frost\"\n}\n", |dir| {
        std::os::unix::fs::symlink("/usr/share/icons", dir.join("data-system/icons")).unwrap();
        std::fs::create_dir_all(dir.join("data-system/applications")).unwrap();
        for entry in fs::read_dir("/usr/share/applications").unwrap().flatten() {
            let _ = fs::copy(entry.path(), dir.join("data-system/applications").join(entry.file_name()));
        }
        for d in 0..50 {
            let folder = dir.join(format!("home/projects/p{d}/src"));
            fs::create_dir_all(&folder).unwrap();
            for f in 0..100 {
                fs::write(folder.join(format!("module_{f}.rs")), format!("fn item_{f}() {{}}\n")).unwrap();
            }
        }
    })
    .await;
    let pid = VersionProxy::new(&isle.bus).await.unwrap().pid().await.unwrap();
    let test = isle.test().await;
    ultrawide(&isle).await;
    let app = isle.client_with(&["--app-id", "isle.test.Work", "--size", "1600x900"]);
    app.wait_for("keyboard-enter").await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    let snap = |what: &str| {
        let (rss, own) = memory(pid);
        println!("{what}: RSS {rss:.0} MB, own {own:.0} MB, VRAM {:?} MiB", vram(pid));
    };
    let status = || async { serde_json::from_str::<serde_json::Value>(&test.spotlight().await.unwrap()).unwrap() };
    let chord = |keys: &'static [u32]| {
        let test = test.clone();
        async move {
            for k in keys {
                test.key(*k, true).await.unwrap();
            }
            for k in keys.iter().rev() {
                test.key(*k, false).await.unwrap();
            }
        }
    };
    snap("rest");

    let draws = || async {
        ShellProxy::new(&isle.bus)
            .await
            .unwrap()
            .list()
            .await
            .unwrap()
            .into_iter()
            .find(|s| s.name == "spotlight")
            .map_or(0, |s| s.draws)
    };
    // The chord to Spotlight's first drawn frame, as the test sees it.
    let mut first = Vec::new();
    for _ in 0..5 {
        let before = draws().await;
        let sent = Instant::now();
        chord(&[125, 57]).await;
        wait_until(|| async { draws().await > before }).await;
        first.push(sent.elapsed().as_secs_f64() * 1000.0);
        chord(&[125, 57]).await;
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    println!("Cmd+Space to the first frame, ms: {first:.1?}");

    test.measure(true).await.unwrap();
    chord(&[125, 57]).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    snap("open, empty");
    // f i r e f o x, then away and back, three times: a key every 120 ms.
    for _ in 0..3 {
        for code in [33, 23, 19, 18, 33, 24, 45, 14, 14, 14, 14, 14, 14, 14] {
            tap(&isle, code).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }
    }
    let times: Vec<u64> = status().await["search_us"].as_array().unwrap().iter().filter_map(|v| v.as_u64()).collect();
    let mut sorted = times.clone();
    sorted.sort_unstable();
    let at = |q: f64| sorted[((sorted.len() - 1) as f64 * q) as usize] as f64 / 1000.0;
    println!("fast sources per key ({} searches): p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms", sorted.len(), at(0.5), at(0.95), at(1.0));
    report("the list, typing", &surface(&isle, "spotlight").await);
    snap("open, typing");
    chord(&[125, 57]).await;
    tokio::time::sleep(Duration::from_millis(400)).await;

    test.measure(false).await.unwrap();
    test.measure(true).await.unwrap();
    chord(&[125, 57]).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    chord(&[125, 2]).await;
    tokio::time::sleep(Duration::from_millis(600)).await;
    for code in [106, 106, 106, 108, 108, 108, 108, 105, 103, 103].repeat(3) {
        tap(&isle, code).await;
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
    report("the Apps view, arrows", &surface(&isle, "spotlight").await);
    snap("the Apps view");
    chord(&[125, 57]).await;
    tokio::time::sleep(Duration::from_millis(400)).await;

    test.measure(false).await.unwrap();
    test.measure(true).await.unwrap();
    chord(&[29, 125, 57]).await;
    tokio::time::sleep(Duration::from_millis(600)).await;
    snap("the emoji grid opened");
    for code in [108; 30] {
        tap(&isle, code).await;
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
    report("the emoji grid, down", &surface(&isle, "spotlight").await);
    snap("the emoji grid");
    chord(&[125, 57]).await;
    test.measure(false).await.unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;
    let before = draws().await;
    tokio::time::sleep(Duration::from_secs(10)).await;
    let after = draws().await;
    println!("closed, 10 s: Spotlight drew {} times", after - before);
    snap("closed again");
    drop(app);
    let _ = isle.compositor_exit();
}

/// CPU seconds a process has used, from `/proc/<pid>/stat`.
fn cpu_seconds(pid: u32) -> f64 {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    let fields: Vec<&str> = stat
        .rsplit_once(')')
        .map(|(_, rest)| rest.split_whitespace().collect())
        .unwrap_or_default();
    let ticks = |i: usize| fields.get(i).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    (ticks(11) + ticks(12)) / 100.0
}

fn percentiles(mut v: Vec<u64>) -> (f64, f64, f64) {
    if v.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    v.sort();
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize] as f64 / 1000.0;
    (at(0.5), at(0.95), at(1.0))
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn capture_cost() {
    let release = cfg!(debug_assertions).then_some("").unwrap_or("--release");
    let mut build = std::process::Command::new(env!("CARGO"));
    build.args(["build", "-q", "-p", "isle-record"]);
    if !release.is_empty() {
        build.arg(release);
    }
    assert!(build.status().unwrap().success());
    let profile = if release.is_empty() { "debug" } else { "release" };
    let helper = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../target/{profile}/isle-record"));
    let mut isle = Isle::start_env(
        "cost-capture",
        "wallpaper {\n    source \"frost\"\n}\n",
        |_| {},
        move |_| vec![("ISLE_RECORD_HELPER".into(), helper.display().to_string())],
    )
    .await;
    let pid = VersionProxy::new(&isle.bus).await.unwrap().pid().await.unwrap();
    let test = isle.test().await;
    ultrawide(&isle).await;
    let app = isle.client_with(&["--app-id", "isle.test.Work", "--size", "1600x900"]);
    app.wait_for("keyboard-enter").await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let snap = |what: &str| {
        let (rss, own) = memory(pid);
        println!("{what}: RSS {rss:.0} MB, own {own:.0} MB, VRAM {:?} MiB", vram(pid));
    };
    let chord = |keys: &'static [u32]| {
        let test = test.clone();
        async move {
            for k in keys {
                test.key(*k, true).await.unwrap();
            }
            for k in keys.iter().rev() {
                test.key(*k, false).await.unwrap();
            }
        }
    };
    snap("rest");
    let shots = isle.dir.join("home/Pictures/Screenshots");
    let count = || fs::read_dir(&shots).map(|d| d.count()).unwrap_or(0);

    // Shift+Cmd+3 to the file on disk.
    let mut written = Vec::new();
    for _ in 0..5 {
        let before = count();
        let sent = Instant::now();
        chord(&[42, 125, 4]).await;
        wait_until(|| async { count() > before }).await;
        written.push(sent.elapsed().as_secs_f64() * 1000.0);
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    println!("Shift+Cmd+3 to a written file, ms: {written:.1?}");
    tokio::time::sleep(Duration::from_secs(1)).await;
    snap("after five shots");

    // The freeze with one window, then with eight.
    let pick = || async { test.pick().await.unwrap() != "null" };
    let mut freezes = Vec::new();
    let mut extra = Vec::new();
    for n in [1usize, 8] {
        while extra.len() + 1 < n {
            let c = isle.client_with(&["--app-id", &format!("isle.test.W{}", extra.len()), "--size", "800x600"]);
            c.wait_for("keyboard-enter").await;
            extra.push(c);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        let mut times = Vec::new();
        for _ in 0..3 {
            let sent = Instant::now();
            chord(&[42, 125, 5]).await;
            wait_until(|| async { pick().await }).await;
            times.push(sent.elapsed().as_secs_f64() * 1000.0);
            snap(&format!("frozen, {n} windows"));
            tap(&isle, 1).await;
            wait_until(|| async { !pick().await }).await;
        }
        freezes.push((n, times));
    }
    println!("Shift+Cmd+4 to the frozen pick, ms: {freezes:.1?}");
    snap("after the picks");
    for c in &extra {
        c.send("close");
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    if !isle.start_audio().await {
        println!("no PipeWire: streams and recording not measured");
        return;
    }
    // A screen stream beside a client drawing every frame.
    app.send("animate");
    let frames = || async { test.frames("BIG").await.unwrap() };
    let f0 = frames().await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    let alone = (frames().await - f0) as f64 / 3.0;
    println!("output frames a second, the client animating alone: {alone:.1}");
    let id = test.cast_start(r#"{"screen": "BIG"}"#).await.unwrap();
    let node = loop {
        let casts: Vec<serde_json::Value> = serde_json::from_str(&test.casts().await.unwrap()).unwrap();
        if let Some(n) = casts.iter().find(|c| c["id"] == id).and_then(|c| c["node"].as_u64()) {
            break n as u32;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let consumer = isle.consumer(node);
    consumer.wait_for("format 3440x1440").await;
    test.measure(true).await.unwrap();
    let cpu0 = cpu_seconds(pid);
    let f0 = frames().await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    let streamed = (frames().await - f0) as f64 / 5.0;
    let cpu = (cpu_seconds(pid) - cpu0) / 5.0 * 100.0;
    let times = test.cast_times().await.unwrap();
    let got = consumer.lines.lock().unwrap().iter().filter(|l| l.starts_with("frame ")).count();
    let (p50, p95, max) = percentiles(times.clone());
    println!(
        "a 3440x1440 shared-memory stream: {} frames rendered, {got} received in all; the frame's render on the loop p50 {p50:.2} ms, p95 {p95:.2} ms, max {max:.2} ms; output frames a second {streamed:.1}; compositor CPU {cpu:.0}%",
        times.len()
    );
    snap("streaming");
    test.measure(false).await.unwrap();
    test.cast_stop(id).await.unwrap();

    // A recording beside the same client.
    test.record(r#"{"screen": "BIG"}"#).await.unwrap();
    let recorder = loop {
        let r: serde_json::Value = serde_json::from_str(&test.recording().await.unwrap()).unwrap();
        if let Some(p) = r["pid"].as_u64() {
            break p as u32;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (cpu0, rcpu0) = (cpu_seconds(pid), cpu_seconds(recorder));
    let f0 = frames().await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    let recorded = (frames().await - f0) as f64 / 5.0;
    let cpu = (cpu_seconds(pid) - cpu0) / 5.0 * 100.0;
    let rcpu = (cpu_seconds(recorder) - rcpu0) / 5.0 * 100.0;
    let (rrss, _) = memory(recorder);
    println!(
        "recording 3440x1440: output frames a second {recorded:.1}; compositor CPU {cpu:.0}%, recorder CPU {rcpu:.0}%, recorder RSS {rrss:.0} MB"
    );
    snap("recording");
    test.stop_recording().await.unwrap();
    wait_until(|| async { test.recording().await.unwrap() == "null" }).await;

    // Still: nothing streams and nothing draws.
    app.send("animate");
    let id = test.cast_start(r#"{"screen": "BIG"}"#).await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let count = |id: u64| {
        let test = test.clone();
        async move {
            let casts: Vec<serde_json::Value> = serde_json::from_str(&test.casts().await.unwrap()).unwrap();
            casts.iter().find(|c| c["id"] == id).map_or(0, |c| c["frames"].as_u64().unwrap())
        }
    };
    let (s0, f0) = (count(id).await, frames().await);
    tokio::time::sleep(Duration::from_secs(5)).await;
    println!(
        "still for 5 s with a stream open: {} stream frames, {} output frames",
        count(id).await - s0,
        frames().await - f0
    );
}
