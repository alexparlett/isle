//! The app above its windows: one inbox for launches, menus and the compositor, the windows
//! it has open, the work that keeps it running without any, and its end (`apps.md`
//! Conventions 3–5).

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use freya::{
    prelude::*,
    winit::window::WindowId,
};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_util::StreamExt;
use isle_ui::ThemeContext;
use tracing::{info, warn};

use crate::{
    Delegate, Desktop, Spec, Worker, desktop,
    instance::Request,
    wayland::Extras,
    window::{Frame, WindowSpec},
};

/// A window the app opened, from the moment it asked for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowKey(pub(crate) u64);

/// Keeps the app running with no window open while it is held; dropped, it lets go.
pub struct Hold(Rc<Inner>);

impl Drop for Hold {
    fn drop(&mut self) {
        self.0.holds.set(self.0.holds.get() - 1);
        AppHandle(self.0.clone()).end_if_idle();
    }
}

/// What the inbox carries.
pub(crate) enum Inbound {
    Update(desktop::Update),
    Request(Request),
    /// A menu item chosen in a window's menu bar, by the menu's number and the item's id.
    Menu(u64, i32),
    /// A settings change to send.
    Write(desktop::Write),
    /// The compositor's answer to a change: its key, and why it was refused.
    Written(String, Option<String>),
}

/// A window as the app keeps it.
pub(crate) struct Open {
    pub id: Option<WindowId>,
    pub extras: Rc<RefCell<Option<Extras>>>,
    /// Runs a chosen menu item in the window.
    pub choose: Option<Rc<dyn Fn(i32)>>,
}

pub(crate) struct Inner {
    pub spec: Spec,
    pub worker: Worker,
    pub bus: Option<zbus::Connection>,
    pub desktop: Desktop,
    pub theme: ThemeContext,
    pub proxy: RefCell<Option<LaunchProxy>>,
    pub windows: RefCell<BTreeMap<WindowKey, Open>>,
    pub inbox: UnboundedSender<Inbound>,
    next: Cell<u64>,
    holds: Cell<usize>,
    /// The clipboard is set up once, by the first window.
    pub clipboard: Cell<bool>,
    delegate: RefCell<Option<Rc<dyn Delegate>>>,
}

/// The running app, for its delegate and any of its components.
#[derive(Clone)]
pub struct AppHandle(pub(crate) Rc<Inner>);

impl AppHandle {
    pub(crate) fn new(
        spec: Spec,
        worker: Worker,
        bus: Option<zbus::Connection>,
        desktop: Desktop,
        theme: ThemeContext,
    ) -> (Self, UnboundedReceiver<Inbound>) {
        let (inbox, rx) = unbounded();
        let inner = Inner {
            spec,
            worker,
            bus,
            desktop,
            theme,
            proxy: RefCell::new(None),
            windows: RefCell::default(),
            inbox,
            next: Cell::new(1),
            holds: Cell::new(0),
            clipboard: Cell::new(false),
            delegate: RefCell::new(None),
        };
        (Self(Rc::new(inner)), rx)
    }

    pub fn spec(&self) -> &Spec {
        &self.0.spec
    }

    pub fn desktop(&self) -> Desktop {
        self.0.desktop
    }

    pub fn worker(&self) -> Worker {
        self.0.worker.clone()
    }

    pub fn bus(&self) -> Option<zbus::Connection> {
        self.0.bus.clone()
    }

