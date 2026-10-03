//! `isle`: the command line over the compositor's D-Bus API.

use std::{
    process::ExitCode,
    time::{Duration, UNIX_EPOCH},
};

use clap::{Parser, Subcommand};
use isle_ipc::{
    displays::DisplaysProxy, session::SessionProxy, test::TestProxy, version::VersionProxy,
    windows::WindowsProxy,
};
use serde::Serialize;

/// Exit status when Isle is not running on this session bus.
const NOT_RUNNING: u8 = 69;

#[derive(Parser)]
#[command(name = "isle", version, about = "Talk to the running Isle.")]
struct Cli {
    /// Print JSON instead of text.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// The running compositor's build.
    Version,
    /// End the session.
    Quit,
    #[command(subcommand)]
    Windows(Windows),
    #[command(subcommand)]
    Displays(Displays),
    /// Hooks served only by test builds.
    #[command(subcommand, hide = true)]
    Test(Test),
}

#[derive(Subcommand)]
enum Windows {
    /// Every window, front first.
    List,
    /// Focus and raise a window by its id.
    Focus { id: u64 },
    /// Ask a window to close, by its id.
    Close { id: u64 },
}

#[derive(Subcommand)]
enum Displays {
    List,
}

#[derive(Subcommand)]
enum Test {
    /// Hold the main loop.
    Stall { millis: u32 },
    /// Panic in the worker's D-Bus service.
    PanicService,
    /// Panic on the main loop.
    PanicCore,
}

#[derive(Serialize)]
struct VersionReport {
    version: String,
    commit: String,
    built: String,
    features: Vec<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let not_running = matches!(
                &err,
                zbus::Error::MethodError(name, _, _)
                    if name.as_str() == "org.freedesktop.DBus.Error.ServiceUnknown"
            ) || matches!(&err, zbus::Error::FDO(e) if matches!(**e, zbus::fdo::Error::ServiceUnknown(_)));
            if not_running {
                eprintln!("isle: Isle is not running on this session bus");
                ExitCode::from(NOT_RUNNING)
            } else {
                eprintln!("isle: {err}");
                ExitCode::FAILURE
            }
        }
    }
}

async fn run(cli: &Cli) -> zbus::Result<()> {
    let bus = zbus::Connection::session().await?;
    match &cli.command {
        Command::Version => {
            let proxy = VersionProxy::new(&bus).await?;
            let report = VersionReport {
                version: proxy.version().await?,
                commit: proxy.commit().await?,
                built: humantime::format_rfc3339_seconds(
                    UNIX_EPOCH + Duration::from_secs(proxy.built().await?),
                )
                .to_string(),
                features: proxy.features().await?,
            };
            if cli.json {
                print_json(&report);
            } else {
                let features = if report.features.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", report.features.join(", "))
                };
                println!(
                    "Isle {} ({}) built {}{features}",
                    report.version, report.commit, report.built
                );
            }
        }
        Command::Quit => SessionProxy::new(&bus).await?.quit().await?,
        Command::Windows(command) => {
            let proxy = WindowsProxy::new(&bus).await?;
            match command {
                Windows::List => {
                    let windows = proxy.list().await?;
                    if cli.json {
                        print_json(&windows);
                    } else {
                        for w in windows {
                            let mark = if w.focused { "*" } else { " " };
                            println!(
                                "{mark} {:>4}  {:<24} {}  [{}]",
                                w.id, w.app_id, w.title, w.output
                            );
                        }
                    }
                }
                Windows::Focus { id } => found(proxy.focus(*id).await?, *id)?,
                Windows::Close { id } => found(proxy.close(*id).await?, *id)?,
            }
        }
        Command::Displays(Displays::List) => {
            let displays = DisplaysProxy::new(&bus).await?.list().await?;
            if cli.json {
                print_json(&displays);
            } else {
                for d in displays {
                    println!(
                        "{}  {} {} {}  {}x{}@{:.2}  scale {}  at {},{}",
                        d.name,
                        d.make,
                        d.model,
                        d.serial,
                        d.width,
                        d.height,
                        d.refresh as f64 / 1000.0,
                        d.scale,
                        d.x,
                        d.y
                    );
                }
            }
        }
        Command::Test(command) => {
            let proxy = TestProxy::new(&bus).await?;
            match command {
                Test::Stall { millis } => proxy.stall(*millis).await?,
                Test::PanicService => proxy.panic_service().await?,
                Test::PanicCore => proxy.panic_core().await?,
            }
        }
    }
    Ok(())
}

fn found(found: bool, id: u64) -> zbus::Result<()> {
    if found {
        Ok(())
    } else {
        Err(zbus::Error::Failure(format!("no window has id {id}")))
    }
}

fn print_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).expect("JSON"));
}
