//! Windows moving: open, close, hide, minimise and restore, the frame moving to a tile, and
//! the desktop switch (`p1/surfaces/window-chrome.md` Motion; `mission-control.md`, the
//! switch). Every move starts and ends at a real box; the compositor draws it.

use std::time::{Duration, Instant};

use isle_shell::transition::Motion;
use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            Bind, Color32F, Frame, Offscreen, Renderer,
            element::texture::TextureBuffer,
            gles::{GlesRenderer, GlesTexture},
        },
    },
    output::Output,
    utils::{Logical, Rectangle, Size, Transform},
};
use tracing::warn;

use super::geometry::Rect;
use crate::state::{Isle, State};

/// How long a frame waits for its client's new size before it shows what it has.
const MORPH_WAIT: Duration = Duration::from_millis(250);

/// How windows move now: the theme's springs, a fade under Reduce Motion, or not at all.
#[derive(Clone, Copy)]
pub struct Style {
    animated: bool,
    fade: Option<Duration>,
    open: isle_tokens::Spring,
    exit: isle_tokens::Spring,
    moving: isle_tokens::Spring,
    switch: isle_tokens::Spring,
    /// How far windows travel in a desktop switch, in points.
    switch_distance: f64,
    show: Duration,
    crossfade: Duration,
    from_scale: f64,
    minimise_width: f64,
}

impl Style {
    /// Whether windows move at all now.
    pub fn animated(&self) -> bool {
        self.animated
    }

    pub fn of(isle: &Isle) -> Self {
        let theme = isle.shell.theme();
        Self {
            // Game Mode holds windows still (`compositor.md` Gaming display path 8).
            animated: isle.shell.animated() && isle.window_motion && !isle.game_mode_active(),
            fade: theme
                .appearance
                .reduce_motion
                .then(|| isle_tokens::Theme::millis(theme.tokens.motion_reduced_fade_ms)),
            open: theme.spring_open(),
            exit: theme.spring_exit(),
            moving: theme.spring_move(),
            switch: theme.spring_switch(),
            switch_distance: theme.tokens.motion_switch_distance as f64,
            show: isle_tokens::Theme::millis(theme.tokens.motion_fade_ms),
            crossfade: isle_tokens::Theme::millis(theme.tokens.motion_fade_ms),
            from_scale: theme.tokens.motion_window_scale as f64,
            minimise_width: theme.tokens.size_minimise_target as f64,
        }
    }
}

/// A frame drawn somewhere other than its real box: scaled about its centre, moved and faded.
#[derive(Clone, Copy, Debug)]
pub struct WindowMotion {
    pub opacity: Motion,
    pub scale: Motion,
    pub dx: Motion,
    pub dy: Motion,
    last: Instant,
}

impl WindowMotion {
    fn at(opacity: f64, scale: f64, dx: f64, dy: f64) -> Self {
        Self {
            opacity: Motion::at(opacity),
            scale: Motion::at(scale),
            dx: Motion::at(dx),
            dy: Motion::at(dy),
            last: Instant::now(),
        }
    }

    fn rest() -> Self {
        Self::at(1.0, 1.0, 0.0, 0.0)
    }

    /// Aims every value at its target on `spring`, or fades the opacity under Reduce Motion
    /// with everything else in place.
    fn aim(&mut self, style: &Style, spring: isle_tokens::Spring, to: (f64, f64, f64, f64)) {
        self.last = Instant::now();
        match style.fade {
            Some(length) => {
                self.scale = Motion::at(1.0);
                self.dx = Motion::at(0.0);
                self.dy = Motion::at(0.0);
                self.opacity.fade_to(to.0, length);
            }
            None => {
                self.opacity.spring_to(to.0, spring);
                self.scale.spring_to(to.1, spring);
                self.dx.spring_to(to.2, spring);
                self.dy.spring_to(to.3, spring);
            }
        }
    }

    /// Advances to `now`; true while it still moves.
    pub fn step(&mut self, now: Instant) -> bool {
        let dt = now.duration_since(self.last).as_secs_f64().min(0.1);
        self.last = now;
        for m in [
            &mut self.opacity,
            &mut self.scale,
            &mut self.dx,
            &mut self.dy,
        ] {
            m.step(now, dt);
        }
        self.moving()
    }

