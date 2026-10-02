//! A Slint software-rendered surface for the spike's base: a run-time-loaded
//! `.slint` file drawn into a CPU buffer and uploaded to a GL texture by dirty region.

use std::{
    cell::Cell,
    collections::BTreeMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};

use notify::Watcher;
use slint::{
    LogicalPosition, PhysicalSize,
    platform::{
        Key, PointerEventButton, WindowAdapter, WindowEvent,
        software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType},
    },
};
use slint_interpreter::{ComponentHandle, Compiler, ComponentDefinition, ComponentInstance, Value};
use spike_base::{
    UiEvent,
    smithay::{
        backend::{
            allocator::Fourcc,
            renderer::{
                ImportMem,
                gles::{GlesRenderer, GlesTexture},
            },
        },
        input::keyboard::xkb,
        utils::{Buffer, Physical, Rectangle, Size},
    },
};

pub use slint_interpreter;

/// Quiet time after the last file event before reloading, so a save that
/// arrives as several writes compiles once.
const DEBOUNCE: Duration = Duration::from_millis(30);

struct Platform {
    window: Rc<MinimalSoftwareWindow>,
    start: Instant,
}

impl slint::platform::Platform for Platform {
    // Every instance, including each reload, shares the one window.
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> Duration {
        self.start.elapsed()
    }
}

/// Per-frame samples for one kind of frame, named by the caller.
#[derive(Default)]
struct Samples {
    draw_ms: Vec<f64>,
    upload_ms: Vec<f64>,
    pixels: Vec<f64>,
    rects: usize,
}

type Setup = Box<dyn Fn(&ComponentInstance)>;

pub struct Surface {
    window: Rc<MinimalSoftwareWindow>,
    path: PathBuf,
    component: &'static str,
    compiler: Compiler,
    definition: Option<ComponentDefinition>,
    instance: Option<ComponentInstance>,
    on_instance: Option<Setup>,
    events: mpsc::Receiver<notify::Result<notify::Event>>,
    _watcher: notify::RecommendedWatcher,
    changed_at: Option<Instant>,
    scale: f32,
    size: Size<i32, Physical>,
    buffer: Vec<PremultipliedRgbaColor>,
    texture: Option<GlesTexture>,
    dirty: Vec<Rectangle<i32, Buffer>>,
    draw_time: Duration,
    full_redraw: Cell<bool>,
    pointer: LogicalPosition,
    samples: BTreeMap<&'static str, Samples>,
}

fn compile(compiler: &Compiler, path: &Path, component: &str) -> Option<ComponentDefinition> {
    // The interpreter's compiler is async only for custom file loaders; the
    // default one is ready on the first poll.
    let mut future = std::pin::pin!(compiler.build_from_path(path));
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let std::task::Poll::Ready(result) = future.as_mut().poll(&mut cx) else {
        unreachable!("the default file loader never pends")
    };
    for diagnostic in result.diagnostics() {
        eprintln!("{diagnostic}");
    }
    if result.has_errors() {
        return None;
    }
    result.component(component)
}

fn key_text(keysym: xkb::Keysym, text: Option<String>) -> Option<String> {
    use xkb::Keysym as K;
    let key = match keysym {
        K::BackSpace => Key::Backspace,
        K::Delete | K::KP_Delete => Key::Delete,
        K::Return | K::KP_Enter => Key::Return,
        K::Tab => Key::Tab,
        K::ISO_Left_Tab => Key::Backtab,
        K::Escape => Key::Escape,
        K::Left | K::KP_Left => Key::LeftArrow,
        K::Right | K::KP_Right => Key::RightArrow,
        K::Up | K::KP_Up => Key::UpArrow,
        K::Down | K::KP_Down => Key::DownArrow,
        K::Home | K::KP_Home => Key::Home,
        K::End | K::KP_End => Key::End,
        K::Shift_L => Key::Shift,
        K::Shift_R => Key::ShiftR,
        K::Control_L => Key::Control,
        K::Control_R => Key::ControlR,
        K::Alt_L | K::Alt_R => Key::Alt,
        K::Super_L => Key::Meta,
        K::Super_R => Key::MetaR,
        _ => {
            // Releases carry no text from the base, and Ctrl turns letters
            // into control codes; Slint wants the plain character for both.
            let text = text.filter(|t| !t.chars().all(char::is_control));
            return text
                .or_else(|| Some(xkb::keysym_to_utf8(keysym).trim_end_matches('\0').to_string()))
                .filter(|t| !t.is_empty());
        }
    };
    Some(char::from(key).to_string())
}

fn stats(v: &[f64]) -> String {
    if v.is_empty() {
        return "-".into();
    }
    let mut v = v.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    format!("{:.2}/{:.2}/{:.2}", at(0.5), at(0.95), v[v.len() - 1])
}

