//! The standard capture protocols: ext-foreign-toplevel-list naming the windows,
//! ext-image-capture-source turning an output or a window into a source, and
//! ext-image-copy-capture copying a source into a client's buffer (`p11-capture.md` choice
//! 29). Frames are the same renders as a shot's, so a locked screen gives the lock.

use std::{cell::Cell, collections::HashMap, time::Duration};

use smithay::{
    output::{Output, WeakOutput},
    reexports::wayland_server::protocol::wl_shm,
    utils::{Buffer as BufferCoords, Size, Transform},
    wayland::{
        foreign_toplevel_list::{
            ForeignToplevelHandle, ForeignToplevelListHandler, ForeignToplevelListState,
        },
        image_capture_source::{
            ImageCaptureSource, ImageCaptureSourceHandler, OutputCaptureSourceHandler,
            OutputCaptureSourceState, ToplevelCaptureSourceHandler, ToplevelCaptureSourceState,
        },
        image_copy_capture::{
            BufferConstraints, CaptureFailureReason, CursorSession, CursorSessionRef, Frame,
            ImageCopyCaptureHandler, ImageCopyCaptureState, Session, SessionRef,
        },
        shm::with_buffer_contents_mut,
    },
};
use tracing::warn;

use crate::{
    capture::Target,
    state::{Isle, State},
};

/// What a capture source stands for.
#[derive(Clone, Debug)]
enum Captured {
    Output(WeakOutput),
    Window(u64),
}

/// The window a foreign-toplevel handle names.
struct HandleWindow(u64);

/// How many frames a session has had; past one it is a stream, and shows.
#[derive(Default)]
struct Frames(Cell<u32>);

/// The cursor image's serial a cursor session's last frame held.
#[derive(Default)]
struct CursorSent(Cell<u64>);

/// A session's number in the Sharing row and the app of the client that opened it.
struct Copier {
    id: u64,
    app: String,
}

pub struct ImageCopy {
    pub copy: ImageCopyCaptureState,
    pub outputs: OutputCaptureSourceState,
    pub toplevels: ToplevelCaptureSourceState,
    pub list: ForeignToplevelListState,
    /// The foreign handle of each window, by id.
    handles: HashMap<u64, ForeignToplevelHandle>,
    /// Open sessions, kept so they live.
    sessions: Vec<Session>,
    /// Frames waiting for their source's next redraw.
    pending: Vec<(SessionRef, Frame)>,
    /// Pointer cursor sessions, kept so they live.
    cursors: Vec<CursorSession>,
    /// Cursor frames waiting for the pointer's image to change.
    cursor_pending: Vec<(CursorSessionRef, Frame)>,
    pub dirty: Cell<bool>,
    next_id: u64,
    /// The streaming sessions the Sharing row lists.
    listed: Vec<u64>,
}

impl ImageCopy {
    pub fn new(dh: &smithay::reexports::wayland_server::DisplayHandle) -> Self {
        Self {
            copy: ImageCopyCaptureState::new::<State>(dh),
            outputs: OutputCaptureSourceState::new::<State>(dh),
            toplevels: ToplevelCaptureSourceState::new::<State>(dh),
            list: ForeignToplevelListState::new::<State>(dh),
            handles: HashMap::new(),
            sessions: Vec::new(),
            pending: Vec::new(),
            cursors: Vec::new(),
            cursor_pending: Vec::new(),
            dirty: Cell::new(true),
            next_id: 0,
            listed: Vec::new(),
        }
    }

    /// The sessions past their first frame: streams, with the client of each.
    pub fn streaming(&self) -> impl Iterator<Item = &Session> {
        self.sessions
            .iter()
            .filter(|s| s.user_data().get::<Frames>().is_some_and(|f| f.0.get() > 1))
    }
}