    /// Opens a window drawing `app`; it counts as open from now.
    pub fn open<A: App + 'static>(&self, spec: WindowSpec, app: A) -> WindowKey {
        let key = WindowKey(self.0.next.get());
        self.0.next.set(key.0 + 1);
        let extras: Rc<RefCell<Option<Extras>>> = Rc::default();
        self.0.windows.borrow_mut().insert(
            key,
            Open {
                id: None,
                extras: extras.clone(),
                choose: None,
            },
        );
        let config = Frame::config(self.clone(), key, spec, extras, app);
        match self.0.proxy.borrow().as_ref() {
            Some(proxy) => {
                let _ = proxy.post_callback(move |ctx| {
                    ctx.launch_window(config);
                });
            }
            None => warn!("no event loop to open a window on"),
        }
        key
    }

    /// The windows open or opening, oldest first.
    pub fn windows(&self) -> Vec<WindowKey> {
        self.0.windows.borrow().keys().copied().collect()
    }

    /// The window's id once it exists.
    pub fn window_id(&self, key: WindowKey) -> Option<WindowId> {
        self.0.windows.borrow().get(&key).and_then(|w| w.id)
    }

    pub fn close(&self, key: WindowKey) {
        let Some(id) = self.window_id(key) else { return };
        if let Some(proxy) = self.0.proxy.borrow().as_ref() {
            let _ = proxy.post_callback(move |ctx| {
                ctx.windows.remove(&id);
            });
        }
    }

    /// Raises the window with a token another launch handed over.
    pub fn activate(&self, key: WindowKey, token: &str) {
        if let Some(open) = self.0.windows.borrow().get(&key)
            && let Some(extras) = open.extras.borrow().as_ref()
        {
            extras.activate(token);
        }
    }

    /// Keeps the app running while the returned hold lives, though no window is open.
    pub fn hold(&self) -> Hold {
        self.0.holds.set(self.0.holds.get() + 1);
        Hold(self.0.clone())
    }

    /// Ends the app now.
    pub fn exit(&self) {
        info!("quitting");
        if let Some(proxy) = self.0.proxy.borrow().as_ref() {
            let _ = proxy.post_callback(|ctx| ctx.exit());
        }
    }

    /// Cmd+Q: the delegate may ask first and quit later itself.
    pub fn quit(&self) {
        let delegate = self.0.delegate.borrow().clone();
        if delegate.is_none_or(|d| d.quit(self)) {
            self.exit();
        }
    }

    /// The progress of the app's work on its switcher card, 0 to 1; `None` takes it away.
    pub fn progress(&self, progress: Option<f64>) {
        let Some(bus) = self.bus() else { return };
        let uri = format!("application://{}.desktop", self.0.spec.id);
        self.0.worker.spawn(async move {
            let mut props: std::collections::HashMap<&str, zvariant::Value> =
                std::collections::HashMap::from([("progress-visible", progress.is_some().into())]);
            if let Some(p) = progress {
                props.insert("progress", p.clamp(0.0, 1.0).into());
            }
            let sent = bus
                .emit_signal(
                    None::<()>,
                    "/io/github/alexparlett/Isle/LauncherEntry",
                    "com.canonical.Unity.LauncherEntry",
                    "Update",
                    &(uri, props),
                )
                .await;
            if let Err(err) = sent {
                warn!("the progress could not be sent: {err}");
            }
        });
    }

    pub(crate) fn opened(&self, key: WindowKey, id: WindowId, choose: Rc<dyn Fn(i32)>) {
        if let Some(open) = self.0.windows.borrow_mut().get_mut(&key) {
            open.id = Some(id);
            open.choose = Some(choose);
        }
    }

    pub(crate) fn closed(&self, key: WindowKey) {
        self.0.windows.borrow_mut().remove(&key);
        self.end_if_idle();
    }

    /// Ends the app once no window is open and nothing holds it (`apps.md` Conventions 4).
    fn end_if_idle(&self) {
        if self.0.windows.borrow().is_empty() && self.0.holds.get() == 0 {
            self.exit();
        }
    }

    /// Takes the inbox until the bus closes; runs on the event loop for the app's life.
    pub(crate) async fn serve(
        self,
        proxy: LaunchProxy,
        delegate: Rc<dyn Delegate>,
        mut inbox: UnboundedReceiver<Inbound>,
    ) {
        *self.0.proxy.borrow_mut() = Some(proxy);
        *self.0.delegate.borrow_mut() = Some(delegate.clone());
        let mut desktop = self.0.desktop;
        let mut theme = self.0.theme;
        while let Some(message) = inbox.next().await {
            match message {
                Inbound::Update(desktop::Update::Theme(info)) => {
                    theme.0.set(crate::resolve(&info));
                    if *desktop.tokens_text.peek() != info.tokens {
                        desktop.tokens_text.set(info.tokens.clone());
                    }
                }
                Inbound::Update(desktop::Update::TokensStatus(status)) => {
                    desktop.tokens_status.set(Some(status));
                }
                Inbound::Update(desktop::Update::Values(values)) => desktop.take_values(values),
                Inbound::Update(desktop::Update::Status(status)) => {
                    desktop.status.set(Some(status));
                }
                Inbound::Request(request) => {
                    info!(?request, "asked");
                    delegate.request(&self, request);
                }
                Inbound::Menu(menu, item) => {
                    let choose = self
                        .0
                        .windows
                        .borrow()
                        .get(&WindowKey(menu))
                        .and_then(|w| w.choose.clone());
                    if let Some(choose) = choose {
                        choose(item);
                    }
                }
                Inbound::Write(write) => self.send(write),
                Inbound::Written(key, refusal) => {
                    if let Some(why) = refusal {
                        desktop.refused.set(Some((key.clone(), why)));
                    }
                    let next = {
                        let mut flight = desktop.flight.write();
                        flight.sending.remove(&key);
                        flight.waiting.remove(&key)
                    };
                    if let Some(next) = next {
                        self.send(next);
                    }
                }
            }
        }
    }

    /// Sends a change, one a key at a time: a later change to a key waits for the earlier.
    fn send(&self, write: desktop::Write) {
        let mut flight = self.0.desktop.flight;
        let key = write.key().to_owned();
        if flight.peek().sending.contains(&key) {
            flight.write().waiting.insert(key, write);
            return;
        }
        let Some(bus) = self.bus() else {
            warn!(?write, "not on the bus, the change is lost");
            return;
        };
        flight.write().sending.insert(key.clone());
        let inbox = self.0.inbox.clone();
        self.0.worker.spawn(async move {
            let refusal = desktop::send(bus, write.clone()).await.err();
            if let Some(why) = &refusal {
                warn!(?write, "refused: {why}");
            }
            let _ = inbox.unbounded_send(Inbound::Written(key, refusal));
        });
    }
}

/// The running app, from any of its components.
pub fn app() -> AppHandle {
    use_consume::<AppHandle>()
}
