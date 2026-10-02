//! The spike's toy compositor: a winit window with one island whose shape the
//! compositor animates and whose content an `IslandContent` draws.

mod metrics;
mod shaders;
mod spring;

use std::time::{Duration, Instant};

use smithay::{
    backend::{
        input::{
            AbsolutePositionEvent, ButtonState, InputEvent, KeyState, KeyboardKeyEvent,
            PointerButtonEvent,
        },
        renderer::{
            Color32F, Frame, Renderer,
            gles::{GlesRenderer, GlesTexture, Uniform},
        },
        winit::{self, WinitEvent},
    },
    input::keyboard::xkb,
    reexports::{
        calloop::EventLoop,
        winit::{dpi::LogicalSize, window::WindowAttributes},
    },
    utils::{Physical, Rectangle, Size, Transform},
};

pub use metrics::{Phase, Recorder};
use spring::Spring;

pub use smithay;

/// Logical sizes of the island in its two states.
pub const REST: (f64, f64, f64) = (220.0, 38.0, 19.0);
pub const EXPANDED: (f64, f64, f64) = (520.0, 240.0, 28.0);
const TOP: f64 = 14.0;

pub enum UiEvent {
    /// Pointer position in physical pixels relative to the content texture.
    PointerMove {
        x: f64,
        y: f64,
    },
    PointerButton {
        pressed: bool,
    },
    PointerLeave,
    Key {
        keysym: xkb::Keysym,
        text: Option<String>,
        pressed: bool,
    },
}

pub trait IslandContent {
    fn name(&self) -> &'static str;
    /// Sets up GL resources for a texture of `size` physical pixels.
    fn init(&mut self, renderer: &mut GlesRenderer, size: Size<i32, Physical>, scale: f64);
    fn set_expanded(&mut self, expanded: bool);
    /// Lays the content out in `size`, top-centred in the texture. Called every
    /// frame of a morph in relayout mode only.
    fn set_layout_size(&mut self, size: Size<i32, Physical>);
    fn event(&mut self, event: UiEvent);
    /// Advances time; true when the next render would change the texture.
    fn tick(&mut self, now: Instant) -> bool;
    /// Redraws the texture if anything changed and returns it.
    fn render(&mut self, renderer: &mut GlesRenderer) -> GlesTexture;
    /// Checked between frames; true when a source changed and was reloaded.
    fn poll_reload(&mut self) -> bool {
        false
    }
    /// Launcher mode only: false when the compositor animates the surface's
    /// opacity and offset, so the content should show and hide instantly.
    fn set_animated(&mut self, _animated: bool) {}
    /// Puts the expanded content in the screenshot state: the text field
    /// holding `SHOT_TEXT` and the slider at 0.6.
    fn prepare_shot(&mut self);
}

/// Mixed scripts and an emoji, to compare shaping and font fallback.
pub const SHOT_TEXT: &str = "Résumé — 東京 ✓ 🎧";

struct Options {
    bench: bool,
    relayout: bool,
    shot: Option<String>,
    /// Launcher mode: the compositor animates the surface, not the content.
    comp_anim: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Island,
    Launcher,
}

/// Default launcher texture: the host's output.
const LAUNCHER_SIZE: (i32, i32) = (3440, 1440);
const LAUNCHER_TOGGLE: Duration = Duration::from_millis(2000);
/// Frames this soon after a toggle count as the open or close animation.
const LAUNCHER_TRANSITION: Duration = Duration::from_millis(400);

/// Where a `w`×`h` texture lands in the window, letterboxed.
fn letterbox(out: Size<i32, Physical>, tex: Size<i32, Physical>) -> Rectangle<f64, Physical> {
    let k = (out.w as f64 / tex.w as f64).min(out.h as f64 / tex.h as f64);
    let (w, h) = (tex.w as f64 * k, tex.h as f64 * k);
    Rectangle::new(
        ((out.w as f64 - w) / 2.0, (out.h as f64 - h) / 2.0).into(),
        (w, h).into(),
    )
}

fn keysym_for(c: char) -> xkb::Keysym {
    xkb::utf32_to_keysym(c as u32)
}

struct Island {
    expanded: bool,
    w: Spring,
    h: Spring,
    r: Spring,
}

