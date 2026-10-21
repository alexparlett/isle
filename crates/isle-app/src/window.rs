//! One of an app's windows: its configuration, the contexts its tree reads, its own menu bar
//! and Wayland extras, and its commands.

use std::{cell::RefCell, rc::Rc};

use freya::prelude::*;
use isle_ui::GlassShapes;
use tracing::warn;

use crate::{
    Blurs, Chrome, SessionBus, Table, clipboard, commands,
    hub::{AppHandle, Inbound, WindowKey},
    wayland::Extras,
};

/// A window an app opens.
#[derive(Clone, Debug)]
pub struct WindowSpec {
    pub title: String,
    pub size: (f64, f64),
    pub min_size: (f64, f64),
    /// The largest it grows to; a filled or tiled window takes this, centred.
    pub max_size: Option<(f64, f64)>,
    /// The lights' group from the window's top leading corner, in points, drawn by the
    /// compositor over the app's own toolbar; `None` keeps the compositor's title bar.
    pub lights: Option<(f64, f64)>,
    /// The window's box the compositor blurs behind, from its top leading corner, in points.
    pub backdrop: Option<(f64, f64, f64, f64)>,
    /// The activation token the launch that asked for it carried.
    pub token: Option<String>,
}

impl WindowSpec {
    pub fn new(title: impl Into<String>, size: (f64, f64), min_size: (f64, f64)) -> Self {
        Self {
            title: title.into(),
            size,
            min_size,
            max_size: None,
            lights: None,
            backdrop: None,
            token: None,
        }
    }

    pub fn lights(mut self, inset: (f64, f64)) -> Self {
        self.lights = Some(inset);
        self
    }

    pub fn max_size(mut self, size: (f64, f64)) -> Self {
        self.max_size = Some(size);
        self
    }

    pub fn backdrop(mut self, rect: (f64, f64, f64, f64)) -> Self {
        self.backdrop = Some(rect);
        self
    }

    pub fn token(mut self, token: Option<String>) -> Self {
        self.token = token;
        self
    }
}

/// The window an element is in, as its app knows it.
pub fn this_window() -> WindowKey {
    use_consume::<WindowKey>()
}

pub(crate) struct Frame<A> {
    app: AppHandle,
    key: WindowKey,
    spec: WindowSpec,
    extras: Rc<RefCell<Option<Extras>>>,
    inner: A,
}

impl<A: App + 'static> Frame<A> {
    pub fn config(
        app: AppHandle,
        key: WindowKey,
        spec: WindowSpec,
        extras: Rc<RefCell<Option<Extras>>>,
        inner: A,
    ) -> WindowConfig {
        let id = app.spec().id;
        let gpu = cfg!(feature = "gpu") && std::env::var_os("FREYA_RENDERER").is_some();
        let renderer = if gpu { RendererPreference::Auto } else { RendererPreference::Software };
        let (lights, backdrop, token) = (spec.lights, spec.backdrop, spec.token.clone());
        let (size, min_size, max_size) = (spec.size, spec.min_size, spec.max_size);
        // winit takes a static title; a window's title is set once and lives as long.
        let title: &'static str = Box::leak(spec.title.clone().into_boxed_str());
        let frame = Frame {
            app,
            key,
            spec,
            extras: extras.clone(),
            inner,
        };
        let mut window = WindowConfig::new_app(frame);
        if let Some((w, h)) = max_size {
            window = window.with_max_size(w, h);
        }
        if backdrop.is_some() {
            window = window.with_background(Color::TRANSPARENT);
        }
        window
            .with_app_id(id)
            .with_title(title)
            .with_size(size.0, size.1)
            .with_min_size(min_size.0, min_size.1)
            .with_renderer(renderer)
            .with_transparency(backdrop.is_some())
            .with_window_attributes(move |attributes, _| match token {
                Some(token) => {
                    use freya::winit::{
                        platform::startup_notify::WindowAttributesExtStartupNotify,
                        window::ActivationToken,
                    };
                    attributes.with_activation_token(ActivationToken::from_raw(token))
                }
                None => attributes,
            })
            .with_window_handle(move |window| {
                *extras.borrow_mut() = Extras::connect(window, lights, backdrop);
            })
    }
}

/// The path a window's menu bar is served at.
fn menu_path(key: WindowKey) -> String {
    format!("/MenuBar/{}", key.0)
}

