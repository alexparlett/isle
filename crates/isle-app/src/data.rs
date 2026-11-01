//! The app's `wl_data_device`, on a thread and queue of its own beside winit's: the selection
//! in any type an app offers (files as `text/uri-list` and `x-special/gnome-copied-files`,
//! text as `text/plain`), and drags out of and onto the app's windows (`p15-files.md`
//! choice 5). winit reports neither; smithay-clipboard carried text alone. Its thread also
//! hears whether each window is shown (`isle_visibility_v1`), which winit drops.

use std::{
    io::{Read, Write},
    os::fd::OwnedFd,
    sync::{Arc, mpsc},
    time::Duration,
};

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use smithay_client_toolkit::{
    data_device_manager::{
        DataDeviceManagerState, WritePipe,
        data_device::{DataDevice, DataDeviceHandler},
        data_offer::{DataOfferHandler, DragOffer},
        data_source::{CopyPasteSource, DataSourceHandler, DragSource},
    },
    delegate_data_device, delegate_pointer, delegate_registry, delegate_seat,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
};
use wayland_backend::client::Backend;

use crate::wayland::isle_titlebar::{
    isle_titlebar_manager_v1::IsleTitlebarManagerV1,
    isle_visibility_v1::{self, IsleVisibilityV1},
};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    globals::registry_queue_init,
    protocol::{
        wl_data_device::WlDataDevice,
        wl_data_device_manager::DndAction,
        wl_data_source::WlDataSource,
        wl_keyboard::{self, WlKeyboard},
        wl_pointer::WlPointer,
        wl_seat::WlSeat,
        wl_surface::WlSurface,
    },
};

/// What one item of the selection or a drag holds, in each type it is offered as.
pub type Contents = Vec<(String, Vec<u8>)>;

/// A drag over one of the app's windows, by its surface's protocol id.
#[derive(Clone, Debug, PartialEq)]
pub enum DragEvent {
    Enter { surface: u32, x: f64, y: f64, types: Vec<String> },
    Motion { surface: u32, x: f64, y: f64 },
    Leave { surface: u32 },
    /// Dropped: what it carried in the type asked for, and whether it was this app's own.
    Drop { surface: u32, x: f64, y: f64, data: Vec<u8>, own: bool, action: Action },
    /// A drag from this app ended, dropped or not.
    Ended { dropped: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Copy,
    Move,
}

/// How a secret selection ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretEnd {
    /// Another copy, this app's or another's, took the selection.
    Replaced,
    /// The app withdrew it.
    Withdrawn,
}

enum Ask {
    SetSelection(Contents),
    /// A selection whose contents are zeroed when it ends, and which says how it ended.
    SetSecret(Contents, futures_channel::oneshot::Sender<SecretEnd>),
    /// Ends the secret selection, if it is still the selection.
    WithdrawSecret,
    ReadSelection(Vec<String>, mpsc::Sender<Option<(String, Vec<u8>)>>),
    StartDrag { surface: u32, contents: Contents },
    /// The drop target's type of choice for drags onto the app.
    Accept(Vec<String>),
    /// Tells the app whether the window of this surface is shown.
    WatchShown(WlSurface),
}

/// A window's surface by its protocol id, and whether it is shown.
pub type ShownEvent = (u32, bool);

struct Inner {
    asks: mpsc::Sender<Ask>,
    wake: OwnedFd,
}

/// The app's data device; one per app.
#[derive(Clone)]
pub struct Data(Arc<Inner>);