    pub fn moving(&self) -> bool {
        ![self.opacity, self.scale, self.dx, self.dy]
            .iter()
            .all(Motion::settled)
    }

    /// Where `frame` is drawn now.
    pub fn apply(&self, frame: Rect) -> Rectangle<f64, Logical> {
        let s = self.scale.value();
        let (w, h) = (frame.size.w as f64 * s, frame.size.h as f64 * s);
        let cx = frame.loc.x as f64 + frame.size.w as f64 / 2.0 + self.dx.value();
        let cy = frame.loc.y as f64 + frame.size.h as f64 / 2.0 + self.dy.value();
        Rectangle::new((cx - w / 2.0, cy - h / 2.0).into(), (w, h).into())
    }

    pub fn opacity(&self) -> f32 {
        self.opacity.value().clamp(0.0, 1.0) as f32
    }
}

/// A window's last frame, as a texture at its output's scale.
pub struct Snapshot {
    pub buffer: TextureBuffer<GlesTexture>,
    /// The frame it shows, in the global space.
    pub frame: Rect,
}

/// A window drawn from its last frame while it goes: closed, hidden, minimised, or on a
/// desktop being left.
pub struct Leaving {
    pub output: Output,
    pub snapshot: Snapshot,
    pub motion: WindowMotion,
}

/// A frame moving to a new box: the old content held scaled to the frame until the client
/// draws at the new size, then crossfaded to it.
pub struct Morph {
    pub snapshot: Snapshot,
    pub from: Rect,
    pub progress: Motion,
    pub crossfade: Motion,
    pub size: Size<i32, Logical>,
    started: Instant,
    last: Instant,
}

impl Morph {
    /// The frame drawn now, between where it was and `to`.
    pub fn frame(&self, to: Rect) -> Rectangle<f64, Logical> {
        let t = self.progress.value();
        let lerp = |a: i32, b: i32| a as f64 + (b - a) as f64 * t;
        Rectangle::new(
            (
                lerp(self.from.loc.x, to.loc.x),
                lerp(self.from.loc.y, to.loc.y),
            )
                .into(),
            (
                lerp(self.from.size.w, to.size.w),
                lerp(self.from.size.h, to.size.h),
            )
                .into(),
        )
    }

    pub fn moving(&self) -> bool {
        !self.progress.settled() || !self.crossfade.settled() || self.crossfade.target() < 1.0
    }
}

impl Isle {
    /// Whether a window on `output` is still moving, so it needs another frame.
    pub fn windows_moving(&self, output: &Output) -> bool {
        self.leaving.iter().any(|l| &l.output == output)
            || self.wm.records.iter().any(|r| {
                (r.motion.is_some_and(|m| m.moving()) || r.morph.is_some())
                    && self.output_of(r.id).as_ref() == Some(output)
            })
    }

    /// Moves every window motion on to `now`, dropping those that ended.
    pub fn step_windows(&mut self, now: Instant) {
        self.leaving.retain_mut(|l| l.motion.step(now));
        let style = Style::of(self);
        for record in &mut self.wm.records {
            if let Some(motion) = &mut record.motion
                && !motion.step(now)
            {
                record.motion = None;
            }
            let size = record.window.geometry().size;
            if let Some(morph) = &mut record.morph {
                let dt = now.duration_since(morph.last).as_secs_f64().min(0.1);
                morph.last = now;
                morph.progress.step(now, dt);
                if morph.crossfade.target() < 1.0
                    && (size == morph.size || now.duration_since(morph.started) > MORPH_WAIT)
                {
                    morph.crossfade.fade_to(1.0, style.crossfade);
                }
                morph.crossfade.step(now, dt);
                if !morph.moving() {
                    record.morph = None;
                }
            }
        }
    }
}