impl Surface {
    /// `file` is relative to this crate's `ui/` directory; `component` is the
    /// exported component to instantiate.
    pub fn new(file: &str, component: &'static str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join(file);
        // A reused buffer: the renderer repaints only what changed into the
        // pixels it left last frame, and those rectangles are all that is uploaded.
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        slint::platform::set_platform(Box::new(Platform { window: window.clone(), start: Instant::now() }))
            .map_err(|e| format!("{e:?}"))?;

        // Editors save by writing a new file and renaming it over the old
        // one, so the directory is watched rather than the file.
        let (tx, events) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(path.parent().ok_or("no parent directory")?, notify::RecursiveMode::NonRecursive)?;

        Ok(Self {
            window,
            path,
            component,
            compiler: Compiler::default(),
            definition: None,
            instance: None,
            on_instance: None,
            events,
            _watcher: watcher,
            changed_at: None,
            scale: 1.0,
            size: (0, 0).into(),
            buffer: Vec::new(),
            texture: None,
            dirty: Vec::new(),
            draw_time: Duration::ZERO,
            full_redraw: Cell::new(false),
            pointer: LogicalPosition::default(),
            samples: BTreeMap::new(),
        })
    }

    /// Runs on every new instance, the first and each reload, after the old
    /// instance's properties were carried over: for models and callbacks.
    pub fn on_instance(&mut self, setup: impl Fn(&ComponentInstance) + 'static) {
        self.on_instance = Some(Box::new(setup));
    }

    pub fn instance(&self) -> &ComponentInstance {
        self.instance.as_ref().expect("used after init")
    }

    pub fn set(&self, name: &str, value: Value) {
        if let Err(e) = self.instance().set_property(name, value) {
            eprintln!("slint: cannot set {name}: {e}");
        }
    }

    pub fn invoke(&self, name: &str) {
        if let Err(e) = self.instance().invoke(name, &[]) {
            eprintln!("slint: cannot invoke {name}: {e}");
        }
    }

    fn dispatch(&self, event: WindowEvent) {
        self.window.window().dispatch_event(event);
    }

    pub fn init(&mut self, renderer: &mut GlesRenderer, size: Size<i32, Physical>, scale: f64) {
        self.scale = scale as f32;
        self.size = size;
        self.dispatch(WindowEvent::ScaleFactorChanged { scale_factor: self.scale });
        self.window.set_size(PhysicalSize::new(size.w as u32, size.h as u32));
        let definition = compile(&self.compiler, &self.path, self.component).expect("the .slint file compiles");
        let instance = definition.create().expect("create the component");
        if let Some(setup) = &self.on_instance {
            setup(&instance);
        }
        self.instance = Some(instance);
        self.definition = Some(definition);
        self.instance().show().expect("show the component");
        self.dispatch(WindowEvent::WindowActiveChanged(true));
        self.buffer = vec![PremultipliedRgbaColor::default(); (size.w * size.h) as usize];
        // Slint's premultiplied pixel is R, G, B, A in memory: DRM's
        // little-endian ABGR8888.
        self.texture = Some(
            renderer
                .import_memory(bytemuck::cast_slice(&self.buffer), Fourcc::Abgr8888, (size.w, size.h).into(), false)
                .expect("import the texture"),
        );
    }

    /// Recompiles the file and swaps the instance, carrying every public
    /// property over, and the field's focus when it had it.
    fn reload(&mut self) -> bool {
        let start = Instant::now();
        let Some(definition) = compile(&self.compiler, &self.path, self.component) else {
            eprintln!("slint: reload failed, keeping the running component");
            return false;
        };
        let instance = match definition.create() {
            Ok(instance) => instance,
            Err(e) => {
                eprintln!("slint: reload failed: {e}");
                return false;
            }
        };
        let (old_definition, old) = (self.definition.as_ref().unwrap(), self.instance());
        for (name, _) in old_definition.properties() {
            let Ok(value) = old.get_property(&name) else { continue };
            if definition.properties().any(|(n, _)| n == name) {
                // Output properties refuse the write, which is what they should do.
                let _ = instance.set_property(&name, value);
            }
        }
        let focused = old.get_property("search-focused") == Ok(Value::Bool(true));
        if let Some(setup) = &self.on_instance {
            setup(&instance);
        }
        self.instance = Some(instance);
        self.definition = Some(definition);
        if let Err(e) = self.instance().show() {
            eprintln!("slint: show after reload: {e}");
        }
        self.dispatch(WindowEvent::WindowActiveChanged(true));
        if focused {
            self.invoke("focus-search");
        }
        self.full_redraw.set(true);
        self.window.request_redraw();
        eprintln!("slint: reloaded {} in {:.1} ms", self.path.display(), start.elapsed().as_secs_f64() * 1000.0);
        true
    }