impl Data {
    /// Starts the thread on winit's display; `None` off Wayland or without a data device.
    ///
    /// # Safety
    /// `display` is a live `wl_display` that outlives the app.
    pub unsafe fn start(
        display: *mut std::ffi::c_void,
    ) -> Option<(Data, UnboundedReceiver<DragEvent>, UnboundedReceiver<ShownEvent>)> {
        let backend = unsafe { Backend::from_foreign_display(display.cast()) };
        let conn = Connection::from_backend(backend);
        let (globals, queue): (_, EventQueue<State>) = registry_queue_init(&conn).ok()?;
        let qh = queue.handle();
        let manager = DataDeviceManagerState::bind(&globals, &qh).ok()?;
        let (events, rx) = unbounded();
        let (shown, shown_rx) = unbounded();
        let titlebars = globals.bind::<IsleTitlebarManagerV1, _, _>(&qh, 4..=4, ()).ok();
        let state = State {
            titlebars,
            shown,
            registry: RegistryState::new(&globals),
            seats: SeatState::new(&globals, &qh),
            manager,
            device: None,
            pointer: None,
            keyboard: None,
            serial: 0,
            press: None,
            selection: None,
            secret: None,
            drag: None,
            accepts: vec!["text/uri-list".into()],
            events,
            dragging_own: false,
        };
        let (asks, asked) = mpsc::channel();
        let wake = rustix::event::eventfd(0, rustix::event::EventfdFlags::CLOEXEC | rustix::event::EventfdFlags::NONBLOCK).ok()?;
        let wake_read = wake.try_clone().ok()?;
        std::thread::Builder::new()
            .name("isle-app-data".into())
            .spawn(move || run(conn, queue, state, asked, wake_read))
            .ok()?;
        Some((Data(Arc::new(Inner { asks, wake })), rx, shown_rx))
    }

    /// Reports whether `surface`'s window is shown, on the receiver `start` gave.
    pub fn watch_shown(&self, surface: WlSurface) {
        self.ask(Ask::WatchShown(surface));
    }

    fn ask(&self, ask: Ask) {
        let _ = self.0.asks.send(ask);
        let _ = rustix::io::write(&self.0.wake, &1u64.to_ne_bytes());
    }

    /// Makes `contents` the selection, as the last key or click allows.
    pub fn set_selection(&self, contents: Contents) {
        self.ask(Ask::SetSelection(contents));
    }

    /// Makes `contents` the selection as a secret: zeroed when it ends, and the receiver
    /// says whether it was replaced or withdrawn.
    pub fn set_secret_selection(&self, contents: Contents) -> futures_channel::oneshot::Receiver<SecretEnd> {
        let (tx, rx) = futures_channel::oneshot::channel();
        self.ask(Ask::SetSecret(contents, tx));
        rx
    }

    /// Withdraws the secret selection; the compositor then clears the selection.
    pub fn withdraw_secret(&self) {
        self.ask(Ask::WithdrawSecret);
    }

    /// The selection in the first of `types` it is offered as; waits at most a second.
    pub fn read_selection(&self, types: &[&str]) -> Option<(String, Vec<u8>)> {
        let (tx, rx) = mpsc::channel();
        self.ask(Ask::ReadSelection(types.iter().map(|t| t.to_string()).collect(), tx));
        rx.recv_timeout(Duration::from_secs(1)).ok().flatten()
    }

    /// Starts a drag of `contents` from the surface the button was pressed in.
    pub fn start_drag(&self, surface: u32, contents: Contents) {
        self.ask(Ask::StartDrag { surface, contents });
    }

    /// The types the app takes in a drop, in order of preference.
    pub fn accept(&self, types: Vec<String>) {
        self.ask(Ask::Accept(types));
    }
}

struct State {
    /// The compositor's manager, at the version with visibility; none on another compositor.
    titlebars: Option<IsleTitlebarManagerV1>,
    shown: UnboundedSender<ShownEvent>,
    registry: RegistryState,
    seats: SeatState,
    manager: DataDeviceManagerState,
    device: Option<DataDevice>,
    pointer: Option<WlPointer>,
    keyboard: Option<WlKeyboard>,
    /// The latest input serial, for setting the selection.
    serial: u32,
    /// The last button press: its serial and surface, for starting a drag.
    press: Option<(u32, WlSurface)>,
    selection: Option<(CopyPasteSource, Contents)>,
    /// Set while the selection is a secret.
    secret: Option<futures_channel::oneshot::Sender<SecretEnd>>,
    drag: Option<(DragSource, Contents)>,
    accepts: Vec<String>,
    events: UnboundedSender<DragEvent>,
    /// The drag over the app is its own.
    dragging_own: bool,
}

fn run(conn: Connection, mut queue: EventQueue<State>, mut state: State, asked: mpsc::Receiver<Ask>, wake: OwnedFd) {
    let qh = queue.handle();
    serve(&conn, &mut queue, &qh, &mut state, &asked, &wake);
    // The connection failed because winit has closed the display it borrows; destroying the
    // proxies now would write through the freed `wl_display`.
    std::mem::forget((state, qh, queue, conn));
}

