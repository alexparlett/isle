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
    /// Lock the screen.
    Lock,
    /// How long the session has been idle, and what keeps it awake.
    Idle,
    #[command(subcommand)]
    Windows(Windows),
    #[command(subcommand)]
    Apps(Apps),
    #[command(subcommand)]
    Desktops(Desktops),
    #[command(subcommand)]
    Displays(Displays),
    #[command(subcommand)]
    Keyboard(Keyboard),
    #[command(subcommand)]
    Wallpaper(Wallpaper),
    #[command(subcommand)]
    Island(Island),
    #[command(subcommand)]
    Menus(Menus),
    /// The system services: whether each daemon answers, and what Isle knows of it.
    Services,
    /// Focus: what is in force, a mode on for a while or until turned off, or off.
    #[command(subcommand)]
    Focus(FocusCommand),
    /// Notifications Isle holds: list them, close one, or clear them all.
    #[command(subcommand)]
    Notify(Notify),
    #[command(subcommand)]
    Shortcuts(Shortcuts),
    #[command(subcommand)]
    Settings(Settings),
    #[command(subcommand)]
    Shell(Shell),
    /// Open Spotlight (`open`, `apps`, `emoji`) or close it (`close`).
    Spotlight { how: String },
    /// Spotlight's rows for a query, without opening it.
    Search {
        query: String,
        /// general, apps, files, contents, actions, clipboard, calculator, emoji, windows,
        /// run, or app:<desktop id>.
        #[arg(long, default_value = "general")]
        scope: String,
    },
    #[command(subcommand)]
    Capture(Capture),
    #[command(subcommand)]
    Portal(Portal),
    /// Hooks served only by test builds.
    #[command(subcommand, hide = true)]
    Test(Test),
}

