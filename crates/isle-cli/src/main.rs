use clap::Parser;

/// Isle's command line, over the compositor's D-Bus API.
#[derive(Parser)]
#[command(name = "isle", version)]
struct Cli {}

fn main() {
    Cli::parse();
}
