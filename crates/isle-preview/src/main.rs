//! `isle-preview [--light] [--watch] <surface>`: a shell surface in a window of its own, with live
//! tokens (B7). `--watch` rebuilds it when the workspace's sources change and starts it again.

use std::{path::PathBuf, sync::mpsc, time::Duration};

use freya::prelude::*;
use isle_tokens::{Appearance, Mode, Source};
use notify::{RecursiveMode, Watcher};

fn tokens_path() -> PathBuf {
    let config = std::env::var_os("ISLE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(|c| PathBuf::from(c).join("isle"))
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/isle")))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    config.join("tokens.kdl")
}

fn load(path: &std::path::Path) -> Source {
    match std::fs::read_to_string(path) {
        Ok(text) => match Source::with_user(&text, path) {
            Ok((source, _)) => source,
            Err(err) => {
                tracing::warn!("{err}; the shipped tokens stand");
                Source::shipped()
            }
        },
        Err(_) => Source::shipped(),
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,isle_preview=info,isle_shell=info".into()),
        )
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--watch") {
        watch(args.into_iter().filter(|a| a != "--watch").collect());
    }
    let surfaces = isle_shell::preview::surfaces();
    let names: Vec<&str> = surfaces.iter().map(|s| s.name()).collect();
    let Some(def) = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .and_then(|name| surfaces.iter().find(|s| s.name() == name))
        .cloned()
    else {
        eprintln!(
            "usage: isle-preview [--light] [--watch] <surface>; surfaces: {}",
            names.join(", ")
        );
        std::process::exit(2);
    };
    let appearance = Appearance {
        mode: if args.iter().any(|a| a == "--light") {
            Mode::Light
        } else {
            Mode::Dark
        },
        ..Default::default()
    };

    let path = tokens_path();
    let first = load(&path);
    let (sources, received) = futures_channel::mpsc::unbounded::<Source>();
    std::thread::spawn({
        let path = path.clone();
        move || {
            let (events, changes) = mpsc::channel();
            let mut watcher = notify::recommended_watcher(events).expect("a file watcher");
            let dir = path.parent().unwrap_or(std::path::Path::new(".")).to_owned();
            let _ = std::fs::create_dir_all(&dir);
            watcher
                .watch(&dir, RecursiveMode::NonRecursive)
                .expect("the config directory watched");
            for event in changes.into_iter().flatten() {
                let event: notify::Event = event;
                if event.kind.is_access() || !event.paths.iter().any(|p| p.ends_with("tokens.kdl")) {
                    continue;
                }
                // Editors write in several steps; read once they settle.
                std::thread::sleep(Duration::from_millis(100));
                if sources.unbounded_send(load(&path)).is_err() {
                    return;
                }
            }
        }
    });

    let size = def.geometry((1920, 1080).into()).size;
    let hosted = isle_shell::preview::Hosted::new(def, appearance, first, received);
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new_app(hosted)
                .with_size(size.w as f64, size.h as f64)
                .with_title("Isle preview"),
        ),
    );
}

/// Runs the preview as a child, and on a saved change to the workspace's sources rebuilds it and
/// starts the new one in place of the old; a failed build leaves the old one running.
fn watch(args: Vec<String>) -> ! {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let exe = std::env::current_exe().expect("the preview's own path");
    let start = || {
        std::process::Command::new(&exe)
            .args(&args)
            .spawn()
            .expect("the preview")
    };
    let mut child = start();
    let (events, changes) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(events).expect("a file watcher");
    for dir in ["crates", "data/tokens"] {
        watcher
            .watch(&root.join(dir), RecursiveMode::Recursive)
            .expect("the sources watched");
    }
    loop {
        let Ok(Ok(event)) = changes.recv() else {
            let _ = child.kill();
            let _ = child.wait();
            std::process::exit(1);
        };
        let event: notify::Event = event;
        let source = |p: &std::path::Path| {
            p.extension()
                .is_some_and(|e| e == "rs" || e == "toml" || e == "kdl")
        };
        if event.kind.is_access() || !event.paths.iter().any(|p| source(p)) {
            continue;
        }
        // Editors write in several steps; build once they settle.
        std::thread::sleep(Duration::from_millis(200));
        while changes.try_recv().is_ok() {}
        tracing::info!("rebuilding");
        let built = std::process::Command::new(env!("CARGO"))
            .args(["build", "-p", "isle-preview"])
            .current_dir(&root)
            .status();
        if built.is_ok_and(|s| s.success()) {
            let _ = child.kill();
            let _ = child.wait();
            child = start();
        } else {
            tracing::warn!("the build failed; the running preview stays");
        }
    }
}