impl State {
    /// Draws window `id` as it is now into a texture, for the motions that outlive its
    /// buffers or its place.
    pub fn snapshot(&mut self, id: u64) -> Option<Snapshot> {
        let frame = self.isle.frame(id)?;
        let output = self.isle.output_of(id)?;
        let scale = output.current_scale().fractional_scale();
        let renderer = self.backend.renderer_opt()?;
        let mut elements = Vec::new();
        self.isle
            .window_pieces(renderer, id, frame.loc, scale, false, 1.0, &mut |e| {
                elements.push(e)
            });
        if elements.is_empty() {
            return None;
        }
        let size = frame.size.to_f64().to_physical(scale).to_i32_round();
        match render_to_texture(renderer, size, scale, &elements) {
            Ok(texture) => {
                // Drawn at an explicit size, so the buffer's own scale does not matter.
                let buffer =
                    TextureBuffer::from_texture(renderer, texture, 1, Transform::Normal, None);
                Some(Snapshot { buffer, frame })
            }
            Err(err) => {
                warn!(id, "no snapshot: {err}");
                None
            }
        }
    }

    /// A window going away, hidden or minimised: its last frame keeps moving on its output.
    /// `toward` is minimise's target, else it closes in place.
    pub fn animate_leaving(&mut self, id: u64, toward: Option<Rect>) {
        let style = Style::of(&self.isle);
        if !style.animated {
            return;
        }
        let Some(output) = self.isle.output_of(id) else {
            return;
        };
        if !self.isle.wm.is_shown(id) {
            return;
        }
        let Some(snapshot) = self.snapshot(id) else {
            return;
        };
        let frame = snapshot.frame;
        let mut motion = WindowMotion::rest();
        let to = match toward {
            Some(target) => {
                let s = (style.minimise_width / frame.size.w.max(1) as f64).min(1.0);
                let dx = (target.loc.x + target.size.w / 2) as f64
                    - (frame.loc.x as f64 + frame.size.w as f64 / 2.0);
                let dy = (target.loc.y + target.size.h / 2) as f64
                    - (frame.loc.y as f64 + frame.size.h as f64 / 2.0);
                (0.0, s, dx, dy)
            }
            None => (0.0, style.from_scale, 0.0, 0.0),
        };
        motion.aim(&style, style.exit, to);
        self.isle.leaving.push(Leaving {
            output: output.clone(),
            snapshot,
            motion,
        });
        self.isle.queue_redraw(&output);
    }

    /// A window opening, or coming back: from slightly smaller in place, or from `from`.
    pub fn animate_arriving(&mut self, id: u64, from: Option<Rect>) {
        let style = Style::of(&self.isle);
        if !style.animated {
            return;
        }
        let Some(frame) = self.isle.frame(id) else {
            return;
        };
        let motion = match from {
            Some(from) => {
                let s = from.size.w as f64 / frame.size.w.max(1) as f64;
                let dx = (from.loc.x + from.size.w / 2) as f64
                    - (frame.loc.x as f64 + frame.size.w as f64 / 2.0);
                let dy = (from.loc.y + from.size.h / 2) as f64
                    - (frame.loc.y as f64 + frame.size.h as f64 / 2.0);
                WindowMotion::at(0.0, s, dx, dy)
            }
            None => WindowMotion::at(0.0, style.from_scale, 0.0, 0.0),
        };
        self.start_motion(id, motion, style.open, &style);
    }

    /// Shown again after a hide: a fade in its box.
    pub fn animate_showing(&mut self, id: u64) {
        let style = Style::of(&self.isle);
        if !style.animated {
            return;
        }
        let Some(record) = self.isle.wm.get_mut(id) else {
            return;
        };
        let mut motion = WindowMotion::at(0.0, 1.0, 0.0, 0.0);
        motion
            .opacity
            .fade_to(1.0, style.fade.unwrap_or(style.show));
        record.motion = Some(motion);
        let window = record.window.clone();
        self.isle.queue_redraw_for_window(&window);
    }

    fn start_motion(
        &mut self,
        id: u64,
        mut motion: WindowMotion,
        spring: isle_tokens::Spring,
        style: &Style,
    ) {
        motion.aim(style, spring, (1.0, 1.0, 0.0, 0.0));
        let Some(record) = self.isle.wm.get_mut(id) else {
            return;
        };
        record.motion = Some(motion);
        let window = record.window.clone();
        self.isle.queue_redraw_for_window(&window);
    }

