//! Store's worker: follows `Updates1`, reads the installed apps and the catalogues, works out
//! what Get and Remove do, and carries the window's asks to the compositor.

use std::collections::HashSet;
use std::path::PathBuf;

use isle_catalog::catalogue::{self, Offer};

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_util::StreamExt;
use tracing::warn;

use crate::installed::{self, App};
use crate::plan::{self, Plan};

pub enum Event {
    /// `Updates1.Status`, read again on every change.
    Status(serde_json::Value),
    Installed(Vec<App>),
    /// Every package installed, by name.
    Packages(HashSet<String>),
    /// The packages no repository has, as `pacman -Qm` gives them: name and version.
    Foreign(Vec<(String, String)>),
    /// The packages installed on purpose, not as another's dependency (`pacman -Qe`).
    Explicit(Vec<(String, String)>),
    /// The packages Store never removes.
    Core(HashSet<String>),
    /// CachyOS's and Flathub's apps.
    Offers(Vec<Offer>),
    /// Catalogue icons made drawable: the catalogue's file and the drawable one.
    Icons(Vec<(PathBuf, Option<PathBuf>)>),
    /// A screenshot fetched: its address and the local copy, or none when it could not be.
    Shot(String, Option<PathBuf>),
    /// The AUR's answer to a search.
    Aur(String, Result<Vec<Offer>, String>),
    /// What Get or Remove would do, by the key it was asked under.
    Plan(String, Plan),
    /// The registered AppImages and whether each can update, by entry id.
    Images(std::collections::HashMap<String, Image>),
    /// Why the last ask was refused or failed.
    Failed(String),
}

#[derive(Debug)]
pub enum Ask {
    /// A run of a part: `system`, `flathub`, `flathub:<id>`, `aur`, `aur:<package>`.
    Install(String),
    Check,
    Stop,
    Launch(String),
    /// The restart a system update owes, through the session's confirmation.
    Restart,
    /// An AppImage to the Trash, by its entry id.
    Trash(String),
    Rescan,
    Icons(Vec<PathBuf>),
    Shots(Vec<String>),
    SearchAur(String),
    PlanGet(String, Offer),
    PlanRemove(String, String, String),
}

#[derive(Clone)]
pub struct Data(UnboundedSender<Ask>);

impl PartialEq for Data {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Data {
    pub fn ask(&self, ask: Ask) {
        let _ = self.0.unbounded_send(ask);
    }

    /// A Data no worker reads, its asks kept for a test to look at.
    #[cfg(test)]
    pub fn detached() -> (Data, UnboundedReceiver<Ask>) {
        let (asks, asked) = unbounded();
        (Data(asks), asked)
    }
}

pub fn start(
    worker: &isle_app::Worker,
    bus: Option<zbus::Connection>,
) -> (Data, UnboundedReceiver<Event>) {
    let (asks, asked) = unbounded();
    let (events, rx) = unbounded();
    worker.spawn(run(asked, events, bus));
    (Data(asks), rx)
}

/// An AppImage and how it updates.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub path: PathBuf,
    /// Its embedded update information, if it has any.
    pub info: Option<String>,
    /// From `appimageupdatetool -j`: `None` when the tool is not installed or could not tell.
    pub newer: Option<bool>,
}

/// Moves the image registered as `id` to its trash; why not, when it could not.
fn trash_image(id: &str) -> Option<String> {
    let home = isle_apps::appimage::data_home()?;
    let image = isle_apps::appimage::registered(&home.join("applications"))
        .into_values()
        .find(|r| r.id == id)?
        .image;
    isle_fs::trash::trash(&image).err().map(|e| format!("Couldn't move it to the Trash: {e}"))
}

fn images() -> std::collections::HashMap<String, Image> {
    let Some(home) = isle_apps::appimage::data_home() else {
        return Default::default();
    };
    let tool = isle_updates::appimage_tool();
    isle_apps::appimage::registered(&home.join("applications"))
        .into_values()
        .map(|r| {
            let info = isle_apps::appimage::update_info(&r.image);
            // `-j` exits 1 when an update is available and 0 when the image is current.
            let newer = tool
                .as_ref()
                .filter(|_| info.is_some())
                .map(|tool| {
                    std::process::Command::new(tool)
                        .arg("-j")
                        .arg(&r.image)
                        .output()
                        .ok()
                        .and_then(|o| match o.status.code() {
                            Some(0) => Some(false),
                            Some(1) => Some(true),
                            _ => None,
                        })
                })
                .flatten();
            (
                r.id,
                Image {
                    path: r.image,
                    info,
                    newer,
                },
            )
        })
        .collect()
}