/// `pixels` copied into the frame's shared-memory buffer as BGRA rows; false when its size or
/// pool does not fit them.
fn write_shm(frame: &Frame, pixels: &crate::backend::Pixels) -> bool {
    let buffer = frame.buffer();
    let written = with_buffer_contents_mut(&buffer, |ptr, len, data| {
        if data.width as u32 != pixels.width || data.height as u32 != pixels.height {
            return false;
        }
        let alpha = data.format == wl_shm::Format::Argb8888;
        let stride = data.stride as usize;
        let row = pixels.width as usize * 4;
        for y in 0..pixels.height as usize {
            let to = data.offset as usize + y * stride;
            if to + row > len {
                return false;
            }
            // SAFETY: the range was checked against the pool's length.
            let dst = unsafe { std::slice::from_raw_parts_mut(ptr.add(to), row) };
            let src = &pixels.rgba[y * row..(y + 1) * row];
            for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
                d[3] = if alpha { s[3] } else { 255 };
            }
        }
        true
    });
    matches!(written, Ok(true))
}

fn captured(source: &ImageCaptureSource) -> Option<Captured> {
    source.user_data().get::<Captured>().cloned()
}

/// The monotonic clock now, as presentation times are counted.
fn now() -> Duration {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: clock_gettime writes into the timespec given.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
}

impl Isle {
    /// Each streaming capture session's number, app and what it takes.
    pub fn copy_rows(&self) -> Vec<(u64, String, String)> {
        self.image_copy
            .streaming()
            .filter_map(|s| {
                let copier = s.user_data().get::<Copier>()?;
                let what = match captured(&s.source())? {
                    Captured::Output(_) => "Entire screen".to_owned(),
                    Captured::Window(id) => self
                        .window_infos()
                        .into_iter()
                        .find(|i| i.id == id)
                        .map(|i| i.title)
                        .unwrap_or_else(|| "A window".into()),
                };
                Some((copier.id, copier.app.clone(), what))
            })
            .collect()
    }

    /// The size a source's frames are, in pixels.
    fn source_pixels(&self, captured: &Captured) -> Option<Size<i32, BufferCoords>> {
        match captured {
            Captured::Output(weak) => {
                let size = weak.upgrade()?.current_mode()?.size;
                Some((size.w, size.h).into())
            }
            Captured::Window(id) => {
                let frame = self.frame(*id)?;
                let scale = self.output_of(*id)?.current_scale().fractional_scale();
                let size = frame.size.to_f64().to_physical(scale).to_i32_round::<i32>();
                Some((size.w, size.h).into())
            }
        }
    }
}

impl State {
    /// The window list changed: handles follow, titles and app ids kept current.
    pub fn sync_foreign_toplevels(&mut self) {
        if !self.isle.image_copy.dirty.replace(false) {
            return;
        }
        let infos = self.isle.window_infos();
        let ids: Vec<u64> = infos.iter().map(|w| w.id).collect();
        let gone: Vec<u64> = self
            .isle
            .image_copy
            .handles
            .keys()
            .copied()
            .filter(|id| !ids.contains(id))
            .collect();
        for id in gone {
            if let Some(handle) = self.isle.image_copy.handles.remove(&id) {
                self.isle.image_copy.list.remove_toplevel(&handle);
            }
        }
        for info in infos {
            match self.isle.image_copy.handles.get(&info.id) {
                Some(handle) => {
                    if handle.title() != info.title || handle.app_id() != info.app_id {
                        handle.send_title(&info.title);
                        handle.send_app_id(&info.app_id);
                        handle.send_done();
                    }
                }
                None => {
                    let handle = self
                        .isle
                        .image_copy
                        .list
                        .new_toplevel::<State>(&info.title, &info.app_id);
                    handle
                        .user_data()
                        .insert_if_missing(|| HandleWindow(info.id));
                    self.isle.image_copy.handles.insert(info.id, handle);
                }
            }
        }
    }

    /// After `output`'s frame: the frames waiting on a source it shows are copied.
    pub fn copy_frames(&mut self, output: &Output) {
        self.follow_cursor_sessions();
        if self.isle.image_copy.pending.is_empty() {
            return;
        }
        let (now_due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.isle.image_copy.pending)
            .into_iter()
            .partition(|(session, _)| match captured(&session.source()) {
                Some(Captured::Output(weak)) => weak.upgrade().as_ref() == Some(output),
                Some(Captured::Window(id)) => self.isle.output_of(id).as_ref() == Some(output),
                None => true,
            });
        self.isle.image_copy.pending = later;
        for (session, frame) in now_due {
            self.copy_frame(&session, frame);
        }
    }

