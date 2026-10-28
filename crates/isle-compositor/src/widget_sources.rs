//! The gallery's sources: fetching through gitoxide on the worker, Browse, Install, Update,
//! Remove, the consent panel and Sources (`p17-widgets.md` choices 21–23).

use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use isle_providers::{
    Grant,
    sources::{self, Index, Listing},
};
use isle_shell::dashboard::surface::{ConsentView, DashboardAction, SourceRow, SourcesView};

use crate::state::{Isle, State};

/// How long a source's index stays fresh.
const FRESH: u64 = 60 * 60;
/// The longest a fetch may take.
const FETCH_LIMIT: Duration = Duration::from_secs(60);

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The sources' clones: `~/.cache/isle/widget-sources`, or the environment's for tests.
pub fn cache() -> PathBuf {
    if let Some(dir) = std::env::var_os("ISLE_CACHE_DIR") {
        return PathBuf::from(dir).join("widget-sources");
    }
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache"))
        .join("isle/widget-sources")
}

/// What the consent panel asks.
#[derive(Clone, Debug)]
pub enum Asking {
    Install { url: String, listing: Listing, new_source: bool },
    /// An update that asks for more.
    Update { url: String, listing: Listing },
    /// A source turned off, for one of its widgets.
    AllowSource { url: String, id: String },
    /// The user's own provider's declared permissions.
    AllowOwn { id: String },
}

#[derive(Clone, Debug)]
pub struct Consent {
    pub asking: Asking,
    pub permissions: Vec<Grant>,
    pub armed: bool,
}

impl Isle {
    /// The listing `id` from `url`'s last good index.
    pub fn listing(&self, url: &str, id: &str) -> Option<Listing> {
        match self.dashboard.browse.get(url)? {
            Ok(index) => index.widgets.iter().find(|l| l.id == id).cloned(),
            Err(_) => None,
        }
    }

    /// Installed providers from sources with a later version offered.
    pub fn updates(&self) -> Vec<(String, String, Listing)> {
        self.dashboard
            .providers
            .iter()
            .filter_map(|(id, p)| {
                let url = p.source.clone()?;
                let listing = self.listing(&url, id)?;
                p.manifest.older_than(&listing.manifest).then_some((id.clone(), url, listing))
            })
            .collect()
    }

    pub fn sources_view(&self, adding: bool) -> SourcesView {
        SourcesView {
            rows: self
                .settings
                .widgets
                .sources
                .iter()
                .map(|url| {
                    let fetched = self.dashboard.memory.sources.get(url);
                    let (line, error) = match fetched.and_then(|f| f.error.clone()) {
                        Some(e) => (format!("Couldn't update {}. {e}", crate::dashboard::source_name(url)), true),
                        None => match fetched.and_then(|f| f.fetched_at) {
                            Some(at) => {
                                let when = jiff::Timestamp::from_second(at as i64)
                                    .map(|t| t.to_zoned(jiff::tz::TimeZone::system()).strftime("%-d %b %H:%M").to_string())
                                    .unwrap_or_default();
                                (format!("Last updated {when}"), false)
                            }
                            None => ("Not fetched yet".to_owned(), false),
                        },
                    };
                    SourceRow {
                        url: url.clone(),
                        line,
                        error,
                        allowed: self.settings.widget_source.get(url).is_some_and(|s| s.allowed),
                    }
                })
                .collect(),
            adding,
        }
    }

