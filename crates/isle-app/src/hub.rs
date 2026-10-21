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

impl WindowKey {
    /// Its number, unique for the app's life.
    pub fn number(self) -> u64 {
        self.0
    }
}

/// Keeps the app running with no window open while it is held; dropped, it lets go.
pub struct Hold(Rc<Inner>);

impl Drop for Hold {
    fn drop(&mut self) {
        self.0.holds.set(self.0.holds.get() - 1);
        AppHandle(self.0.clone()).end_if_idle();
    }
}

/// Sends requests to the app's delegate, from any thread.
#[derive(Clone)]
pub struct Requester(UnboundedSender<Inbound>);

impl Requester {
    pub fn send(&self, request: Request) {
        let _ = self.0.unbounded_send(Inbound::Request(request));
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
    /// Drags over the window, from the data device.
    pub drags: Option<UnboundedSender<crate::data::DragEvent>>,
    pub extras: Rc<RefCell<Option<Extras>>>,
    /// Runs a chosen menu item in the window.
    pub choose: Option<Rc<dyn Fn(i32)>>,
    /// Closed before it was created: it closes as it opens.
    pub closing: bool,
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
    /// The data device, started by the first window on winit's display.
    pub data: RefCell<Option<crate::data::Data>>,
    delegate: RefCell<Option<Rc<dyn Delegate>>>,
    /// Tasks of the app's own, run beside the inbox for the app's life.
    tasks: UnboundedSender<Task>,
    spawned: RefCell<Option<UnboundedReceiver<Task>>>,
}

type Task = std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>;

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
        let (tasks, spawned) = unbounded();
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
            data: RefCell::new(None),
            delegate: RefCell::new(None),
            tasks,
            spawned: RefCell::new(Some(spawned)),
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

    /// Runs `task` on the event loop for as long as it takes, whatever windows close: work
    /// that outlives the window that started it.
    pub fn spawn(&self, task: impl std::future::Future<Output = ()> + 'static) {
        let _ = self.0.tasks.unbounded_send(Box::pin(task));
    }

    /// Hands requests to the delegate from any thread: the app's own D-Bus objects.
    pub fn requester(&self) -> Requester {
        Requester(self.0.inbox.clone())
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
                drags: None,
                extras: extras.clone(),
                choose: None,
                closing: false,
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
        let Some(id) = self.window_id(key) else {
            if let Some(open) = self.0.windows.borrow_mut().get_mut(&key) {
                open.closing = true;
            }
            return;
        };
        if let Some(proxy) = self.0.proxy.borrow().as_ref() {
            let _ = proxy.post_callback(move |ctx| {
                ctx.windows.remove(&id);
            });
        }
    }

    /// Raises the window with a token another launch handed over.
    /// A window's xdg-foreign handle, `wayland:<handle>` as portals take it.
    pub fn export(&self, key: WindowKey) -> Option<String> {
        let windows = self.0.windows.borrow();
        let open = windows.get(&key)?;
        let extras = open.extras.borrow();
        extras.as_ref()?.export().map(|h| format!("wayland:{h}"))
    }

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

    pub(crate) fn opened(
        &self,
        key: WindowKey,
        id: WindowId,
        choose: Rc<dyn Fn(i32)>,
        drags: UnboundedSender<crate::data::DragEvent>,
    ) {
        let closing = match self.0.windows.borrow_mut().get_mut(&key) {
            Some(open) => {
                open.id = Some(id);
                open.choose = Some(choose);
                open.drags = Some(drags);
                open.closing
            }
            None => false,
        };
        if closing {
            self.close(key);
        }
    }

    /// The app's data device: the selection in any type, and drags.
    pub fn data(&self) -> Option<crate::data::Data> {
        self.0.data.borrow().clone()
    }

    /// Starts the data device on winit's display, once, and hands each drag event to the
    /// window it is over.
    pub(crate) fn start_data(&self, display: *mut std::ffi::c_void) {
        if self.0.data.borrow().is_some() {
            return;
        }
        // The display is winit's, alive as long as the app.
        let Some((data, mut events)) = (unsafe { crate::data::Data::start(display) }) else {
            warn!("no data device: no clipboard of files and no drags");
            return;
        };
        *self.0.data.borrow_mut() = Some(data.clone());
        GlobalContexts::get().insert_context(freya::clipboard::Clipboard::create(Some(Box::new(
            crate::data::Clipboard(data),
        ))));
        let app = self.clone();
        self.spawn(async move {
            // The window a drag was last over, for its end.
            let mut over: Option<u32> = None;
            while let Some(event) = events.next().await {
                use crate::data::DragEvent as E;
                let surface = match &event {
                    E::Enter { surface, .. } | E::Motion { surface, .. } | E::Drop { surface, .. } => Some(*surface),
                    E::Leave { surface } => Some(*surface).filter(|s| *s != 0).or(over),
                    E::Ended { .. } => None,
                };
                over = match &event {
                    E::Leave { .. } | E::Drop { .. } => None,
                    _ => surface.or(over),
                };
                let windows = app.0.windows.borrow();
                for open in windows.values() {
                    let mine = open.extras.borrow().as_ref().map(|e| e.surface_id());
                    if surface.is_none() || surface == mine {
                        if let Some(tx) = &open.drags {
                            let _ = tx.unbounded_send(event.clone());
                        }
                    }
                }
            }
        });
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
        delegate.started(&self);
        let mut desktop = self.0.desktop;
        let mut theme = self.0.theme;
        let mut spawned = self.0.spawned.borrow_mut().take().expect("served once");
        let mut tasks = futures_util::stream::FuturesUnordered::<Task>::new();
        loop {
            let next = std::future::poll_fn(|cx| {
                use std::task::Poll;
                while let Poll::Ready(Some(task)) = spawned.poll_next_unpin(cx) {
                    tasks.push(task);
                }
                while let Poll::Ready(Some(())) = tasks.poll_next_unpin(cx) {}
                inbox.poll_next_unpin(cx)
            })
            .await;
            let Some(message) = next else { break };
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