    /// Before a frame moves to a new box: holds its content to move with it.
    pub fn begin_morph(&mut self, id: u64, to: Rect) {
        let style = Style::of(&self.isle);
        if !style.animated || !self.isle.wm.is_shown(id) {
            return;
        }
        let Some(from) = self.isle.frame(id) else {
            return;
        };
        if from == to {
            return;
        }
        let Some(snapshot) = self.snapshot(id) else {
            return;
        };
        let bar = self.isle.bar_height(id);
        let mut progress = Motion::at(0.0);
        let crossfade = Motion::at(0.0);
        match style.fade {
            // Reduce Motion: cut to the new box, the crossfade still running.
            Some(_) => progress = Motion::at(1.0),
            None => progress.spring_to(1.0, style.moving),
        }
        let now = Instant::now();
        let Some(record) = self.isle.wm.get_mut(id) else {
            return;
        };
        record.morph = Some(Morph {
            snapshot,
            from,
            progress,
            crossfade,
            size: (to.size.w, to.size.h - bar).into(),
            started: now,
            last: now,
        });
        let window = record.window.clone();
        self.isle.queue_redraw_for_window(&window);
    }

    /// Before the view on `output` moves along its row by `step`: the windows shown now slide
    /// out and fade, drawn from their last frames. `except` is a window going full screen,
    /// which grows in place instead.
    pub fn slide_out(&mut self, output: &Output, step: i32, except: Option<u64>) {
        let style = Style::of(&self.isle);
        if !style.animated {
            return;
        }
        let ids: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| {
                Some(r.id) != except
                    && self.isle.wm.is_shown(r.id)
                    && self.isle.output_of(r.id).as_ref() == Some(output)
            })
            .map(|r| r.id)
            .collect();
        for id in ids {
            let Some(snapshot) = self.snapshot(id) else {
                continue;
            };
            let mut motion = WindowMotion::rest();
            motion.aim(
                &style,
                style.switch,
                (0.0, 1.0, -step as f64 * style.switch_distance, 0.0),
            );
            self.isle.leaving.push(Leaving {
                output: output.clone(),
                snapshot,
                motion,
            });
        }
    }

    /// After the view moved: the windows now shown slide in from the other side, but for
    /// `except`, a window leaving full screen, which shrinks in place instead.
    pub fn slide_in(&mut self, output: &Output, step: i32, except: Option<u64>) {
        let style = Style::of(&self.isle);
        if !style.animated {
            return;
        }
        let ids: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| {
                Some(r.id) != except
                    && self.isle.wm.is_shown(r.id)
                    && self.isle.output_of(r.id).as_ref() == Some(output)
            })
            .map(|r| r.id)
            .collect();
        for id in ids {
            let motion = WindowMotion::at(0.0, 1.0, step as f64 * style.switch_distance, 0.0);
            self.start_motion(id, motion, style.switch, &style);
        }
    }
}

/// Draws `elements`, placed relative to the texture's origin, into a new texture.
pub fn render_to_texture<E>(
    renderer: &mut GlesRenderer,
    size: Size<i32, smithay::utils::Physical>,
    scale: f64,
    elements: &[E],
) -> Result<GlesTexture, String>
where
    E: smithay::backend::renderer::element::RenderElement<GlesRenderer>,
{
    let buffer_size = size.to_logical(1).to_buffer(1, Transform::Normal);
    let mut texture: GlesTexture =
        Offscreen::<GlesTexture>::create_buffer(renderer, Fourcc::Abgr8888, buffer_size)
            .map_err(|e| e.to_string())?;
    {
        let mut target = renderer.bind(&mut texture).map_err(|e| e.to_string())?;
        let mut frame = renderer
            .render(&mut target, size, Transform::Normal)
            .map_err(|e| e.to_string())?;
        let all = Rectangle::from_size(size);
        frame
            .clear(Color32F::TRANSPARENT, &[all])
            .map_err(|e| e.to_string())?;
        smithay::backend::renderer::utils::draw_render_elements(
            &mut frame,
            scale,
            elements,
            &[all],
        )
        .map_err(|e| e.to_string())?;
        let _ = frame.finish().map_err(|e| e.to_string())?;
    }
    Ok(texture)
}
