fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let options = isle_compositor::Options {
        watchdog_abort: std::env::var("ISLE_WATCHDOG_ABORT").as_deref() != Ok("0"),
    };
    if let Err(err) = isle_compositor::run(options) {
        tracing::error!("{err}");
        std::process::exit(1);
    }
}
