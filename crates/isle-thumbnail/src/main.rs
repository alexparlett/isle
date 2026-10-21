//! `isle-thumbnail`: Files' helper. Reads asks as JSON lines on standard input, makes each
//! thumbnail into the shared cache on one of four threads, and answers as JSON lines; a file
//! that crashes it costs Files only a restart of the helper.

use std::{
    io::{BufRead, Write},
    sync::{Arc, Mutex, mpsc},
};

use isle_thumbnail::{Answer, Ask, cache, make};

const THREADS: usize = 4;

fn answer(out: &Mutex<std::io::Stdout>, answer: &Answer) {
    let mut out = out.lock().unwrap();
    let _ = serde_json::to_writer(&mut *out, answer);
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}

fn handle(ask: Ask, thumbnailers: &[make::Thumbnailer]) -> Answer {
    let root = cache::root();
    let pixels = ask.flavour.pixels();
    let target = cache::path(&root, &ask.path, ask.flavour);
    if let Some(found) = cache::find(&root, &ask.path, ask.flavour, ask.mtime, ask.size) {
        return Answer { id: ask.id, thumbnail: Some(found) };
    }
    if cache::failed(&root, &ask.path, ask.mtime) {
        return Answer { id: ask.id, thumbnail: None };
    }
    let made = std::panic::catch_unwind(|| make::make(&ask.path, &ask.mime, pixels, thumbnailers)).ok().flatten();
    let thumbnail = match made {
        Some(rgba) => cache::write(&root, &target, &ask.path, &rgba, ask.mtime, ask.size, &ask.mime)
            .ok()
            .map(|_| target),
        None => {
            let _ = cache::write_failed(&root, &ask.path, ask.mtime);
            None
        }
    };
    Answer { id: ask.id, thumbnail }
}

/// `isle-thumbnail page FILE N PIXELS OUT`: one PDF page as a PNG, for Quick Look; prints the
/// page count. A page that crashes it costs Quick Look only that page.
fn page(args: &[String]) -> std::process::ExitCode {
    let [file, n, pixels, out] = args else { return std::process::ExitCode::FAILURE };
    let path = std::path::Path::new(file);
    let (Ok(n), Ok(pixels)) = (n.parse::<usize>(), pixels.parse::<u32>()) else {
        return std::process::ExitCode::FAILURE;
    };
    let Some(count) = make::pdf_pages(path) else { return std::process::ExitCode::FAILURE };
    match make::pdf_page(path, n, pixels).map(|img| img.save(out)) {
        Some(Ok(())) => {
            println!("{count}");
            std::process::ExitCode::SUCCESS
        }
        _ => std::process::ExitCode::FAILURE,
    }
}

/// `isle-thumbnail frames FILE COUNT HEIGHT PREFIX`: frames across a video, each written to
/// `PREFIX-N.png`, for Quick Look's trim strip; prints the length in seconds.
fn frames(args: &[String]) -> std::process::ExitCode {
    let [file, count, height, prefix] = args else { return std::process::ExitCode::FAILURE };
    let (Ok(count), Ok(height)) = (count.parse::<u32>(), height.parse::<u32>()) else {
        return std::process::ExitCode::FAILURE;
    };
    let probed = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", file])
        .output();
    let Some(length) = probed.ok().and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f64>().ok()) else {
        return std::process::ExitCode::FAILURE;
    };
    println!("{length}");
    for n in 0..count {
        let at = length * (n as f64 + 0.5) / count as f64;
        let _ = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-ss", &format!("{at:.3}"), "-i", file, "-frames:v", "1"])
            .args(["-vf", &format!("scale=-2:{height}"), "-y", &format!("{prefix}-{n}.png")])
            .status();
    }
    std::process::ExitCode::SUCCESS
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("page") {
        return page(&args[1..]);
    }
    if args.first().map(String::as_str) == Some("frames") {
        return frames(&args[1..]);
    }
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();
    let thumbnailers = Arc::new(make::thumbnailers());
    let out = Arc::new(Mutex::new(std::io::stdout()));
    let (tx, rx) = mpsc::channel::<Ask>();
    let rx = Arc::new(Mutex::new(rx));
    let workers: Vec<_> = (0..THREADS)
        .map(|_| {
            let (rx, out, thumbnailers) = (rx.clone(), out.clone(), thumbnailers.clone());
            std::thread::spawn(move || loop {
                let next = rx.lock().unwrap().recv();
                let Ok(ask) = next else { return };
                answer(&out, &handle(ask, &thumbnailers));
            })
        })
        .collect();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        match serde_json::from_str::<Ask>(&line) {
            Ok(ask) => {
                if tx.send(ask).is_err() {
                    break;
                }
            }
            Err(err) => tracing::warn!("not an ask: {err}"),
        }
    }
    drop(tx);
    for worker in workers {
        let _ = worker.join();
    }
    std::process::ExitCode::SUCCESS
}