#[derive(Subcommand)]
enum Capture {
    /// Every screen to a file each, as Shift+Cmd+3 does.
    Screen {
        /// The main screen to the clipboard instead.
        #[arg(long)]
        copy: bool,
    },
    /// The frozen pick of a region or a window, as Shift+Cmd+4.
    Pick {
        #[arg(long)]
        copy: bool,
    },
    /// The capture toolbar, as Shift+Cmd+5.
    Toolbar {
        #[arg(long)]
        copy: bool,
    },
    /// The colour pick, as Shift+Cmd+6.
    Colour,
    /// Start recording the main screen, or stop the recording.
    Record { action: RecordAction },
    /// The streams, shares and recording running.
    Streams {
        /// End this stream from Isle's side.
        #[arg(long)]
        stop: Option<u64>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum RecordAction {
    Start,
    Stop,
}

#[derive(Subcommand)]
enum Portal {
    /// Whether Isle is the portal backend, and the shares running.
    Status,
    /// What apps may do without asking: screenshots, and shares they keep.
    Permissions {
        /// Take back everything this app id was allowed.
        #[arg(long)]
        revoke: Option<String>,
    },
}

const PERMISSIONS: &str = "org.freedesktop.impl.portal.PermissionStore";
const PERMISSIONS_PATH: &str = "/org/freedesktop/impl/portal/PermissionStore";

/// Each app's permissions in the store's `table`, by entry id.
async fn permissions(
    bus: &zbus::Connection,
    table: &str,
) -> zbus::Result<Vec<(String, std::collections::HashMap<String, Vec<String>>)>> {
    let ids: Vec<String> = bus
        .call_method(Some(PERMISSIONS), PERMISSIONS_PATH, Some(PERMISSIONS), "List", &(table,))
        .await?
        .body()
        .deserialize()?;
    let mut out = Vec::new();
    for id in ids {
        let (apps, _): (std::collections::HashMap<String, Vec<String>>, zvariant::OwnedValue) = bus
            .call_method(
                Some(PERMISSIONS),
                PERMISSIONS_PATH,
                Some(PERMISSIONS),
                "Lookup",
                &(table, id.as_str()),
            )
            .await?
            .body()
            .deserialize()?;
        out.push((id, apps));
    }
    Ok(out)
}

#[derive(Subcommand)]
enum Windows {
    /// Every window, front first.
    List,
    /// Focus and raise a window by its id.
    Focus { id: u64 },
    /// Ask a window to close, by its id.
    Close { id: u64 },
    Minimise { id: u64 },
    Unminimise { id: u64 },
    /// Hide every window of an app.
    Hide { app: String },
    /// Show a hidden app's windows again.
    Unhide { app: String },
    /// Tile a window to a zone: left-half, right-half, left-third, centre-third, right-third,
    /// left-two-thirds, right-two-thirds, top-left, top-right, bottom-left, bottom-right,
    /// centre, fill, beside.
    Tile { id: u64, zone: String },
    /// Return a tiled or filled window to its box before.
    Restore { id: u64 },
    /// Move and size a window, in logical points.
    SetBox {
        id: u64,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
}

#[derive(Subcommand)]
enum Shortcuts {
    /// Every action with its chords, and the packet that builds those not built yet.
    List,
}

#[derive(Subcommand)]
enum Keyboard {
    /// Every keyboard, with its profile and layouts; `*` marks the one the seat follows.
    List,
    /// The active layout, Caps Lock and Num Lock.
    Status,
    /// Set a keyboard's profile by its name: mac, pc or auto.
    Profile { name: String, profile: String },
}

/// `--output` names a display; the one with focus when left out.
#[derive(Subcommand)]
enum Desktops {
    /// Every display's desktops.
    List,
    /// Show a desktop, by its place from 1.
    Switch {
        index: u32,
        #[arg(long, default_value = "")]
        output: String,
    },
    /// Add a desktop at the end of the row.
    Add {
        #[arg(long, default_value = "")]
        output: String,
    },
    /// Remove a desktop; its windows join the one before.
    Remove {
        index: u32,
        #[arg(long, default_value = "")]
        output: String,
    },
    /// Move a window to a desktop of its display.
    Move { id: u64, index: u32 },
    /// Put a window into full screen, or take it out with --off.
    FullScreen {
        id: u64,
        #[arg(long)]
        off: bool,
    },
}

#[derive(Subcommand)]
enum Apps {
    /// Apps with windows, most recently used first, then apps running in the background.
    List,
    /// Show an app's windows and focus its most recent.
    Activate { app: String },
    /// Close an app's windows, then end its processes.
    Quit { app: String },
    /// End an app running in the background with no window.
    Stop { app: String },
    /// Start an app by desktop id, or one of its entry's actions.
    Launch {
        app: String,
        #[arg(long)]
        action: Option<String>,
        uris: Vec<String>,
    },
    /// Open a link or file in the app that handles it.
    Open { target: String },
}

#[derive(Subcommand)]
enum Displays {
    List,
    /// Release every display and probe the connectors again, as after a KVM switch.
    Redetect,
    /// Turn the displays off now; any input lights them.
    Sleep,
    /// What each frame of a display put on its planes, and how often it was scanned out.
    Trace {
        output: String,
        /// Print frames as they are presented until interrupted.
        #[arg(long)]
        follow: bool,
        /// Clear the counters first.
        #[arg(long)]
        reset: bool,
    },
}

#[derive(Subcommand)]
enum Menus {
    /// The front app's menu bar, as JSON.
    Bar,
    /// Choose an item by its path: a title, then labels down its menus.
    Activate { path: Vec<String> },
    /// A window's own menus and where they came from, by window id.
    Show { id: u64 },
    /// The front app's menu items that match, with their paths.
    Search { query: String },
}

#[derive(Subcommand)]
enum FocusCommand {
    /// The Focus in force and why.
    Status,
    /// Turns a mode on, for `--for` minutes or until turned off.
    On {
        mode: String,
        #[arg(long = "for")]
        minutes: Option<u32>,
    },
    /// Turns the Focus in force off.
    Off,
}

#[derive(Subcommand)]
enum Notify {
    /// Every notification, live and recorded.
    List,
    /// Closes one by its id.
    Close { id: u32 },
    /// Clears every one.
    Clear,
}

#[derive(Subcommand)]
enum Island {
    /// What each screen's island shows, as JSON.
    Status,
}

#[derive(Subcommand)]
enum Wallpaper {
    /// What each screen shows, and why a choice is not shown.
    Status,
    /// Show a picture, a folder of pictures, a colour as #rrggbb, or `frost`.
    Set {
        source: String,
        /// One screen's own, by its output name; every screen without one when left out.
        #[arg(long)]
        output: Option<String>,
        /// Only in the dark appearance.
        #[arg(long)]
        dark: bool,
    },
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
    /// Whether tokens.kdl is in force, or why not.
    Tokens,
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
    // Output into a closed pipe (`| head`) ends the program quietly, as other tools do.
    // Safety: the default disposition; nothing here installs a SIGPIPE handler.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
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
        Command::Lock => SessionProxy::new(&bus).await?.lock().await?,
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
                            let mut state = w.mode.clone();
                            if w.minimised {
                                state.push_str(", minimised");
                            }
                            if w.hidden {
                                state.push_str(", hidden");
                            }
                            println!(
                                "{mark} {:>4}  {:<24} {}  [{}] {state}",
                                w.id, w.app, w.title, w.output
                            );
                        }
                    }
                }
                Windows::Focus { id } => found(proxy.focus(*id).await?, *id)?,
                Windows::Close { id } => found(proxy.close(*id).await?, *id)?,
                Windows::Minimise { id } => found(proxy.minimise(*id).await?, *id)?,
                Windows::Unminimise { id } => found(proxy.unminimise(*id).await?, *id)?,
                Windows::Hide { app } => app_found(proxy.set_app_hidden(app, true).await?, app)?,
                Windows::Unhide { app } => app_found(proxy.set_app_hidden(app, false).await?, app)?,
                Windows::Tile { id, zone } => {
                    if !proxy.tile(*id, zone).await? {
                        return Err(zbus::Error::Failure(format!(
                            "window {id} could not tile to {zone}"
                        )));
                    }
                }
                Windows::Restore { id } => {
                    if !proxy.restore(*id).await? {
                        return Err(zbus::Error::Failure(format!(
                            "window {id} is neither tiled nor filled"
                        )));
                    }
                }
                Windows::SetBox {
                    id,
                    x,
                    y,
                    width,
                    height,
                } => found(proxy.set_box(*id, *x, *y, *width, *height).await?, *id)?,
            }
        }
        Command::Shortcuts(Shortcuts::List) => {
            let proxy = isle_ipc::keyboard::KeyboardProxy::new(&bus).await?;
            let shortcuts = proxy.shortcuts().await?;
            if cli.json {
                print_json(&shortcuts);
            } else {
                for s in shortcuts {
                    let chords = if s.chords.is_empty() {
                        "-".to_owned()
                    } else {
                        s.chords.join(", ")
                    };
                    let packet = if s.packet.is_empty() {
                        String::new()
                    } else {
                        format!("  ({})", s.packet)
                    };
                    println!("{:<24} {:<34} {}{packet}", s.id, chords, s.label);
                }
            }
        }
        Command::Keyboard(command) => {
            let proxy = isle_ipc::keyboard::KeyboardProxy::new(&bus).await?;
            match command {
                Keyboard::List => {
                    let keyboards = proxy.list().await?;
                    if cli.json {
                        print_json(&keyboards);
                    } else {
                        for k in keyboards {
                            let mark = if k.active { "*" } else { " " };
                            let profile = if k.guessed {
                                format!("{} (guessed)", k.profile)
                            } else {
                                k.profile.clone()
                            };
                            let mut notes = Vec::new();
                            if k.swap {
                                notes.push("Alt and Super swapped");
                            }
                            if k.remapper {
                                notes.push("v1's xremap");
                            }
                            println!(
                                "{mark} {:<40} {:<14} {:<10} {}",
                                k.name,
                                profile,
                                k.layouts,
                                notes.join(", ")
                            );
                        }
                    }
                }
                Keyboard::Status => {
                    let status = proxy.status().await?;
                    if cli.json {
                        print_json(&status);
                    } else {
                        let on = |b: bool| if b { "on" } else { "off" };
                        println!("layout     {}", status.layout);
                        println!("caps lock  {}", on(status.caps_lock));
                        println!("num lock   {}", on(status.num_lock));
                    }
                }
                Keyboard::Profile { name, profile } => proxy.set_profile(name, profile).await?,
            }
        }
        Command::Desktops(command) => {
            let proxy = isle_ipc::desktops::DesktopsProxy::new(&bus).await?;
            let refused = |what: &str| Err(zbus::Error::Failure(what.to_owned()));
            match command {
                Desktops::List => {
                    let desktops = proxy.list().await?;
                    if cli.json {
                        print_json(&desktops);
                    } else {
                        for d in desktops {
                            let mark = if d.current { "*" } else { " " };
                            let what = if d.window > 0 {
                                format!("full screen: window {}", d.window)
                            } else {
                                format!("{} window(s)", d.windows)
                            };
                            println!("{mark} {:<12} {:>2}  {what}", d.output, d.index);
                        }
                    }
                }
                Desktops::Switch { index, output } => {
                    if !proxy.switch(output, *index).await? {
                        return refused("no such desktop");
                    }
                }
                Desktops::Add { output } => match proxy.add(output).await? {
                    0 => return refused("the row is full"),
                    n => println!("{n}"),
                },
                Desktops::Remove { index, output } => {
                    if !proxy.remove(output, *index).await? {
                        return refused("that desktop cannot be removed");
                    }
                }
                Desktops::Move { id, index } => {
                    if !proxy.move_window(*id, *index).await? {
                        return refused("the window could not move there");
                    }
                }
                Desktops::FullScreen { id, off } => {
                    if !proxy.set_full_screen(*id, !off).await? {
                        return refused("no change");
                    }
                }
            }
        }
        Command::Apps(command) => {
            let proxy = WindowsProxy::new(&bus).await?;
            match command {
                Apps::List => {
                    let apps = proxy.apps().await?;
                    if cli.json {
                        print_json(&apps);
                    } else {
                        for a in apps {
                            let mut marks = Vec::new();
                            if a.background {
                                marks.push("running in background".to_owned());
                            }
                            if a.hidden {
                                marks.push("hidden".to_owned());
                            }
                            if a.minimised {
                                marks.push("minimised".to_owned());
                            }
                            if a.urgent {
                                marks.push("urgent".to_owned());
                            } else if a.attention {
                                marks.push("attention".to_owned());
                            }
                            if a.not_responding {
                                marks.push("not responding".to_owned());
                            }
                            if a.badge >= 0 {
                                marks.push(format!("badge {}", a.badge));
                            }
                            if a.progress >= 0.0 {
                                marks.push(format!("{:.0}% done", a.progress * 100.0));
                            }
                            println!(
                                "{:<32} {:<24} {} window(s) {}",
                                a.app,
                                a.name,
                                a.windows,
                                marks.join(", ")
                            );
                        }
                    }
                }
                Apps::Activate { app } => app_found(proxy.activate_app(app).await?, app)?,
                Apps::Quit { app } => app_found(proxy.quit_app(app).await?, app)?,
                Apps::Stop { app } => app_found(proxy.stop_app(app).await?, app)?,
                Apps::Launch { app, action, uris } => {
                    let uris: Vec<&str> = uris.iter().map(String::as_str).collect();
                    let action = action.as_deref().unwrap_or("");
                    app_found(proxy.launch(app, action, &uris).await?, app)?
                }
                Apps::Open { target } => app_found(proxy.open(target).await?, target)?,
            }
        }
        Command::Displays(Displays::Redetect) => DisplaysProxy::new(&bus).await?.redetect().await?,
        Command::Displays(Displays::Sleep) => DisplaysProxy::new(&bus).await?.sleep().await?,
        Command::Displays(Displays::Trace {
            output,
            follow,
            reset,
        }) => {
            let (follow, reset) = (*follow, *reset);
            let displays = DisplaysProxy::new(&bus).await?;
            if reset {
                displays.reset_trace(output).await?;
            }
            let trace = displays.trace(output, 0).await?;
            if cli.json && !follow {
                print_json(&trace);
                return Ok(());
            }
            let mut last = 0;
            if !follow {
                for frame in &trace.recent {
                    print_frame(frame, cli.json);
                }
            } else {
                last = trace.recent.last().map_or(0, |f| f.frame);
            }
            if !cli.json {
                println!(
                    "{}: {} frames, {} scanned out, {} async",
                    trace.output, trace.frames, trace.scanned_out, trace.async_flips
                );
            }
            if follow {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    let trace = displays.trace(output, last).await?;
                    for frame in &trace.recent {
                        print_frame(frame, cli.json);
                    }
                    last = trace.recent.last().map_or(last, |f| f.frame);
                }
            }
        }
        Command::Idle => {
            let status: serde_json::Value =
                serde_json::from_str(&SessionProxy::new(&bus).await?.idle().await?).unwrap_or_default();
            if cli.json {
                println!("{status}");
            } else {
                let holders: Vec<String> = status["holders"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|h| h.as_str().map(str::to_owned)).collect())
                    .unwrap_or_default();
                println!("idle for {} s", status["idle_ms"].as_u64().unwrap_or(0) / 1000);
                if holders.is_empty() {
                    println!("nothing holds it");
                } else {
                    println!("held by {}", holders.join(", "));
                }
            }
        }
        Command::Displays(Displays::List) => {
            let displays = DisplaysProxy::new(&bus).await?.list().await?;
            if cli.json {
                print_json(&displays);
            } else {
                for d in displays {
                    println!(
                        "{}  {} {} {}  {}x{}@{:.2}  scale {}  at {},{}  vrr {} ({}, {})",
                        d.name,
                        d.make,
                        d.model,
                        d.serial,
                        d.width,
                        d.height,
                        d.refresh as f64 / 1000.0,
                        d.scale,
                        d.x,
                        d.y,
                        if d.vrr_on { "on" } else { "off" },
                        d.vrr_setting,
                        d.vrr_capability,
                    );
                }
            }
        }
        Command::Settings(command) => settings(cli, &bus, command).await?,
        Command::Menus(Menus::Search { query }) => {
            println!("{}", isle_ipc::menus::MenusProxy::new(&bus).await?.search(query).await?);
        }
        Command::Menus(Menus::Show { id }) => {
            println!("{}", isle_ipc::menus::MenusProxy::new(&bus).await?.show(*id).await?);
        }
        Command::Menus(Menus::Bar) => {
            println!("{}", isle_ipc::menus::MenusProxy::new(&bus).await?.bar().await?);
        }
        Command::Menus(Menus::Activate { path }) => {
            let path: Vec<&str> = path.iter().map(String::as_str).collect();
            let found = isle_ipc::menus::MenusProxy::new(&bus).await?.activate(&path).await?;
            app_found(found, &path.join(" › "))?;
        }
        Command::Services => {
            let status = isle_ipc::services::ServicesProxy::new(&bus).await?.status().await?;
            let status: serde_json::Value = serde_json::from_str(&status)
                .map_err(|e| zbus::Error::Failure(e.to_string()))?;
            if cli.json {
                println!("{status}");
            } else if let Some(services) = status.as_object() {
                for (name, service) in services {
                    let available = service["available"].as_bool().unwrap_or(false);
                    println!(
                        "{name}: {} {}",
                        if available { "available" } else { "not available" },
                        service["state"]
                    );
                }
            }
        }
        Command::Focus(command) => {
            let proxy = isle_ipc::focus::FocusProxy::new(&bus).await?;
            match command {
                FocusCommand::Status => {
                    let status: serde_json::Value = serde_json::from_str(&proxy.status().await?)
                        .map_err(|e| zbus::Error::Failure(e.to_string()))?;
                    if cli.json {
                        println!("{status}");
                    } else {
                        println!("{}", status["active"].as_str().unwrap_or("Off"));
                    }
                }
                FocusCommand::On { mode, minutes } => proxy.set(mode, minutes.unwrap_or(0)).await?,
                FocusCommand::Off => proxy.off().await?,
            }
        }
        Command::Notify(command) => {
            let proxy = isle_ipc::notifications::NotificationsProxy::new(&bus).await?;
            match command {
                Notify::List => {
                    let list: serde_json::Value = serde_json::from_str(&proxy.list().await?)
                        .map_err(|e| zbus::Error::Failure(e.to_string()))?;
                    if cli.json {
                        println!("{list}");
                    } else {
                        for e in list["entries"].as_array().into_iter().flatten() {
                            println!(
                                "{:>5} {:<8} {}: {}",
                                e["id"],
                                if e["live"] == true { "live" } else { "record" },
                                e["app"].as_str().unwrap_or_default(),
                                e["summary"].as_str().unwrap_or_default()
                            );
                        }
                    }
                }
                Notify::Close { id } => {
                    if !proxy.close(*id).await? {
                        return Err(zbus::Error::Failure(format!("no notification {id}")));
                    }
                }
                Notify::Clear => proxy.clear().await?,
            }
        }
        Command::Island(Island::Status) => {
            println!("{}", isle_ipc::island::IslandProxy::new(&bus).await?.status().await?);
        }
        Command::Wallpaper(Wallpaper::Status) => {
            let screens = isle_ipc::wallpaper::WallpaperProxy::new(&bus).await?.status().await?;
            if cli.json {
                print_json(&screens);
            } else {
                for s in screens {
                    let shown = if s.file.is_empty() { &s.source } else { &s.file };
                    println!("{}  {}", s.output, shown);
                    if !s.problem.is_empty() {
                        println!("    {}", s.problem);
                    }
                }
            }
        }
        Command::Wallpaper(Wallpaper::Set {
            source,
            output,
            dark,
        }) => {
            let section = match output {
                Some(name) => {
                    let displays = DisplaysProxy::new(&bus).await?.list().await?;
                    let Some(d) = displays.iter().find(|d| &d.name == name) else {
                        return Err(zbus::Error::Failure(format!("no output is named {name}")));
                    };
                    format!("wallpaper-output[{} {} {}]", d.make, d.model, d.serial)
                }
                None => "wallpaper".to_owned(),
            };
            let key = format!("{section}.{}", if *dark { "dark-source" } else { "source" });
            // A path is kept whole, so a relative one names the same file for the compositor.
            let source = match std::path::Path::new(source).canonicalize() {
                Ok(path) if !source.starts_with('#') => path.display().to_string(),
                _ => source.clone(),
            };
            let changed = SettingsProxy::new(&bus)
                .await?
                .set(&key, &zvariant::Value::from(source))
                .await?;
            if cli.json {
                print_json(&changed);
            }
        }
        Command::Capture(command) => {
            let proxy = isle_ipc::capture::CaptureProxy::new(&bus).await?;
            match command {
                Capture::Screen { copy } => proxy.shoot("screen", *copy).await?,
                Capture::Pick { copy } => proxy.shoot("pick", *copy).await?,
                Capture::Toolbar { copy } => proxy.shoot("toolbar", *copy).await?,
                Capture::Colour => proxy.shoot("colour", false).await?,
                Capture::Record { action } => {
                    let action = match action {
                        RecordAction::Start => "start",
                        RecordAction::Stop => "stop",
                    };
                    let running = proxy.record(action).await?;
                    if cli.json {
                        print_json(&running);
                    }
                }
                Capture::Streams { stop } => {
                    if let Some(id) = stop
                        && !proxy.stop_stream(*id).await?
                    {
                        return Err(zbus::Error::Failure(format!("no stream {id}")));
                    }
                    let status: serde_json::Value =
                        serde_json::from_str(&proxy.status().await?).unwrap_or_default();
                    if cli.json {
                        print_json(&status);
                    } else {
                        for s in status["streams"].as_array().into_iter().flatten() {
                            println!(
                                "{}  {}  {}  node {}  {}x{}",
                                s["id"],
                                s["owner"].as_str().unwrap_or(""),
                                s["source"],
                                s["node"],
                                s["size"][0],
                                s["size"][1]
                            );
                        }
                        if let Some(path) = status["recording"].as_str() {
                            println!("recording to {path}");
                        }
                    }
                }
            }
        }
        Command::Portal(command) => match command {
            Portal::Status => {
                let status: serde_json::Value = serde_json::from_str(
                    &isle_ipc::capture::CaptureProxy::new(&bus).await?.status().await?,
                )
                .unwrap_or_default();
                if cli.json {
                    print_json(&serde_json::json!({"backend": status["portal"], "shares": status["shares"]}));
                } else {
                    let owned = status["portal"].as_bool().unwrap_or(false);
                    println!(
                        "backend: {}",
                        if owned { "Isle" } else { "not Isle's (another program owns it)" }
                    );
                    for (path, share) in status["shares"].as_object().into_iter().flatten() {
                        println!("sharing with {}  {path}", share["app"].as_str().unwrap_or(""));
                    }
                }
            }
            Portal::Permissions { revoke } => {
                let tables = ["screenshot", "screencast"];
                if let Some(app) = revoke {
                    for table in tables {
                        for (id, apps) in permissions(&bus, table).await.unwrap_or_default() {
                            if apps.contains_key(app.as_str()) {
                                bus.call_method(
                                    Some(PERMISSIONS),
                                    PERMISSIONS_PATH,
                                    Some(PERMISSIONS),
                                    "DeletePermission",
                                    &(table, id.as_str(), app.as_str()),
                                )
                                .await?;
                            }
                        }
                    }
                }
                let mut rows = Vec::new();
                for table in tables {
                    for (id, apps) in permissions(&bus, table).await.unwrap_or_default() {
                        for (app, granted) in apps {
                            rows.push(serde_json::json!({"table": table, "id": id, "app": app, "granted": granted}));
                        }
                    }
                }
                if cli.json {
                    print_json(&rows);
                } else {
                    for row in &rows {
                        let app = row["app"].as_str().filter(|a| !a.is_empty()).unwrap_or("(apps with no id)");
                        let granted = row["granted"].as_array().map(|g| {
                            g.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(",")
                        });
                        println!("{}  {app}  {}", row["table"].as_str().unwrap_or(""), granted.unwrap_or_default());
                    }
                }
            }
        },
        Command::Spotlight { how } => {
            let open = ShellProxy::new(&bus).await?.spotlight(how).await?;
            if cli.json {
                print_json(&open);
            }
        }
        Command::Search { query, scope } => {
            let found = ShellProxy::new(&bus).await?.search(query, scope).await?;
            if cli.json {
                println!("{found}");
            } else {
                let rows: serde_json::Value = serde_json::from_str(&found).unwrap_or_default();
                for row in rows.as_array().into_iter().flatten() {
                    let group = row["group"].as_str().unwrap_or("");
                    let title = row["title"].as_str().unwrap_or("");
                    match row["detail"].as_str() {
                        Some(detail) => println!("{group}  {title}  ({detail})"),
                        None => println!("{group}  {title}"),
                    }
                }
            }
        }
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
                Shell::Tokens => {
                    let status = proxy.tokens_status().await?;
                    if cli.json {
                        print_json(&status);
                    } else if status.ok {
                        println!("in force: {}", status.file);
                    } else {
                        println!(
                            "{}:{}:{}: {}",
                            status.file, status.line, status.column, status.message
                        );
                        return Err(zbus::Error::Failure("tokens.kdl is not in force".into()));
                    }
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

fn app_found(found: bool, app: &str) -> zbus::Result<()> {
    if found {
        Ok(())
    } else {
        Err(zbus::Error::Failure(format!("no window belongs to {app}")))
    }
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

fn print_frame(frame: &isle_ipc::FrameTrace, json: bool) {
    if json {
        println!("{}", serde_json::to_string(frame).unwrap_or_default());
        return;
    }
    println!(
        "{:>8}  {:>8.3} ms  {:<28}  cursor {:<5}{}{}{}",
        frame.frame,
        frame.interval_us as f64 / 1000.0,
        frame.primary,
        frame.cursor,
        if frame.asynchronous { "  async" } else { "" },
        if frame.vrr { "  vrr" } else { "" },
        if frame.hardware { "" } else { "  (Isle's time)" },
    );
}