    pub fn consent_view(&self, c: &Consent) -> ConsentView {
        let words: Vec<(String, String)> = c.permissions.iter().map(|g| (g.glyph().to_owned(), g.words())).collect();
        let source_line = |url: &str| Some(format!("From {}", crate::dashboard::source_name(url)));
        match &c.asking {
            Asking::Install { url, listing, new_source } => ConsentView {
                title: format!("Install “{}”?", listing.manifest.name),
                command: Some(listing.manifest.command()),
                ask: "It asks to:".into(),
                permissions: words,
                source: source_line(url),
                also: new_source.then(|| {
                    format!("This also allows other widgets from {} to run.", crate::dashboard::source_name(url))
                }),
                confirm: "Install".into(),
                armed: c.armed,
            },
            Asking::Update { url, listing } => ConsentView {
                title: format!("Update “{}”?", listing.manifest.name),
                command: Some(listing.manifest.command()),
                ask: format!("“{}” now asks to:", listing.manifest.name),
                permissions: words,
                source: source_line(url),
                also: None,
                confirm: "Update".into(),
                armed: c.armed,
            },
            Asking::AllowSource { url, id } => {
                let name = self.provider_of(id).map_or(id.clone(), |p| p.manifest.name.clone());
                ConsentView {
                    title: format!("Allow “{name}” to run?"),
                    command: self.provider_of(id).map(|p| p.manifest.command()),
                    ask: "It asks to:".into(),
                    permissions: words,
                    source: source_line(url),
                    also: Some(format!(
                        "This also allows other widgets from {} to run.",
                        crate::dashboard::source_name(url)
                    )),
                    confirm: "Allow".into(),
                    armed: c.armed,
                }
            }
            Asking::AllowOwn { id } => {
                let name = self.provider_of(id).map_or(id.clone(), |p| p.manifest.name.clone());
                ConsentView {
                    title: format!("Allow “{name}” to:"),
                    command: self.provider_of(id).map(|p| p.manifest.command()),
                    ask: String::new(),
                    permissions: words,
                    source: None,
                    also: None,
                    confirm: "Allow".into(),
                    armed: c.armed,
                }
            }
        }
    }
}

