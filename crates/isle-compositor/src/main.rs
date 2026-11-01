use std::io::IsTerminal;

use isle_compositor::backend::Kind;

fn main() {
    // Two malloc arenas, set before any thread starts, so what one thread frees another
    // reuses rather than each keeping its own.
    // SAFETY: mallopt only sets allocator parameters.
    unsafe {
        libc::mallopt(libc::M_ARENA_MAX, 2);
    }
    // No transparent huge pages for the compositor, whose sparse arenas they make resident;
    // children inherit it, and apps are given them back in `child_env::as_an_app` (B137).
    // SAFETY: the prctl only sets this process's flag.
    unsafe {
        libc::prctl(libc::PR_SET_THP_DISABLE, 1, 0, 0, 0);
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_ansi(std::io::stdout().is_terminal())
        .init();
    let nested =
        std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some();
    let mut kind = None;
    let mut greeter = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--headless" => kind = Some(Kind::Headless),
            "--winit" => kind = Some(Kind::Winit),
            "--udev" => kind = Some(Kind::Udev),
            "--greeter" => greeter = true,
            other => {
                eprintln!(
                    "isle-compositor: unknown option {other}; use --udev, --winit or --headless, \
                     and --greeter under greetd"
                );
                std::process::exit(2);
            }
        }
    }
    let kind = kind.unwrap_or(if nested { Kind::Winit } else { Kind::Udev });
    if let Err(err) = isle_compositor::run(kind, greeter) {
        tracing::error!("{err}");
        std::process::exit(1);
    }
}