fn packages() -> HashSet<String> {
    std::fs::read_dir("/var/lib/pacman/local")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|d| {
            let dir = d.file_name().to_string_lossy().into_owned();
            let mut parts = dir.rsplitn(3, '-');
            parts.next()?;
            parts.next()?;
            parts.next().map(str::to_owned)
        })
        .collect()
}

fn foreign() -> Vec<(String, String)> {
    listed("-Qm")
}

fn explicit() -> Vec<(String, String)> {
    listed("-Qe")
}

/// `pacman`'s `name version` lines for a query.
fn listed(query: &str) -> Vec<(String, String)> {
    std::process::Command::new("pacman")
        .arg(query)
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.split_once(' '))
                .map(|(n, v)| (n.to_owned(), v.to_owned()))
                .collect()
        })
        .unwrap_or_default()
}

fn cache(kind: &str) -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("isle/store").join(kind)
}

fn offers() -> Vec<Offer> {
    let root = std::path::Path::new("/");
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut all = catalogue::repositories(root);
    all.extend(catalogue::flathub(&home, root));
    all
}

async fn shot(url: String) -> Option<PathBuf> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut h);
    let ext = url.rsplit('.').next().filter(|e| e.len() <= 4).unwrap_or("png");
    let dir = cache("shots");
    let path = dir.join(format!("{:016x}.{ext}", h.finish()));
    if path.exists() {
        return Some(path);
    }
    let bytes = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .bytes()
        .await
        .ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    let part = path.with_extension("part");
    std::fs::write(&part, &bytes).ok()?;
    std::fs::rename(&part, &path).ok()?;
    Some(path)
}

/// The latest search asked for, so an older one's answer is dropped and typing is not a
/// request per key.
static LATEST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

async fn search_aur(term: String, events: UnboundedSender<Event>) {
    *LATEST.lock().unwrap() = term.clone();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    if *LATEST.lock().unwrap() != term {
        return;
    }
    let got = catalogue::search_aur(&term).await;
    let _ = events.unbounded_send(Event::Aur(term, got));
}

/// The asks that only read, each in its own task so a slow one holds up none.
fn side(ask: Ask, events: &UnboundedSender<Event>) -> Option<Ask> {
    let events = events.clone();
    match ask {
        Ask::Icons(list) => {
            tokio::task::spawn_blocking(move || {
                let dir = cache("icons");
                let made = list
                    .into_iter()
                    .map(|icon| {
                        let drawable = isle_catalog::icons::drawable(&icon, &dir);
                        (icon, drawable)
                    })
                    .collect();
                let _ = events.unbounded_send(Event::Icons(made));
            });
        }
        Ask::Shots(urls) => {
            for url in urls {
                let events = events.clone();
                tokio::spawn(async move {
                    let path = shot(url.clone()).await;
                    let _ = events.unbounded_send(Event::Shot(url, path));
                });
            }
        }
        Ask::SearchAur(term) => {
            tokio::spawn(search_aur(term, events));
        }
        Ask::PlanGet(key, offer) => {
            tokio::spawn(async move {
                let got = plan::get(offer).await;
                let _ = events.unbounded_send(Event::Plan(key, got));
            });
        }
        Ask::PlanRemove(key, catalogue, id) => {
            tokio::task::spawn_blocking(move || {
                let _ = events.unbounded_send(Event::Plan(key, plan::remove(&catalogue, &id)));
            });
        }
        other => return Some(other),
    }
    None
}