    /// Renders the session's source into the frame's buffer.
    fn copy_frame(&mut self, session: &SessionRef, frame: Frame) {
        let Some(captured) = captured(&session.source()) else {
            frame.fail(CaptureFailureReason::Stopped);
            return;
        };
        let target = match &captured {
            Captured::Output(weak) => match weak.upgrade() {
                Some(output) => Target::Screen(output),
                None => {
                    frame.fail(CaptureFailureReason::Stopped);
                    return;
                }
            },
            // A window is given only while it is shown, never behind the lock.
            Captured::Window(id) => {
                if self.isle.lock.locked || !self.isle.wm.is_shown(*id) {
                    frame.fail(CaptureFailureReason::Unknown);
                    return;
                }
                Target::Window {
                    id: *id,
                    shadow: false,
                }
            }
        };
        let pixels = match self.capture_pixels(&target, session.draw_cursor()) {
            Ok(pixels) => pixels,
            Err(err) => {
                warn!("a capture frame: {err}");
                frame.fail(CaptureFailureReason::Unknown);
                return;
            }
        };
        let written = write_shm(&frame, &pixels);
        match written {
            true => {
                let frames = session.user_data().get_or_insert(Frames::default);
                frames.0.set(frames.0.get().saturating_add(1));
                frame.success(Transform::Normal, None, now());
                self.copying_changed();
            }
            _ => frame.fail(CaptureFailureReason::BufferConstraints),
        }
    }

    /// The output a source is on, its area in the global space and the scale its frames are
    /// at.
    fn source_area(&self, captured: &Captured) -> Option<(smithay::utils::Rectangle<i32, smithay::utils::Logical>, f64)> {
        match captured {
            Captured::Output(weak) => {
                let output = weak.upgrade()?;
                Some((
                    self.isle.space.output_geometry(&output)?,
                    output.current_scale().fractional_scale(),
                ))
            }
            Captured::Window(id) => Some((
                self.isle.frame(*id)?,
                self.isle.output_of(*id)?.current_scale().fractional_scale(),
            )),
        }
    }

    /// Each cursor session follows the pointer over its source, and a waiting cursor frame
    /// is answered once the pointer's image has changed.
    fn follow_cursor_sessions(&mut self) {
        if self.isle.image_copy.cursors.is_empty() {
            return;
        }
        self.isle
            .image_copy
            .cursors
            .retain(|s| smithay::utils::IsAlive::alive(&s.as_ref()));
        let pointer = self.isle.seat.get_pointer().unwrap().current_location();
        let sessions: Vec<CursorSessionRef> =
            self.isle.image_copy.cursors.iter().map(|s| s.as_ref()).collect();
        for session in sessions {
            let area = captured(&session.source()).and_then(|c| self.source_area(&c));
            let at = area.filter(|(a, _)| a.to_f64().contains(pointer)).map(|(a, scale)| {
                let p = (pointer - a.loc.to_f64()).to_physical(scale).to_i32_round::<i32>();
                smithay::utils::Point::<i32, BufferCoords>::from((p.x, p.y))
            });
            let scale = area.map_or(1, |(_, s)| s.ceil().max(1.0) as u32);
            if at.is_some() && self.cursor_bitmap(scale).is_some()
                && let Some((_, hotspot)) = self.isle.casting.cursor_image()
            {
                session.set_cursor_hotspot((hotspot.x, hotspot.y));
            }
            session.set_cursor_pos(at);
        }
        for (session, frame) in std::mem::take(&mut self.isle.image_copy.cursor_pending) {
            let scale = captured(&session.source())
                .and_then(|c| self.source_area(&c))
                .map_or(1, |(_, s)| s.ceil().max(1.0) as u32);
            let serial = self.cursor_bitmap(scale).unwrap_or(0);
            let sent = session.user_data().get::<CursorSent>().map(|c| c.0.get());
            if sent == Some(serial) {
                self.isle.image_copy.cursor_pending.push((session, frame));
            } else {
                self.copy_cursor_frame(&session, frame);
            }
        }
    }

