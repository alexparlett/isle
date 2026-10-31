//! Scans a path, or All disks without one, and prints its totals, kinds and cost:
//! `cargo run --release -p isle-scan --example scan -- ~`.

use std::{path::PathBuf, time::Instant};

use isle_scan::{Kind, Scan, TOP};

fn gb(bytes: u64) -> String {
    format!("{:.2} GB", bytes as f64 / 1e9)
}

fn status_kb(field: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with(field))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

fn main() {
    let roots: Vec<PathBuf> = match std::env::args().nth(1) {
        Some(p) => vec![PathBuf::from(p)],
        None => isle_scan::all_disks(&isle_fs::mounts::read())
            .into_iter()
            .map(|d| d.point)
            .collect(),
    };
    let base = status_kb("VmRSS:");
    let start = Instant::now();
    let scan = Scan::start(roots.clone());
    let (tree, progress) = scan.finish();
    let took = start.elapsed();
    let top = tree.node(TOP);
    let kinds = tree.kinds(TOP);
    println!("roots     {roots:?}");
    println!("took      {took:.2?}");
    println!(
        "scanned   {} in {} files, {} folders, used {:?}",
        gb(top.bytes),
        top.files,
        tree.nodes.len() - 1,
        progress.used.map(gb)
    );
    println!("before    {} MB", base / 1024);
    println!(
        "rss       {} MB, peak {} MB",
        status_kb("VmRSS:") / 1024,
        status_kb("VmHWM:") / 1024
    );
    for kind in Kind::ALL {
        println!("  {:<26} {}", kind.name(), gb(kinds[kind.index()]));
    }
    println!("unreadable {}", tree.unreadable().count());
    println!("largest:");
    for f in tree.largest.sorted().iter().take(5) {
        println!("  {:>10}  {}", gb(f.bytes), tree.file_path(f).display());
    }
    println!(
        "untouched {} kept, candidates {}",
        tree.untouched.sorted().len(),
        tree.candidates.len()
    );
    let start = Instant::now();
    let libraries = isle_games::Roots::find().steam_libraries();
    let finds = isle_scan::Rules::load().find(&tree, &isle_scan::Context::here(libraries));
    println!("rules     {:.2?}, {} finds", start.elapsed(), finds.len());
    for safe in [true, false] {
        let mut rows: Vec<_> = finds.iter().filter(|f| f.safe == safe).collect();
        rows.sort_by_key(|f| std::cmp::Reverse(f.bytes));
        let total: u64 = rows.iter().map(|f| f.bytes).sum();
        println!(
            "{} {}",
            if safe {
                "Safe to clear"
            } else {
                "Worth a look"
            },
            gb(total)
        );
        for f in rows.iter().take(12) {
            println!(
                "  {:>9}  {:<34} {}  ({})",
                gb(f.bytes),
                f.name,
                f.path.display(),
                f.why
            );
        }
    }
}
