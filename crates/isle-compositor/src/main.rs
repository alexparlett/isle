use isle_compositor::backend::Kind;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let kind = match std::env::args().nth(1).as_deref() {
        Some("--headless") => Kind::Headless,
        Some("--winit") | None => Kind::Winit,
        Some(other) => {
            eprintln!("isle-compositor: unknown option {other}; use --winit or --headless");
            std::process::exit(2);
        }
    };
    if let Err(err) = isle_compositor::run(kind) {
        tracing::error!("{err}");
        std::process::exit(1);
    }
}