    /// The pointer's image into a cursor frame's buffer, clear while it is hidden.
    fn copy_cursor_frame(&mut self, session: &CursorSessionRef, frame: Frame) {
        let side = crate::cast::CURSOR_SIDE as u32;
        let scale = captured(&session.source())
            .and_then(|c| self.source_area(&c))
            .map_or(1, |(_, s)| s.ceil().max(1.0) as u32);
        let serial = self.cursor_bitmap(scale);
        let clear;
        let pixels = match self.isle.casting.cursor_image() {
            Some((Some(image), _)) if serial.is_some() => image,
            _ => {
                clear = crate::backend::Pixels {
                    width: side,
                    height: side,
                    rgba: vec![0; (side * side * 4) as usize],
                };
                &clear
            }
        };
        if write_shm(&frame, pixels) {
            session
                .user_data()
                .get_or_insert(CursorSent::default)
                .0
                .set(serial.unwrap_or(0));
            frame.success(Transform::Normal, None, now());
        } else {
            frame.fail(CaptureFailureReason::BufferConstraints);
        }
    }

    /// A capture session taking frames shows as a screen share does, with its app in the
    /// Sharing row.
    fn copying_changed(&mut self) {
        use isle_shell::island::model::Privacy;
        let streaming: Vec<u64> = self
            .isle
            .image_copy
            .streaming()
            .filter_map(|s| s.user_data().get::<Copier>().map(|c| c.id))
            .collect();
        let casting = !self.isle.casting.streams.is_empty();
        self.isle
            .island
            .model
            .set_privacy(Privacy::Screen, !streaming.is_empty() || casting);
        if streaming != self.isle.image_copy.listed {
            self.isle.image_copy.listed = streaming;
            self.sharing_activity();
        }
        self.isle.refresh_island();
    }

    /// Stop in the Sharing row: the session ends and its client is told.
    pub fn copy_stop(&mut self, id: u64) {
        let copy = &mut self.isle.image_copy;
        let Some(index) = copy
            .sessions
            .iter()
            .position(|s| s.user_data().get::<Copier>().is_some_and(|c| c.id == id))
        else {
            return;
        };
        let session = copy.sessions.remove(index);
        let gone = session.as_ref();
        copy.pending.retain(|(s, _)| *s != gone);
        session.stop();
        self.copying_changed();
    }

    /// The capture sessions as JSON, for the Test interface.
    #[cfg(feature = "test-hooks")]
    pub fn copy_status(&self) -> serde_json::Value {
        let sessions: Vec<serde_json::Value> = self
            .isle
            .image_copy
            .sessions
            .iter()
            .map(|s| {
                serde_json::json!({
                    "frames": s.user_data().get::<Frames>().map_or(0, |f| f.0.get()),
                    "window": match captured(&s.source()) {
                        Some(Captured::Window(id)) => Some(id),
                        _ => None,
                    },
                })
            })
            .collect();
        let toplevels: serde_json::Map<String, serde_json::Value> = self
            .isle
            .image_copy
            .handles
            .iter()
            .map(|(id, h)| (id.to_string(), h.identifier().into()))
            .collect();
        serde_json::json!({ "sessions": sessions, "toplevels": toplevels })
    }
}

impl ImageCaptureSourceHandler for State {}

impl OutputCaptureSourceHandler for State {
    fn output_capture_source_state(&mut self) -> &mut OutputCaptureSourceState {
        &mut self.isle.image_copy.outputs
    }

    fn output_source_created(&mut self, source: ImageCaptureSource, output: &Output) {
        source
            .user_data()
            .insert_if_missing(|| Captured::Output(output.downgrade()));
    }
}

impl ToplevelCaptureSourceHandler for State {
    fn toplevel_capture_source_state(&mut self) -> &mut ToplevelCaptureSourceState {
        &mut self.isle.image_copy.toplevels
    }

