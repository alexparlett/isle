//! A drag's icon: where it hangs from the pointer, and its return to where the drag began
//! when a drop is cancelled or refused (`conventions.md` § Drag and drop).

use std::time::{Duration, Instant};

use isle_shell::transition::Motion;
use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{element::memory::MemoryRenderBuffer, utils::with_renderer_surface_state},
    },
    reexports::wayland_server::protocol::{wl_shm, wl_surface::WlSurface},
    utils::{Logical, Point, Transform},
    wayland::{
        compositor::{SurfaceAttributes, with_states},
        shm::with_buffer_contents,
    },
};

use crate::state::{Isle, State};

/// The icon's last image going back to the drag's start.
pub struct Return {
    pub buffer: MemoryRenderBuffer,
    /// The image's top-left from the pointer.
    pub offset: Point<i32, Logical>,
    from: Point<f64, Logical>,
    to: Point<f64, Logical>,
    /// 0 at the drop, 1 at the start; under Reduce Motion the image stays and fades.
    progress: Motion,
    fade: bool,
    last_step: Instant,
}

impl Return {
    /// The pointer's place for the image now, and its opacity.
    pub fn at(&self) -> (Point<f64, Logical>, f32) {
        let p = self.progress.value();
        if self.fade {
            (self.from, (1.0 - p).clamp(0.0, 1.0) as f32)
        } else {
            (self.from + (self.to - self.from).upscale(p), 1.0)
        }
    }
}

/// The offset a surface's last commit carried, taken so it is counted once.
pub fn take_offset(surface: &WlSurface) -> Point<i32, Logical> {
    with_states(surface, |states| {
        states
            .cached_state
            .get::<SurfaceAttributes>()
            .current()
            .buffer_delta
            .take()
            .unwrap_or_default()
    })
}

/// The pixels of a surface's shared-memory buffer, with its scale.
fn icon_pixels(surface: &WlSurface) -> Option<MemoryRenderBuffer> {
    let (buffer, scale) =
        with_renderer_surface_state(surface, |s| (s.buffer().cloned(), s.buffer_scale()))?;
    let buffer = buffer?;
    with_buffer_contents(&buffer, |ptr, len, data| {
        if !matches!(
            data.format,
            wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888
        ) {
            return None;
        }
        let (width, height, stride, start) = (
            data.width as usize,
            data.height as usize,
            data.stride as usize,
            data.offset as usize,
        );
        if width == 0 || height == 0 || stride < width * 4 || start + stride * height > len {
            return None;
        }
        // The pool is the client's, mapped for the call.
        let pool = unsafe { std::slice::from_raw_parts(ptr.add(start), stride * height) };
        let mut pixels = Vec::with_capacity(width * height * 4);
        for row in pool.chunks(stride).take(height) {
            pixels.extend_from_slice(&row[..width * 4]);
        }
        let fourcc = if data.format == wl_shm::Format::Argb8888 {
            Fourcc::Argb8888
        } else {
            Fourcc::Xrgb8888
        };
        Some(MemoryRenderBuffer::from_slice(
            &pixels,
            fourcc,
            (data.width, data.height),
            scale.max(1),
            Transform::Normal,
            None,
        ))
    })
    .ok()
    .flatten()
}

impl State {
    /// The pointer during a drag: `copy` while a drop would copy, the arrow otherwise.
    pub fn set_dnd_cursor(&mut self, copies: bool) {
        use smithay::input::pointer::{CursorIcon, CursorImageStatus};
        let status = CursorImageStatus::Named(if copies {
            CursorIcon::Copy
        } else {
            CursorIcon::Default
        });
        if self.isle.cursor_status != status {
            self.isle.cursor_status = status;
            self.isle.queue_redraw_all();
        }
    }

    /// A drag moved onto another surface, which has not chosen its action yet.
    pub fn dnd_moved(&mut self, at: Point<f64, Logical>) {
        let over = self.isle.surface_under(at).map(|(focus, _)| focus);
        if over != self.isle.dnd_over {
            self.isle.dnd_over = over;
            self.set_dnd_cursor(false);
        }
    }

    /// The drag's icon taken off the pointer: dropped, it goes; cancelled or refused, its last
    /// image returns from `at` to where the drag began.
    pub fn end_dnd_icon(&mut self, at: Point<f64, Logical>, returns: bool) {
        let icon = self.isle.dnd_icon.take();
        let offset = std::mem::take(&mut self.isle.dnd_icon_offset);
        self.isle.dnd_over = None;
        self.set_dnd_cursor(false);
        self.isle.queue_redraw_all();
        let still = !self.isle.shell.animated() || self.isle.game_mode_active();
        let Some(icon) = icon.filter(|_| returns && !still) else {
            return;
        };
        let Some(buffer) = icon_pixels(&icon) else {
            return;
        };
        let theme = self.isle.shell.theme().clone();
        let fade = theme.appearance.reduce_motion;
        let mut progress = Motion::at(0.0);
        if fade {
            progress.fade_to(
                1.0,
                Duration::from_millis(theme.tokens.motion_reduced_fade_ms as u64),
            );
        } else {
            progress.spring_to(1.0, theme.spring_move());
        }
        self.isle.dnd_return = Some(Return {
            buffer,
            offset,
            from: at,
            to: self.isle.dnd_start,
            progress,
            fade,
            last_step: Instant::now(),
        });
    }
}

impl Isle {
    /// Moves a returning icon on; settled, it goes.
    pub fn step_dnd_return(&mut self, now: Instant) {
        let Some(back) = self.dnd_return.as_mut() else {
            return;
        };
        let dt = now.duration_since(back.last_step).as_secs_f64().min(0.1);
        back.last_step = now;
        back.progress.step(now, dt);
        if back.progress.settled() {
            self.dnd_return = None;
            self.queue_redraw_all();
        }
    }

    pub fn dnd_return_moving(&self) -> bool {
        self.dnd_return.is_some()
    }
}