fn serve(conn: &Connection, queue: &mut EventQueue<State>, qh: &QueueHandle<State>, state: &mut State, asked: &mpsc::Receiver<Ask>, wake: &OwnedFd) {
    loop {
        if queue.dispatch_pending(state).is_err() {
            return;
        }
        let _ = conn.flush();
        let Some(guard) = queue.prepare_read() else { continue };
        let fd = guard.connection_fd();
        let mut fds = [
            rustix::event::PollFd::new(&fd, rustix::event::PollFlags::IN),
            rustix::event::PollFd::new(wake, rustix::event::PollFlags::IN),
        ];
        if rustix::event::poll(&mut fds, None).is_err() {
            continue;
        }
        let (wayland, woken) = (!fds[0].revents().is_empty(), !fds[1].revents().is_empty());
        drop(fds);
        if wayland {
            if guard.read().is_err() {
                return;
            }
        } else {
            drop(guard);
        }
        if woken {
            let mut buf = [0u8; 8];
            let _ = rustix::io::read(wake, &mut buf);
            while let Ok(ask) = asked.try_recv() {
                state.handle(conn, qh, ask);
            }
        }
    }
}

/// Reads a pipe to its end on a thread of its own, giving up after a second.
fn read_pipe(mut pipe: impl Read + Send + 'static) -> Option<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut data = Vec::new();
        let _ = tx.send(pipe.read_to_end(&mut data).ok().map(|_| data));
    });
    rx.recv_timeout(Duration::from_secs(1)).ok().flatten()
}

impl State {
    fn handle(&mut self, conn: &Connection, qh: &QueueHandle<Self>, ask: Ask) {
        if let Ask::WatchShown(surface) = ask {
            if let Some(manager) = &self.titlebars {
                manager.get_visibility(&surface, qh, surface.id().protocol_id());
            }
            return;
        }
        if let Ask::WithdrawSecret = ask {
            self.end_secret(SecretEnd::Withdrawn);
            let _ = conn.flush();
            return;
        }
        if matches!(ask, Ask::SetSelection(_) | Ask::SetSecret(..)) {
            self.end_secret(SecretEnd::Replaced);
        }
        let Some(device) = &self.device else { return };
        match ask {
            Ask::SetSelection(contents) => {
                let source = self.manager.create_copy_paste_source(qh, contents.iter().map(|(t, _)| t.clone()));
                source.set_selection(device, self.serial);
                self.selection = Some((source, contents));
            }
            Ask::SetSecret(contents, ended) => {
                let source = self.manager.create_copy_paste_source(qh, contents.iter().map(|(t, _)| t.clone()));
                source.set_selection(device, self.serial);
                self.selection = Some((source, contents));
                self.secret = Some(ended);
            }
            Ask::WithdrawSecret => {}
            Ask::ReadSelection(types, reply) => {
                if let Some((_, contents)) = &self.selection {
                    let found = types.iter().find_map(|t| contents.iter().find(|(c, _)| c == t).cloned());
                    let _ = reply.send(found);
                    return;
                }
                let Some(offer) = device.data().selection_offer() else {
                    let _ = reply.send(None);
                    return;
                };
                let offered = offer.with_mime_types(|m| m.to_vec());
                let Some(kind) = types.iter().find(|t| offered.contains(t)).cloned() else {
                    let _ = reply.send(None);
                    return;
                };
                match offer.receive(kind.clone()) {
                    Ok(pipe) => {
                        let _ = conn.flush();
                        std::thread::spawn(move || {
                            let _ = reply.send(read_pipe(pipe).map(|d| (kind, d)));
                        });
                    }
                    Err(_) => {
                        let _ = reply.send(None);
                    }
                }
            }
            Ask::StartDrag { surface, contents } => {
                // A drag starts from the surface its button went down in.
                let Some((serial, origin)) = self.press.clone().filter(|(_, s)| s.id().protocol_id() == surface) else {
                    return;
                };
                let source = self.manager.create_drag_and_drop_source(qh, contents.iter().map(|(t, _)| t.clone()), DndAction::Copy | DndAction::Move);
                source.start_drag(device, &origin, None, serial);
                self.dragging_own = true;
                self.drag = Some((source, contents));
            }
            Ask::Accept(types) => self.accepts = types,
            Ask::WatchShown(_) => {}
        }
    }