    fn toplevel_source_created(
        &mut self,
        source: ImageCaptureSource,
        toplevel: ForeignToplevelHandle,
    ) {
        if let Some(window) = toplevel.user_data().get::<HandleWindow>() {
            let id = window.0;
            source
                .user_data()
                .insert_if_missing(|| Captured::Window(id));
        }
    }
}

impl ImageCopyCaptureHandler for State {
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState {
        &mut self.isle.image_copy.copy
    }

    fn capture_constraints(&mut self, source: &ImageCaptureSource) -> Option<BufferConstraints> {
        let size = self.isle.source_pixels(&captured(source)?)?;
        Some(BufferConstraints {
            size,
            shm: vec![wl_shm::Format::Xrgb8888, wl_shm::Format::Argb8888],
            dma: None,
        })
    }

    fn cursor_capture_constraints(
        &mut self,
        source: &ImageCaptureSource,
        _pointer: &smithay::reexports::wayland_server::protocol::wl_pointer::WlPointer,
    ) -> Option<BufferConstraints> {
        captured(source)?;
        let side = crate::cast::CURSOR_SIDE;
        Some(BufferConstraints {
            size: (side, side).into(),
            shm: vec![wl_shm::Format::Argb8888],
            dma: None,
        })
    }

    fn new_cursor_session(&mut self, session: CursorSession) {
        self.isle.image_copy.cursors.push(session);
        self.follow_cursor_sessions();
    }

    /// A cursor session's first frame is copied at once; later ones wait for the pointer's
    /// image to change.
    fn cursor_frame(&mut self, session: &CursorSessionRef, frame: Frame) {
        if session.user_data().get::<CursorSent>().is_none() {
            return self.copy_cursor_frame(session, frame);
        }
        self.isle.image_copy.cursor_pending.push((session.clone(), frame));
        self.follow_cursor_sessions();
    }

    fn cursor_session_destroyed(&mut self, session: CursorSessionRef) {
        self.isle.image_copy.cursors.retain(|s| *s != session);
        self.isle.image_copy.cursor_pending.retain(|(s, _)| *s != session);
    }

    fn new_session(&mut self, session: Session) {
        let pid = session
            .client()
            .and_then(|c| c.get_credentials(&self.isle.display_handle).ok())
            .map(|c| c.pid);
        let app = pid
            .and_then(|pid| {
                self.isle
                    .processes
                    .app_of_tree(pid)
                    .map(|app| self.isle.app_name(&app))
                    .or_else(|| std::fs::read_to_string(format!("/proc/{pid}/comm")).ok())
            })
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "An app".into());
        self.isle.image_copy.next_id += 1;
        let id = self.isle.image_copy.next_id;
        session.user_data().insert_if_missing(|| Copier { id, app });
        self.isle.image_copy.sessions.push(session);
    }

    /// A session's first frame is copied at once; later ones wait for the source's next
    /// frame, so a client asking in a loop gets one per redraw.
    fn frame(&mut self, session: &SessionRef, frame: Frame) {
        let first = session
            .user_data()
            .get::<Frames>()
            .is_none_or(|f| f.0.get() == 0);
        if first {
            self.copy_frame(session, frame);
            return;
        }
        self.isle.image_copy.pending.push((session.clone(), frame));
        match captured(&session.source()) {
            Some(Captured::Output(weak)) => {
                if let Some(output) = weak.upgrade() {
                    self.isle.queue_redraw(&output);
                }
            }
            Some(Captured::Window(id)) => {
                if let Some(output) = self.isle.output_of(id) {
                    self.isle.queue_redraw(&output);
                }
            }
            None => {}
        }
    }

    fn session_destroyed(&mut self, session: SessionRef) {
        self.isle
            .image_copy
            .sessions
            .retain(|s| s.as_ref() != session);
        self.isle.image_copy.pending.retain(|(s, _)| *s != session);
        self.copying_changed();
    }
}

impl ForeignToplevelListHandler for State {
    fn foreign_toplevel_list_state(&mut self) -> &mut ForeignToplevelListState {
        &mut self.isle.image_copy.list
    }
}