impl State {
    /// Reads each source's clone, and fetches those stale or `force`d, on the worker.
    pub fn refresh_sources(&mut self, force: bool) {
        let (Some(runtime), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone()) else {
            return;
        };
        let cache = cache();
        let offline = std::env::var_os("ISLE_NO_NETWORK").is_some();
        for url in self.isle.settings.widgets.sources.clone() {
            if self.isle.dashboard.fetching.contains(&url) {
                continue;
            }
            let known = self.isle.dashboard.browse.contains_key(&url);
            let fresh = self
                .isle
                .dashboard
                .memory
                .sources
                .get(&url)
                .and_then(|f| f.fetched_at)
                .is_some_and(|at| now_secs().saturating_sub(at) < FRESH);
            let network = !(offline && !url.starts_with("file://"));
            let fetch = network && (force || !fresh);
            if known && !fetch {
                continue;
            }
            self.isle.dashboard.fetching.insert(url.clone());
            let (to_loop, cache) = (to_loop.clone(), cache.clone());
            runtime.spawn(async move {
                let stop = Arc::new(AtomicBool::new(false));
                let timer = {
                    let stop = stop.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(FETCH_LIMIT).await;
                        stop.store(true, std::sync::atomic::Ordering::Relaxed);
                    })
                };
                let (u, c) = (url.clone(), cache.clone());
                let local = tokio::task::spawn_blocking(move || sources::read(&sources::clone_dir(&c, &u)).ok())
                    .await
                    .ok()
                    .flatten();
                if let Some(index) = local.clone() {
                    let u = url.clone();
                    to_loop.post(move |state| {
                        state.isle.dashboard.browse.entry(u).or_insert(Ok(index));
                        state.put_dashboard_frame();
                    });
                }
                let fetched: Option<Result<Index, String>> = if fetch {
                    let (u, c, s) = (url.clone(), cache.clone(), stop.clone());
                    Some(
                        tokio::task::spawn_blocking(move || sources::fetch(&u, &c, &s))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string())),
                    )
                } else {
                    None
                };
                timer.abort();
                to_loop.post(move |state| state.sources_fetched(&url, fetched, local.is_some()));
            });
        }
    }

    fn sources_fetched(&mut self, url: &str, fetched: Option<Result<Index, String>>, had_local: bool) {
        self.isle.dashboard.fetching.remove(url);
        if let Some(result) = fetched {
            let record = self.isle.dashboard.memory.sources.entry(url.to_owned()).or_default();
            match result {
                Ok(index) => {
                    record.fetched_at = Some(now_secs());
                    record.commit = Some(index.commit.clone());
                    record.error = None;
                    self.isle.dashboard.browse.insert(url.to_owned(), Ok(index));
                }
                Err(e) => {
                    tracing::warn!(url, "a widget source: {e}");
                    record.error = Some(e.clone());
                    // The last good index stays in Browse.
                    if !had_local && !self.isle.dashboard.browse.contains_key(url) {
                        self.isle.dashboard.browse.insert(url.to_owned(), Err(e));
                    }
                }
            }
            self.isle.save_widgets();
        }
        self.put_dashboard_frame();
    }

    fn consent(&mut self, asking: Asking, permissions: Vec<Grant>) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        open.consent = Some(Consent {
            asking,
            permissions,
            armed: false,
        });
        let arm = Duration::from_millis(self.isle.shell.theme().tokens.timing_prompt_arm_ms as u64);
        let _ = self.isle.loop_handle.insert_source(
            smithay::reexports::calloop::timer::Timer::from_duration(arm),
            |_, _, state| {
                if let Some(c) = state.isle.dashboard.open.as_mut().and_then(|o| o.consent.as_mut()) {
                    c.armed = true;
                }
                state.put_dashboard_frame();
                smithay::reexports::calloop::timer::TimeoutAction::Drop
            },
        );
        self.put_dashboard_frame();
    }

    /// Install from Browse: the consent panel.
    pub fn ask_install(&mut self, url: &str, id: &str) {
        let Some(listing) = self.isle.listing(url, id) else {
            return;
        };
        let new_source = !self.isle.settings.widget_source.get(url).is_some_and(|s| s.allowed);
        let permissions = listing.manifest.declared();
        self.consent(
            Asking::Install {
                url: url.to_owned(),
                listing,
                new_source,
            },
            permissions,
        );
    }

    /// Update: at once, or through the panel with only what it newly asks for.
    pub fn update_provider(&mut self, id: &str) {
        let Some((_, url, listing)) = self.isle.updates().into_iter().find(|(i, _, _)| i == id) else {
            return;
        };
        let granted = self.isle.granted(id);
        let new: Vec<Grant> = listing.manifest.declared().into_iter().filter(|g| !granted.contains(g)).collect();
        if new.is_empty() {
            self.install(&url, listing, Vec::new());
        } else {
            self.consent(Asking::Update { url, listing }, new);
        }
    }

    /// Allow…: a source turned off, or one's own provider's permissions.
    pub fn ask_allow(&mut self, id: &str) {
        let Some(p) = self.isle.provider_of(id).cloned() else {
            return;
        };
        let granted = self.isle.granted(&p.manifest.id);
        let declared = p.manifest.declared();
        match p.source {
            Some(url) => self.consent(
                Asking::AllowSource {
                    url,
                    id: p.manifest.id.clone(),
                },
                declared,
            ),
            None => {
                let missing: Vec<Grant> = declared.into_iter().filter(|g| !granted.contains(g)).collect();
                self.consent(Asking::AllowOwn { id: p.manifest.id.clone() }, missing);
            }
        }
    }

    fn grant(&mut self, id: &str, more: &[Grant]) {
        let mut granted: Vec<String> = self.isle.granted(id).iter().map(Grant::key).collect();
        for g in more {
            if !granted.contains(&g.key()) {
                granted.push(g.key());
            }
        }
        self.isle
            .write_setting(&format!("provider[{id}].granted"), isle_settings::Value::List(granted));
    }

    fn allow_source(&mut self, url: &str) {
        self.isle
            .write_setting(&format!("widget-source[{url}].allowed"), isle_settings::Value::Bool(true));
    }

    fn install(&mut self, url: &str, listing: Listing, grants: Vec<Grant>) {
        let (Some(runtime), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone()) else {
            return;
        };
        if !grants.is_empty() {
            self.grant(&listing.id, &grants);
        }
        let (url, cache, root) = (url.to_owned(), cache(), crate::providers::installed_folder());
        runtime.spawn_blocking(move || {
            let result = sources::install(&url, &cache, &listing, &root);
            to_loop.post(move |state| {
                if let Err(e) = result {
                    tracing::warn!(id = listing.id, "installing: {e}");
                }
                state.isle.dashboard.providers = crate::providers::scan();
                state.run_providers();
                state.put_dashboard_frame();
            });
        });
    }

    pub fn remove_provider(&mut self, id: &str) {
        let root = crate::providers::installed_folder();
        if let Err(e) = sources::remove(&root, id) {
            tracing::warn!(id, "removing: {e}");
        }
        self.isle.remove_setting(&format!("provider[{id}]"));
        self.isle.dashboard.providers = crate::providers::scan();
        self.run_providers();
        self.put_dashboard_frame();
    }

    pub fn consent_answer(&mut self, confirm: bool) {
        let Some(c) = self.isle.dashboard.open.as_mut().and_then(|o| o.consent.take()) else {
            return;
        };
        if confirm && !c.armed {
            if let Some(open) = self.isle.dashboard.open.as_mut() {
                open.consent = Some(c);
            }
            return;
        }
        if confirm {
            match c.asking {
                Asking::Install { url, listing, .. } => {
                    self.allow_source(&url);
                    self.install(&url, listing, c.permissions);
                }
                Asking::Update { url, listing } => self.install(&url, listing, c.permissions),
                Asking::AllowSource { url, id } => {
                    self.allow_source(&url);
                    self.grant(&id, &c.permissions);
                }
                Asking::AllowOwn { id } => self.grant(&id, &c.permissions),
            }
        }
        self.put_dashboard_frame();
    }

    /// The gallery's and Sources' buttons.
    pub fn gallery_button(&mut self, key: &str, action: &str) {
        let Some(g) = self.isle.dashboard.open.as_mut().and_then(|o| o.gallery.as_mut()) else {
            return;
        };
        match action {
            "sources" => {
                g.sources = Some(false);
                self.refresh_sources(false);
            }
            "sources-back" => g.sources = None,
            "add" => g.sources = Some(true),
            "add-source" => {
                g.sources = Some(false);
                let url = key.trim().to_owned();
                if !url.is_empty() && !self.isle.settings.widgets.sources.contains(&url) {
                    let mut list = self.isle.settings.widgets.sources.clone();
                    list.push(url);
                    self.isle.write_setting("widgets.sources", isle_settings::Value::List(list));
                }
            }
            "remove-source" => {
                let list: Vec<String> = self.isle.settings.widgets.sources.iter().filter(|u| *u != key).cloned().collect();
                self.isle.write_setting("widgets.sources", isle_settings::Value::List(list));
                self.isle.dashboard.browse.remove(key);
            }
            "allow-on" | "allow-off" => {
                self.isle.write_setting(
                    &format!("widget-source[{key}].allowed"),
                    isle_settings::Value::Bool(action == "allow-on"),
                );
            }
            "install" => {
                if let Some((id, url)) = key.split_once('@') {
                    let (id, url) = (id.to_owned(), url.to_owned());
                    self.ask_install(&url, &id);
                }
            }
            "update" => {
                let id = key.split_once('@').map_or(key, |(id, _)| id).to_owned();
                self.update_provider(&id);
            }
            "remove" => {
                let id = key.split_once('@').map_or(key, |(id, _)| id).to_owned();
                self.remove_provider(&id);
            }
            "allow" => self.ask_allow(key),
            // Change…: the one place grants change, after the dashboard closes.
            "permissions" => {
                let id = key.split_once('@').map_or(key, |(id, _)| id).to_owned();
                self.close_dashboard();
                self.open_settings(Some("privacy-security"), Some(&format!("provider:{id}")));
                return;
            }
            other => tracing::info!(key, action = other, "a gallery button not built yet"),
        }
        self.put_dashboard_frame();
    }
}

/// Routes the panel's two buttons.
pub fn consent_action(state: &mut State, action: &DashboardAction) -> bool {
    match action {
        DashboardAction::ConsentConfirm => state.consent_answer(true),
        DashboardAction::ConsentCancel => state.consent_answer(false),
        _ => return false,
    }
    true
}
