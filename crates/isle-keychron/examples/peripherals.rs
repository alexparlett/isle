//! Prints the Keychron peripherals as they change, for a number of seconds (default 20).

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let seconds = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let (publisher, mut snapshots) = isle_bus::snapshot();
    tokio::spawn(isle_keychron::watch(publisher));
    let end = tokio::time::Instant::now() + std::time::Duration::from_secs(seconds);
    while let Ok(Ok(())) = tokio::time::timeout_at(end, snapshots.changed()).await {
        for p in &snapshots.borrow_and_update().state {
            println!("{p:?}");
        }
        println!("--");
    }
}