fn scan() -> Vec<App> {
    let index = isle_apps::Index::scan(&isle_apps::Index::data_dirs());
    let mut icons = isle_apps::icons::Icons::papirus(true);
    let owners = installed::owners(std::path::Path::new("/"));
    let foreign: HashSet<String> = std::process::Command::new("pacman")
        .args(["-Qmq"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    installed::classify(index.entries(), &owners, &foreign, &mut |name| {
        icons.lookup(name, 64)
    })
}

async fn follow(bus: zbus::Connection, events: UnboundedSender<Event>) -> zbus::Result<()> {
    let proxy = isle_ipc::updates::UpdatesProxy::new(&bus).await?;
    let mut changed = proxy.receive_changed().await?;
    loop {
        let status = serde_json::from_str(&proxy.status().await?).unwrap_or_default();
        if events.unbounded_send(Event::Status(status)).is_err() {
            return Ok(());
        }
        if changed.next().await.is_none() {
            return Ok(());
        }
    }
}

async fn act(bus: &zbus::Connection, ask: Ask) -> Result<(), String> {
    let err = |e: zbus::Error| e.to_string();
    let updates = isle_ipc::updates::UpdatesProxy::new(bus)
        .await
        .map_err(err)?;
    let why = match ask {
        Ask::Install(part) => updates.install(&part).await.map_err(err)?,
        Ask::Check => updates.check().await.map(|()| String::new()).map_err(err)?,
        Ask::Stop => updates.stop().await.map_err(err)?,
        Ask::Launch(id) => {
            let windows = isle_ipc::windows::WindowsProxy::new(bus)
                .await
                .map_err(err)?;
            windows.launch(&id, "", &[]).await.map_err(err)?;
            String::new()
        }
        Ask::Restart => updates.restart().await.map(|()| String::new()).map_err(err)?,
        _ => String::new(),
    };
    if why.is_empty() { Ok(()) } else { Err(why) }
}

async fn run(
    mut asked: UnboundedReceiver<Ask>,
    events: UnboundedSender<Event>,
    bus: Option<zbus::Connection>,
) {
    let installed = tokio::task::spawn_blocking(scan).await.unwrap_or_default();
    let _ = events.unbounded_send(Event::Installed(installed));
    let _ = events.unbounded_send(Event::Packages(packages()));
    let _ = events.unbounded_send(Event::Foreign(foreign()));
    let _ = events.unbounded_send(Event::Explicit(explicit()));
    let _ = events.unbounded_send(Event::Core(plan::core_here()));
    {
        let events = events.clone();
        tokio::task::spawn_blocking(move || {
            let _ = events.unbounded_send(Event::Images(images()));
        });
    }
    {
        let events = events.clone();
        tokio::task::spawn_blocking(move || {
            let _ = events.unbounded_send(Event::Offers(offers()));
        });
    }
    let Some(bus) = bus else {
        let _ = events.unbounded_send(Event::Failed("Isle is not running on this session.".into()));
        return;
    };
    {
        let (bus, events) = (bus.clone(), events.clone());
        tokio::spawn(async move {
            if let Err(e) = follow(bus, events).await {
                warn!("updates are not followed: {e}");
            }
        });
    }
    while let Some(ask) = asked.next().await {
        let ask = match ask {
            Ask::Trash(id) => {
                let why = tokio::task::spawn_blocking(move || trash_image(&id))
                    .await
                    .unwrap_or_else(|e| Some(e.to_string()));
                if let Some(why) = why {
                    let _ = events.unbounded_send(Event::Failed(why));
                }
                // The compositor forgets the entry once it sees the image gone.
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                Ask::Rescan
            }
            other => other,
        };
        let Some(ask) = side(ask, &events) else {
            continue;
        };
        if matches!(ask, Ask::Check) {
            let events = events.clone();
            tokio::task::spawn_blocking(move || {
                let _ = events.unbounded_send(Event::Images(images()));
            });
        }
        if matches!(ask, Ask::Rescan) {
            let installed = tokio::task::spawn_blocking(scan).await.unwrap_or_default();
            let _ = events.unbounded_send(Event::Installed(installed));
            let _ = events.unbounded_send(Event::Packages(packages()));
            let _ = events.unbounded_send(Event::Foreign(foreign()));
            let _ = events.unbounded_send(Event::Explicit(explicit()));
            let events = events.clone();
            tokio::task::spawn_blocking(move || {
                let _ = events.unbounded_send(Event::Images(images()));
            });
            continue;
        }
        if let Err(why) = act(&bus, ask).await {
            let _ = events.unbounded_send(Event::Failed(why));
        }
    }
}
