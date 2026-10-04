//! `isle`: the command line over the compositor's D-Bus API.

use std::{
    process::ExitCode,
    time::{Duration, UNIX_EPOCH},
};

use clap::{Parser, Subcommand};
use isle_ipc::{
    displays::DisplaysProxy, session::SessionProxy, settings::SettingsProxy, shell::ShellProxy,
    test::TestProxy, version::VersionProxy, windows::WindowsProxy,
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
    #[command(subcommand)]
    Settings(Settings),
    #[command(subcommand)]
    Shell(Shell),
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
    /// Release every display and probe the connectors again, as after a KVM switch.
    Redetect,
}

#[derive(Subcommand)]
enum Settings {
    /// A setting's value in force.
    Get { key: String },
    /// Set a setting; the value is written as in settings.kdl: 1.25, true, "us".
    Set {
        key: String,
        #[arg(num_args = 1.., required = true)]
        value: Vec<String>,
    },
    /// Whether settings.kdl is in force, or why not.
    Status,
    /// Every key, its type, default and range.
    Schema,
    /// Put settings.kdl in force after Isle started on the defaults.
    UseMine,
}

#[derive(Subcommand)]
enum Shell {
    /// Every shell surface, one line per output it is on.
    List,
    /// Open a surface by its name.
    Open { name: String },
    /// Close a surface by its name.
    Close { name: String },
    /// A surface's accessibility tree, as JSON.
    Tree { name: String },
    /// Turn a surface back on after it panicked too often.
    Restart { name: String },
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
        Command::Displays(Displays::Redetect) => DisplaysProxy::new(&bus).await?.redetect().await?,
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
        Command::Settings(command) => settings(cli, &bus, command).await?,
        Command::Shell(command) => {
            let proxy = ShellProxy::new(&bus).await?;
            match command {
                Shell::List => {
                    let surfaces = proxy.list().await?;
                    if cli.json {
                        print_json(&surfaces);
                    } else {
                        for s in surfaces {
                            let state = if s.disabled {
                                "disabled"
                            } else if s.open {
                                "open"
                            } else {
                                "closed"
                            };
                            println!(
                                "{}  {}  {state}  {}x{} at {},{}  panics {}",
                                s.name, s.output, s.width, s.height, s.x, s.y, s.panics
                            );
                        }
                    }
                }
                Shell::Open { name } => surface_found(proxy.open(name).await?, name)?,
                Shell::Close { name } => surface_found(proxy.close(name).await?, name)?,
                Shell::Tree { name } => println!("{}", proxy.tree(name).await?),
                Shell::Restart { name } => surface_found(proxy.restart(name).await?, name)?,
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

async fn settings(cli: &Cli, bus: &zbus::Connection, command: &Settings) -> zbus::Result<()> {
    let proxy = SettingsProxy::new(bus).await?;
    match command {
        Settings::Get { key } => {
            let value = proxy.get(key).await?;
            let text = match &*value {
                zvariant::Value::Bool(b) => serde_json::json!(b),
                zvariant::Value::I64(i) => serde_json::json!(i),
                zvariant::Value::F64(f) => serde_json::json!(f),
                zvariant::Value::Str(s) => serde_json::json!(s.as_str()),
                zvariant::Value::Array(a) => serde_json::json!(
                    a.iter()
                        .filter_map(|v| v
                            .downcast_ref::<zvariant::Str>()
                            .ok()
                            .map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                ),
                other => serde_json::json!(other.to_string()),
            };
            match (&text, cli.json) {
                (serde_json::Value::String(s), false) => println!("{s}"),
                _ => println!("{text}"),
            }
        }
        Settings::Set { key, value } => {
            let value = isle_settings::Value::parse_kdl(&value.join(" "));
            let value = match value {
                isle_settings::Value::Bool(b) => zvariant::Value::from(b),
                isle_settings::Value::Int(i) => zvariant::Value::from(i),
                isle_settings::Value::Float(f) => zvariant::Value::from(f),
                isle_settings::Value::Str(s) => zvariant::Value::from(s),
                isle_settings::Value::List(l) => zvariant::Value::from(l),
            };
            let changed = proxy.set(key, &value).await?;
            if cli.json {
                print_json(&changed);
            }
        }
        Settings::UseMine => {
            let changed = proxy.use_mine().await?;
            if cli.json {
                print_json(&changed);
            }
        }
        Settings::Status => {
            let status = proxy.status().await?;
            if cli.json {
                print_json(&status);
            } else if status.ok {
                println!("{} is in force", status.file);
            } else {
                println!(
                    "{}:{}:{}: {}",
                    status.file, status.line, status.column, status.message
                );
            }
            if !status.ok {
                return Err(zbus::Error::Failure("settings.kdl has an error".into()));
            }
        }
        Settings::Schema => {
            let keys = proxy.schema().await?;
            if cli.json {
                print_json(&keys);
            } else {
                for k in keys {
                    let constraint = if k.constraint.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", k.constraint)
                    };
                    let default = if k.default.is_empty() {
                        "unset".to_owned()
                    } else {
                        k.default
                    };
                    println!("{}  {}{constraint} = {default}", k.key, k.kind);
                    if !k.doc.is_empty() {
                        println!("    {}", k.doc);
                    }
                }
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

fn surface_found(found: bool, name: &str) -> zbus::Result<()> {
    if found {
        Ok(())
    } else {
        Err(zbus::Error::Failure(format!("no surface is called {name}")))
    }
}

fn print_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).expect("JSON"));
}