impl<A: App + 'static> App for Frame<A> {
    fn render(&self) -> impl IntoElement {
        let app = self.app.clone();
        let key = self.key;
        // Freya's editing and scrolling follow the Mac's conventions, Cmd its primary.
        use_provide_context(|| TargetPlatform::MacOs);
        use_provide_context(|| app.clone());
        use_provide_context(|| key);
        use_provide_context(|| app.worker());
        use_provide_context(|| SessionBus(app.bus()));
        use_provide_context(|| Chrome(self.extras.clone()));
        use_provide_context(|| {
            let asked = self.spec.backdrop.is_some();
            Blurs(asked && self.extras.borrow().as_ref().is_some_and(|e| e.blurs()))
        });
        use_provide_context(|| GlassShapes {
            draw_tint: true,
            ..Default::default()
        });
        use_provide_context(|| app.0.theme);
        // A window's commands are its own; the rest of the desktop's state is the app's.
        let desktop = use_provide_context(|| crate::Desktop {
            commands: State::create(Vec::new()),
            ..app.desktop()
        });
        let table = use_provide_context(|| Table {
            app: State::create(Vec::new()),
        });
        let all = use_memo({
            let spec = app.spec().clone();
            move || {
                let mut all = commands::standard(spec.title, spec.preferences);
                all.extend(table.app.read().iter().cloned());
                all
            }
        });
        let menubar = use_hook(|| {
            let bus = app.bus()?;
            let inbox = app.0.inbox.clone();
            let menubar = isle_menus::export::MenuBar::new(
                Vec::new(),
                std::sync::Arc::new(move |id| {
                    let _ = inbox.unbounded_send(Inbound::Menu(key.0, id));
                }),
            );
            let served = app.worker().spawn({
                let (bus, menubar, path) = (bus.clone(), menubar.clone(), menu_path(key));
                async move { bus.object_server().at(path.as_str(), menubar).await }
            });
            let (extras, path) = (self.extras.clone(), menu_path(key));
            let name = bus.unique_name().map(|n| n.to_string());
            spawn(async move {
                match served.await {
                    Ok(Ok(_)) => {
                        if let (Some(extras), Some(name)) = (extras.borrow().as_ref(), name) {
                            extras.set_menu(&name, &path);
                        }
                    }
                    Ok(Err(err)) => warn!("the menus could not be served: {err}"),
                    Err(err) => warn!("the menus could not be served: {err}"),
                }
            });
            Some((bus, menubar))
        });
        use_side_effect({
            let menubar = menubar.clone();
            let worker = app.worker();
            let title = app.spec().title;
            move || {
                let menus = commands::menus(title, &all.read());
                if let Some((bus, menubar)) = menubar.clone() {
                    worker.spawn(async move {
                        if let Err(err) = menubar.update(&bus, &menu_path(key), menus).await {
                            warn!("the menus could not be updated: {err}");
                        }
                    });
                }
            }
        });
        use_side_effect({
            let inbox = app.0.inbox.clone();
            move || {
                if desktop.writes.read().is_empty() {
                    return;
                }
                let mut queue = desktop.writes;
                for write in std::mem::take(&mut *queue.write()) {
                    let _ = inbox.unbounded_send(Inbound::Write(write));
                }
            }
        });
        let run = Run {
            app: app.clone(),
            key,
            platform: Platform::get(),
            window: Platform::window_id(),
            desktop,
        };
        use_hook(|| {
            let run = run.clone();
            app.opened(
                key,
                run.window,
                Rc::new(move |id| {
                    let command = usize::try_from(id - 1)
                        .ok()
                        .and_then(|i| all.peek().get(i).cloned());
                    if let Some(command) = command.filter(|c| c.enabled) {
                        run.chosen(&command);
                    }
                }),
            );
            if !app.0.clipboard.replace(true)
                && let Some(extras) = self.extras.borrow().as_ref()
            {
                // The display is winit's, alive as long as the app.
                let provider = unsafe { clipboard::WaylandClipboard::new(extras.display) };
                GlobalContexts::get().insert_context(freya::clipboard::Clipboard::create(Some(
                    Box::new(provider),
                )));
            }
        });
        use_drop({
            let app = app.clone();
            move || app.closed(key)
        });
        rect()
            .expanded()
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                let command = all
                    .peek()
                    .iter()
                    .find(|c| {
                        c.enabled
                            && !c.for_field
                            && c.chord.is_some_and(|k| k.matches(&e.key, e.modifiers))
                    })
                    .cloned();
                if let Some(command) = command {
                    run.run(&command.id);
                }
            })
            .child(self.inner.render())
    }
}

/// Carries out commands, from a menu or a key.
#[derive(Clone)]
struct Run {
    app: AppHandle,
    key: WindowKey,
    platform: Platform,
    window: freya::winit::window::WindowId,
    desktop: crate::Desktop,
}

impl Run {
    /// A menu item: one with a chord arrives as its key press, as the key itself would.
    fn chosen(&self, command: &commands::Command) {
        match command.chord {
            Some(chord) => {
                let key = chord.key.key();
                let modifiers = chord.modifiers();
                let _ = self.platform.post_callback(move |window, ctx: &mut RendererContext| {
                    ctx.send_key_press(Some(window), key, Code::Unidentified, modifiers);
                });
            }
            None => self.run(&command.id),
        }
    }

    fn run(&self, id: &str) {
        tracing::info!(id, "command");
        let window = self.window;
        match id {
            "quit" => self.app.quit(),
            "close" => self.app.close(self.key),
            "minimise" => self.platform.with_window(window, |w| w.set_minimized(true)),
            "zoom" => self.platform.with_window(window, |w| w.set_maximized(!w.is_maximized())),
            "hide" | "hide-others" | "show-all" => {
                let Some(bus) = self.app.bus() else { return };
                let (me, id) = (self.app.spec().id, id.to_owned());
                self.app.worker().spawn(async move {
                    let Ok(windows) = isle_ipc::windows::WindowsProxy::new(&bus).await else {
                        return;
                    };
                    let apps = windows.apps().await.unwrap_or_default();
                    for app in apps {
                        let mine = app.app == me;
                        let hide = match id.as_str() {
                            "hide" => mine.then_some(true),
                            "hide-others" => (!mine).then_some(true),
                            _ => Some(false),
                        };
                        if let Some(hide) = hide {
                            let _ = windows.set_app_hidden(&app.app, hide).await;
                        }
                    }
                });
            }
            other => {
                let mut commands = self.desktop.commands;
                commands.write().push(other.to_owned());
            }
        }
    }
}