impl Island {
    fn new() -> Self {
        Self {
            expanded: false,
            w: Spring::new(REST.0),
            h: Spring::new(REST.1),
            r: Spring::new(REST.2),
        }
    }

    fn toggle(&mut self) {
        self.set(!self.expanded);
    }

    fn set(&mut self, expanded: bool) {
        self.expanded = expanded;
        let (w, h, r) = if expanded { EXPANDED } else { REST };
        self.w.target = w;
        self.h.target = h;
        self.r.target = r;
    }

    fn step(&mut self, dt: f64) {
        self.w.step(dt);
        self.h.step(dt);
        self.r.step(dt);
    }

    fn settled(&self) -> bool {
        self.w.settled() && self.h.settled() && self.r.settled()
    }

    /// The shape in physical pixels for an output `out` wide.
    fn rect(&self, out: Size<i32, Physical>, scale: f64) -> Rectangle<f64, Physical> {
        let w = self.w.value * scale;
        let h = self.h.value * scale;
        Rectangle::new(
            ((out.w as f64 - w) / 2.0, TOP * scale).into(),
            (w, h).into(),
        )
    }
}

/// Runs the toy compositor with `content` as the island until the window
/// closes or the bench ends.
pub fn run(content: impl IslandContent) -> Result<(), Box<dyn std::error::Error>> {
    run_mode(content, Mode::Island)
}

/// Runs `content` as a full-output launcher surface (`--size WxH`, default
/// 3440x1440), drawn letterboxed into the window.
pub fn run_launcher(content: impl IslandContent) -> Result<(), Box<dyn std::error::Error>> {
    run_mode(content, Mode::Launcher)
}