    /// Zeroes and drops a secret selection, which destroys its source, and says how it ended.
    fn end_secret(&mut self, end: SecretEnd) {
        let Some(ended) = self.secret.take() else { return };
        if let Some((_, mut contents)) = self.selection.take() {
            for (_, data) in &mut contents {
                zeroize::Zeroize::zeroize(data);
            }
        }
        let _ = ended.send(end);
    }

    fn send(&self, event: DragEvent) {
        let _ = self.events.unbounded_send(event);
    }

    fn offer_type(&self, offer: &DragOffer) -> Option<String> {
        let offered = offer.with_mime_types(|m| m.to_vec());
        self.accepts.iter().find(|t| offered.contains(t)).cloned()
    }
}

impl DataDeviceHandler for State {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, device: &WlDataDevice, x: f64, y: f64, surface: &WlSurface) {
        let Some(offer) = self.device.as_ref().filter(|d| d.inner() == device).and_then(|d| d.data().drag_offer()) else {
            return;
        };
        let kind = self.offer_type(&offer);
        offer.accept_mime_type(offer.serial, kind.clone());
        if kind.is_some() {
            offer.set_actions(DndAction::Copy | DndAction::Move, DndAction::Copy);
        }
        self.send(DragEvent::Enter {
            surface: surface.id().protocol_id(),
            x,
            y,
            types: offer.with_mime_types(|m| m.to_vec()),
        });
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, device: &WlDataDevice) {
        let surface = self
            .device
            .as_ref()
            .filter(|d| d.inner() == device)
            .and_then(|d| d.data().drag_offer())
            .map(|o| o.surface.id().protocol_id())
            .unwrap_or(0);
        self.send(DragEvent::Leave { surface });
    }

    fn motion(&mut self, _: &Connection, _: &QueueHandle<Self>, device: &WlDataDevice, x: f64, y: f64) {
        if let Some(offer) = self.device.as_ref().filter(|d| d.inner() == device).and_then(|d| d.data().drag_offer()) {
            self.send(DragEvent::Motion { surface: offer.surface.id().protocol_id(), x, y });
        }
    }

    fn selection(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataDevice) {}

    fn drop_performed(&mut self, conn: &Connection, _: &QueueHandle<Self>, device: &WlDataDevice) {
        let Some(offer) = self.device.as_ref().filter(|d| d.inner() == device).and_then(|d| d.data().drag_offer()) else {
            return;
        };
        let surface = offer.surface.id().protocol_id();
        let (x, y) = (offer.x, offer.y);
        let action = if offer.selected_action == DndAction::Move { Action::Move } else { Action::Copy };
        let own = self.dragging_own;
        // A drop of the app's own drag is read from what it offered; reading the pipe here
        // would wait on this thread's own answer.
        if own && let Some((_, contents)) = &self.drag {
            let data = self
                .accepts
                .iter()
                .find_map(|t| contents.iter().find(|(c, _)| c == t).map(|(_, d)| d.clone()))
                .unwrap_or_default();
            offer.finish();
            offer.destroy();
            self.send(DragEvent::Drop { surface, x, y, data, own, action });
            return;
        }
        let Some(kind) = self.offer_type(&offer) else {
            offer.destroy();
            return;
        };
        let Ok(pipe) = offer.receive(kind) else { return };
        let _ = conn.flush();
        let events = self.events.clone();
        let data = read_pipe(pipe).unwrap_or_default();
        offer.finish();
        offer.destroy();
        let _ = events.unbounded_send(DragEvent::Drop { surface, x, y, data, own, action });
    }
}

impl DataOfferHandler for State {
    fn source_actions(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &mut DragOffer, _: DndAction) {}
    fn selected_action(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &mut DragOffer, _: DndAction) {}
}

impl DataSourceHandler for State {
    fn accept_mime(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataSource, _: Option<String>) {}

    fn send_request(&mut self, _: &Connection, _: &QueueHandle<Self>, source: &WlDataSource, mime: String, mut pipe: WritePipe) {
        let contents = [&self.selection.as_ref().map(|(s, c)| (s.inner().clone(), c)), &self.drag.as_ref().map(|(s, c)| (s.inner().clone(), c))]
            .into_iter()
            .flatten()
            .find(|(s, _)| s == source)
            .and_then(|(_, c)| c.iter().find(|(t, _)| *t == mime).map(|(_, d)| d.clone()));
        if let Some(mut data) = contents {
            std::thread::spawn(move || {
                let _ = pipe.write_all(&data);
                zeroize::Zeroize::zeroize(&mut data);
            });
        }
    }