    pub fn event(&mut self, event: UiEvent) {
        match event {
            UiEvent::PointerMove { x, y } => {
                self.pointer = LogicalPosition::new(x as f32 / self.scale, y as f32 / self.scale);
                self.dispatch(WindowEvent::PointerMoved { position: self.pointer });
            }
            UiEvent::PointerButton { pressed } => {
                let (position, button) = (self.pointer, PointerEventButton::Left);
                self.dispatch(if pressed {
                    WindowEvent::PointerPressed { position, button }
                } else {
                    WindowEvent::PointerReleased { position, button }
                });
            }
            UiEvent::PointerLeave => self.dispatch(WindowEvent::PointerExited),
            UiEvent::Key { keysym, text, pressed } => {
                if let Some(text) = key_text(keysym, text) {
                    let text = text.into();
                    self.dispatch(if pressed {
                        WindowEvent::KeyPressed { text }
                    } else {
                        WindowEvent::KeyReleased { text }
                    });
                }
            }
        }
    }

    /// Advances Slint's timers and animations and draws into the CPU buffer
    /// if anything changed; true when there is something to upload.
    ///
    /// Drawing happens here rather than at upload: the window only says
    /// whether it needs a redraw by drawing, and the base's `tick` has to know.
    pub fn draw(&mut self) -> bool {
        slint::platform::update_timers_and_animations();
        let start = Instant::now();
        let (buffer, dirty, full) = (&mut self.buffer, &mut self.dirty, &self.full_redraw);
        let stride = self.size.w as usize;
        let drawn = self.window.draw_if_needed(|renderer| {
            if full.replace(false) {
                // Changing the buffer type drops the renderer's cache, so the
                // next frame repaints everything, including what a reload removed.
                renderer.set_repaint_buffer_type(RepaintBufferType::NewBuffer);
                renderer.render(buffer, stride);
                renderer.set_repaint_buffer_type(RepaintBufferType::ReusedBuffer);
                dirty.clear();
                dirty.push(Rectangle::from_size((stride as i32, (buffer.len() / stride) as i32).into()));
                return;
            }
            let region = renderer.render(buffer, stride);
            for (origin, size) in region.iter() {
                if size.width > 0 && size.height > 0 {
                    dirty.push(Rectangle::new(
                        (origin.x, origin.y).into(),
                        (size.width as i32, size.height as i32).into(),
                    ));
                }
            }
        });
        if drawn {
            self.draw_time += start.elapsed();
        }
        !self.dirty.is_empty()
    }

    /// Uploads what was drawn since the last call and returns the texture;
    /// `kind` names the frame in the statistics.
    pub fn upload(&mut self, renderer: &mut GlesRenderer, kind: &'static str) -> GlesTexture {
        let texture = self.texture.clone().expect("upload after init");
        if self.dirty.is_empty() {
            return texture;
        }
        let start = Instant::now();
        let bytes: &[u8] = bytemuck::cast_slice(&self.buffer);
        let mut pixels = 0;
        let rects = self.dirty.len();
        for rect in self.dirty.drain(..) {
            pixels += rect.size.w as i64 * rect.size.h as i64;
            if let Err(e) = renderer.update_memory(&texture, bytes, rect) {
                eprintln!("slint: upload {rect:?}: {e}");
            }
        }
        let s = self.samples.entry(kind).or_default();
        s.upload_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        s.draw_ms.push(std::mem::take(&mut self.draw_time).as_secs_f64() * 1000.0);
        s.pixels.push(pixels as f64);
        s.rects += rects;
        texture
    }

    pub fn poll_reload(&mut self) -> bool {
        for event in self.events.try_iter() {
            let Ok(event) = event else { continue };
            let ours = event.paths.iter().any(|p| p.file_name() == self.path.file_name());
            let write = matches!(event.kind, notify::EventKind::Create(_) | notify::EventKind::Modify(_));
            if ours && write {
                self.changed_at = Some(Instant::now());
            }
        }
        match self.changed_at {
            Some(at) if at.elapsed() >= DEBOUNCE => {
                self.changed_at = None;
                self.reload()
            }
            _ => false,
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        if !std::env::args().any(|a| a == "--bench") {
            return;
        }
        // Draw is Slint rasterising into the CPU buffer; upload is the
        // glTexSubImage2D calls, CPU time without a finish.
        println!(
            "slint uploads ({} px texture)   frames  rects  draw ms p50/p95/max   upload ms p50/p95/max   kpx p50/p95/max",
            self.size.w * self.size.h
        );
        for (kind, s) in &self.samples {
            let kpx: Vec<f64> = s.pixels.iter().map(|p| p / 1000.0).collect();
            println!(
                "  {kind:<26} {:>6} {:>6}  {:>20}  {:>20}  {:>22}",
                s.pixels.len(),
                s.rects,
                stats(&s.draw_ms),
                stats(&s.upload_ms),
                stats(&kpx)
            );
        }
    }
}