fn run_mode(mut content: impl IslandContent, mode: Mode) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
    let args: Vec<String> = std::env::args().collect();
    let opts = Options {
        bench: args.iter().any(|a| a == "--bench"),
        relayout: args.iter().any(|a| a == "--relayout"),
        shot: args
            .iter()
            .position(|a| a == "--shot")
            .and_then(|i| args.get(i + 1).cloned()),
        comp_anim: args.iter().any(|a| a == "--comp-anim"),
    };
    let launcher_size = args
        .iter()
        .position(|a| a == "--size")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.split_once('x'))
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or(LAUNCHER_SIZE);

    let mut event_loop: EventLoop<()> = EventLoop::try_new()?;
    let (mut backend, winit_loop) = winit::init_from_attributes_with_gl_attr::<GlesRenderer>(
        WindowAttributes::default()
            .with_surface_size(LogicalSize::new(1100.0, 420.0))
            .with_title(format!(
                "{}-{}",
                if mode == Mode::Island {
                    "island"
                } else {
                    "launcher"
                },
                content.name()
            )),
        smithay::backend::egl::context::GlAttributes {
            version: (3, 0),
            profile: None,
            debug: false,
            vsync: false,
        },
    )?;

    let scale = backend.window().scale_factor();
    let tex_size: Size<i32, Physical> = match mode {
        Mode::Island => (
            (EXPANDED.0 * scale).round() as i32,
            (EXPANDED.1 * scale).round() as i32,
        )
            .into(),
        Mode::Launcher => launcher_size.into(),
    };
    if mode == Mode::Launcher {
        content.set_animated(!opts.comp_anim);
    }
    let shaders = shaders::Shaders::new(backend.renderer())?;
    let mem_init = metrics::Memory::read();
    let init_start = Instant::now();
    content.init(backend.renderer(), tex_size, scale);
    let init_time = init_start.elapsed();
    let mem_after_init = metrics::Memory::read();

    let mut events: Vec<WinitEvent> = Vec::new();
    let (tx, rx) = std::sync::mpsc::channel();
    event_loop
        .handle()
        .insert_source(winit_loop, move |event, _, _| {
            let _ = tx.send(event);
        })
        .map_err(|e| e.error)?;

    let xkb_ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap =
        xkb::Keymap::new_from_names(&xkb_ctx, "", "", "", "", None, xkb::KEYMAP_COMPILE_NO_FLAGS)
            .ok_or("no keymap")?;
    let mut xkb_state = xkb::State::new(&keymap);

    let mut island = Island::new();
    // Launcher state: open flag in `island.expanded`, plus the compositor's fade.
    let mut toggled_at = Instant::now() - Duration::from_secs(10);
    let mut fade = Spring::new(0.0);
    let mut typed = 0usize;
    let mut recorder = Recorder::new();
    let started = Instant::now();
    let mut last = Instant::now();
    let mut dirty = true;
    let mut pointer_inside = false;
    let toggle_every = if mode == Mode::Island {
        Duration::from_millis(1500)
    } else {
        LAUNCHER_TOGGLE
    };
    let mut bench_next_toggle = toggle_every;
    let bench_toggle_until = Duration::from_secs(30);
    let bench_end = Duration::from_secs(40);
    let mut closed = false;
    // Set after a swap until the host's frame callback arrives as a redraw.
    let mut awaiting_since: Option<Instant> = None;
    if opts.shot.is_some() {
        island.set(true);
        fade.target = 1.0;
        content.set_expanded(true);
        content.prepare_shot();
    }

    while !closed {
        let animating = match mode {
            Mode::Island => !island.settled() || island.expanded,
            Mode::Launcher => toggled_at.elapsed() < LAUNCHER_TRANSITION || !fade.settled(),
        };
        let timeout = if awaiting_since.is_some() {
            Duration::from_millis(1)
        } else if dirty || animating {
            Duration::ZERO
        } else {
            Duration::from_millis(50)
        };
        event_loop.dispatch(Some(timeout), &mut ())?;
        events.extend(rx.try_iter());

        let out = backend.window_size();
        for event in events.drain(..) {
            match event {
                WinitEvent::CloseRequested => closed = true,
                WinitEvent::Redraw => awaiting_since = None,
                WinitEvent::Resized { .. } | WinitEvent::Focus(_) => dirty = true,
                WinitEvent::Input(input) => match input {
                    InputEvent::PointerMotionAbsolute { event } => {
                        let logical = event
                            .position_transformed(out.to_f64().to_logical(scale).to_i32_round());
                        let p = logical.to_physical(scale);
                        let (inside, x, y) = match mode {
                            Mode::Island => {
                                let shape = island.rect(out, scale);
                                let origin_x = (out.w as f64 - tex_size.w as f64) / 2.0;
                                (shape.contains(p), p.x - origin_x, p.y - shape.loc.y)
                            }
                            Mode::Launcher => {
                                let r = letterbox(out, tex_size);
                                let k = tex_size.w as f64 / r.size.w;
                                (
                                    island.expanded && r.contains(p),
                                    (p.x - r.loc.x) * k,
                                    (p.y - r.loc.y) * k,
                                )
                            }
                        };
                        if inside {
                            content.event(UiEvent::PointerMove { x, y });
                        } else if pointer_inside {
                            content.event(UiEvent::PointerLeave);
                        }
                        pointer_inside = inside;
                    }
                    InputEvent::PointerButton { event } => {
                        let pressed = event.state() == ButtonState::Pressed;
                        if pointer_inside && island.expanded {
                            content.event(UiEvent::PointerButton { pressed });
                        } else if pressed {
                            island.toggle();
                            content.set_expanded(island.expanded);
                        }
                    }
                    InputEvent::Keyboard { event } => {
                        let code = event.key_code();
                        let pressed = event.state() == KeyState::Pressed;
                        let keysym = xkb_state.key_get_one_sym(code);
                        let text =
                            Some(xkb_state.key_get_utf8(code)).filter(|t| !t.is_empty() && pressed);
                        xkb_state.update_key(
                            code,
                            if pressed {
                                xkb::KeyDirection::Down
                            } else {
                                xkb::KeyDirection::Up
                            },
                        );
                        if keysym == xkb::Keysym::Escape && pressed {
                            island.set(false);
                            content.set_expanded(false);
                        } else if keysym == xkb::Keysym::space && pressed && !island.expanded {
                            island.toggle();
                            content.set_expanded(island.expanded);
                        } else {
                            content.event(UiEvent::Key {
                                keysym,
                                text,
                                pressed,
                            });
                        }
                    }
                    _ => {}
                },
            }
        }

        let elapsed = started.elapsed();
        if opts.bench {
            if elapsed >= bench_next_toggle && elapsed < bench_toggle_until {
                island.toggle();
                content.set_expanded(island.expanded);
                bench_next_toggle += toggle_every;
            }
            // `fade.target` catches up with a toggle later in this iteration, which also resets `toggled_at`.
            if mode == Mode::Launcher && island.expanded && fade.target == 1.0 {
                // Type "co" then delete a character while the launcher is open.
                let since = toggled_at.elapsed();
                let script: [(u64, Option<char>); 3] =
                    [(600, Some('c')), (900, Some('o')), (1200, None)];
                if let Some(&(at, ch)) = script.get(typed)
                    && since >= Duration::from_millis(at)
                {
                    let keysym = ch.map(keysym_for).unwrap_or(xkb::Keysym::BackSpace);
                    let text = ch.map(String::from);
                    content.event(UiEvent::Key {
                        keysym,
                        text,
                        pressed: true,
                    });
                    content.event(UiEvent::Key {
                        keysym,
                        text: None,
                        pressed: false,
                    });
                    typed += 1;
                }
            }
            if elapsed >= bench_toggle_until && island.expanded {
                island.set(false);
                content.set_expanded(false);
            }
            if elapsed >= bench_end {
                closed = true;
            }
        }

        if awaiting_since.is_some_and(|t| t.elapsed() < Duration::from_millis(100)) {
            continue;
        }
        awaiting_since = None;

        if content.poll_reload() || (opts.shot.is_some() && elapsed > Duration::from_secs(2)) {
            dirty = true;
        }

        let now = Instant::now();
        let dt = (now - last).as_secs_f64().min(0.05);
        last = now;
        let open_target = if island.expanded { 1.0 } else { 0.0 };
        if mode == Mode::Launcher && fade.target != open_target {
            fade.target = open_target;
            toggled_at = now;
            typed = 0;
        }
        let morphing = match mode {
            Mode::Island => {
                let was_settled = island.settled();
                island.step(dt);
                !was_settled || !island.settled()
            }
            Mode::Launcher => {
                let was_settled = fade.settled();
                if opts.comp_anim {
                    fade.step(dt);
                } else {
                    fade.value = open_target;
                }
                !was_settled || toggled_at.elapsed() < LAUNCHER_TRANSITION
            }
        };

        let content_start = Instant::now();
        if opts.relayout && morphing && mode == Mode::Island {
            let r = island.rect(out, scale);
            content.set_layout_size((r.size.w.round() as i32, r.size.h.round() as i32).into());
        }
        let content_dirty = content.tick(now);
        if !(dirty || morphing || content_dirty) {
            continue;
        }
        dirty = false;

        let texture = content.render(backend.renderer());
        if opts.bench {
            // Include the GPU's share of the content's work in its time.
            backend
                .renderer()
                .with_context(|gl| unsafe { gl.Finish() })?;
        }
        let content_time = content_start.elapsed();

        let frame_start = Instant::now();
        {
            let (renderer, mut framebuffer) = backend.bind()?;
            let full = Rectangle::from_size(out);
            let mut frame = renderer.render(&mut framebuffer, out, Transform::Flipped180)?;
            frame.clear(Color32F::new(0.08, 0.08, 0.1, 1.0), &[full])?;
            frame.render_pixel_shader_to(
                &shaders.background,
                Rectangle::from_size(
                    out.to_f64()
                        .to_logical(1.0)
                        .to_buffer(1.0, Transform::Normal),
                ),
                full,
                out.to_logical(1).to_buffer(1, Transform::Normal),
                None,
                1.0,
                &[],
            )?;
            if mode == Mode::Launcher {
                let r = letterbox(out, tex_size);
                let offset = (1.0 - fade.value) * 24.0 * r.size.w / tex_size.w as f64;
                let dest =
                    Rectangle::new((r.loc.x, r.loc.y + offset).into(), r.size).to_i32_round();
                let alpha = if opts.comp_anim {
                    fade.value.clamp(0.0, 1.0) as f32
                } else {
                    1.0
                };
                if alpha > 0.0 {
                    frame.render_texture_from_to(
                        &texture,
                        Rectangle::from_size((tex_size.w as f64, tex_size.h as f64).into()),
                        dest,
                        &[Rectangle::from_size(dest.size)],
                        &[],
                        Transform::Normal,
                        alpha,
                        None,
                        &[],
                    )?;
                }
            }
            let shape = island.rect(out, scale);
            let dest = shape.to_i32_round();
            let radius = (island.r.value * scale) as f32;
            if mode == Mode::Island {
                frame.render_pixel_shader_to(
                    &shaders.fill,
                    Rectangle::from_size(
                        dest.size
                            .to_f64()
                            .to_logical(1.0)
                            .to_buffer(1.0, Transform::Normal),
                    ),
                    dest,
                    dest.size.to_logical(1).to_buffer(1, Transform::Normal),
                    None,
                    1.0,
                    &[Uniform::new("radius", radius)],
                )?;
                // The texture is top-centred on the shape; crop it to the shape.
                let origin_x = (tex_size.w as f64 - shape.size.w) / 2.0;
                let crop = Rectangle::<f64, smithay::utils::Buffer>::new(
                    (origin_x.max(0.0), 0.0).into(),
                    (
                        shape.size.w.min(tex_size.w as f64),
                        shape.size.h.min(tex_size.h as f64),
                    )
                        .into(),
                );
                let crop_origin = (
                    crop.loc.x / tex_size.w as f64,
                    crop.loc.y / tex_size.h as f64,
                );
                let crop_extent = (
                    crop.size.w / tex_size.w as f64,
                    crop.size.h / tex_size.h as f64,
                );
                frame.render_texture_from_to(
                    &texture,
                    crop,
                    dest,
                    &[Rectangle::from_size(dest.size)],
                    &[],
                    Transform::Normal,
                    1.0,
                    Some(&shaders.clip),
                    &[
                        Uniform::new("shape_size", (dest.size.w as f32, dest.size.h as f32)),
                        Uniform::new("radius", radius),
                        Uniform::new("crop_origin", (crop_origin.0 as f32, crop_origin.1 as f32)),
                        Uniform::new("crop_extent", (crop_extent.0 as f32, crop_extent.1 as f32)),
                    ],
                )?;
            }
            let shot = match &opts.shot {
                Some(path) if !morphing && elapsed > Duration::from_secs(2) => {
                    let pixels = frame.with_context(|gl| {
                        let mut buf = vec![0u8; (out.w * out.h * 4) as usize];
                        unsafe {
                            gl.Finish();
                            gl.ReadPixels(
                                0,
                                0,
                                out.w,
                                out.h,
                                smithay::backend::renderer::gles::ffi::RGBA,
                                smithay::backend::renderer::gles::ffi::UNSIGNED_BYTE,
                                buf.as_mut_ptr().cast(),
                            )
                        };
                        buf
                    })?;
                    Some((path.clone(), pixels))
                }
                _ => None,
            };
            let _ = frame.finish()?;
            if let Some((path, pixels)) = shot {
                metrics::write_png(&path, out.w as u32, out.h as u32, &pixels)?;
                println!("wrote {path}");
                break;
            }
        }
        let frame_cpu = frame_start.elapsed();
        let swap_start = Instant::now();
        backend.submit(None)?;
        backend.window().request_redraw();
        awaiting_since = Some(Instant::now());
        let swap = swap_start.elapsed();

        let phase = if opts.bench && elapsed >= bench_toggle_until && !morphing {
            Phase::Idle
        } else if morphing {
            Phase::Morph
        } else if island.expanded {
            Phase::Expanded
        } else {
            Phase::Rest
        };
        recorder.record(phase, content_time, content_time + frame_cpu, swap);
    }

    if opts.bench {
        let mem_end = metrics::Memory::read();
        let variant = match mode {
            Mode::Island if opts.relayout => "island, relayout",
            Mode::Island => "island, cross-fade",
            Mode::Launcher if opts.comp_anim => "launcher, compositor animates",
            Mode::Launcher => "launcher, content animates",
        };
        println!("== {} ({variant}) ==", content.name());
        println!("scale {scale}, texture {}x{}", tex_size.w, tex_size.h);
        println!("content init {:.1} ms", init_time.as_secs_f64() * 1000.0);
        recorder.print();
        println!("memory before content init: {mem_init}");
        println!("memory after content init:  {mem_after_init}");
        println!("memory at end:              {mem_end}");
        println!("gpu: {}", metrics::gpu_memory());
    }
    Ok(())
}