    fn cancelled(&mut self, _: &Connection, _: &QueueHandle<Self>, source: &WlDataSource) {
        if self.selection.as_ref().is_some_and(|(s, _)| s.inner() == source) {
            if self.secret.is_some() {
                self.end_secret(SecretEnd::Replaced);
            }
            self.selection = None;
        }
        if self.drag.as_ref().is_some_and(|(s, _)| s.inner() == source) {
            self.drag = None;
            self.dragging_own = false;
            self.send(DragEvent::Ended { dropped: false });
        }
    }

    fn dnd_dropped(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataSource) {}

    fn dnd_finished(&mut self, _: &Connection, _: &QueueHandle<Self>, source: &WlDataSource) {
        if self.drag.as_ref().is_some_and(|(s, _)| s.inner() == source) {
            self.drag = None;
            self.dragging_own = false;
            self.send(DragEvent::Ended { dropped: true });
        }
    }

    fn action(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataSource, _: DndAction) {}
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_seat(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: WlSeat) {
        if self.device.is_none() {
            self.device = Some(self.manager.get_data_device(qh, &seat));
        }
    }

    fn new_capability(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: WlSeat, capability: Capability) {
        if self.device.is_none() {
            self.device = Some(self.manager.get_data_device(qh, &seat));
        }
        match capability {
            Capability::Pointer if self.pointer.is_none() => {
                self.pointer = self.seats.get_pointer(qh, &seat).ok();
            }
            Capability::Keyboard if self.keyboard.is_none() => {
                self.keyboard = Some(seat.get_keyboard(qh, ()));
            }
            _ => {}
        }
    }

    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat, _: Capability) {}
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
}

impl PointerHandler for State {
    fn pointer_frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlPointer, events: &[PointerEvent]) {
        for event in events {
            match event.kind {
                PointerEventKind::Press { serial, .. } => {
                    self.serial = serial;
                    self.press = Some((serial, event.surface.clone()));
                }
                PointerEventKind::Enter { serial } | PointerEventKind::Release { serial, .. } => self.serial = serial,
                _ => {}
            }
        }
    }
}

impl Dispatch<WlKeyboard, ()> for State {
    fn event(state: &mut Self, _: &WlKeyboard, event: wl_keyboard::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match event {
            wl_keyboard::Event::Enter { serial, .. } | wl_keyboard::Event::Key { serial, .. } => state.serial = serial,
            _ => {}
        }
    }
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![SeatState];
}

impl Dispatch<IsleVisibilityV1, u32> for State {
    fn event(state: &mut Self, _: &IsleVisibilityV1, event: isle_visibility_v1::Event, surface: &u32, _: &Connection, _: &QueueHandle<Self>) {
        let isle_visibility_v1::Event::Shown { shown } = event;
        let _ = state.shown.unbounded_send((*surface, shown != 0));
    }
}

wayland_client::delegate_noop!(State: ignore IsleTitlebarManagerV1);

delegate_registry!(State);
delegate_seat!(State);
delegate_pointer!(State);
delegate_data_device!(State);

/// Text through the data device for Freya's fields.
pub struct Clipboard(pub Data);

impl freya::clipboard::ClipboardProvider for Clipboard {
    fn get_text(&mut self) -> Result<String, freya::clipboard::ClipboardError> {
        self.0
            .read_selection(&["text/plain;charset=utf-8", "UTF8_STRING", "text/plain"])
            .and_then(|(_, data)| String::from_utf8(data).ok())
            .ok_or(freya::clipboard::ClipboardError::FailedToRead)
    }

    fn set_text(&mut self, text: String) -> Result<(), freya::clipboard::ClipboardError> {
        let bytes = text.into_bytes();
        self.0.set_selection(
            ["text/plain;charset=utf-8", "UTF8_STRING", "text/plain"]
                .into_iter()
                .map(|t| (t.to_owned(), bytes.clone()))
                .collect(),
        );
        Ok(())
    }

    fn get_image(&mut self) -> Result<freya::clipboard::ClipboardImage, freya::clipboard::ClipboardError> {
        Err(freya::clipboard::ClipboardError::FailedToRead)
    }

    fn set_image(&mut self, _: freya::clipboard::ClipboardImage) -> Result<(), freya::clipboard::ClipboardError> {
        Err(freya::clipboard::ClipboardError::FailedToSet)
    }
}
