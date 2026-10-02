//! Freya driven by hand inside the compositor: Skia on Smithay's EGL context.

use std::{borrow::Cow, cell::RefCell, rc::Rc};

use freya::prelude::{CursorPoint, Size2D};
use freya_core::{
    integration::*,
    prelude::{
        Code, Color, Key, Modifiers, MouseButton, NamedKey, PreferredTheme, RenderingTicker,
        ScreenReader, TargetPlatform,
    },
};
use freya_engine::prelude::{
    ColorType, DirectContext, FontCollection, FontMgr, FramebufferInfo, Interface, PixelGeometry,
    Surface, SurfaceOrigin, SurfaceProps, SurfacePropsFlags, TypefaceFontProvider,
    backend_render_targets, direct_contexts, wrap_backend_render_target,
};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use ragnarok::{EventsExecutorRunner, EventsMeasurerRunner};
use spike_base::{
    UiEvent,
    smithay::{
        backend::{
            egl,
            renderer::gles::{GlesRenderer, GlesTexture, ffi},
        },
        input::keyboard::xkb::Keysym,
        utils::{Physical, Size},
    },
};

/// The Skia surface over an FBO whose colour attachment is the texture Smithay samples.
struct Gpu {
    context: DirectContext,
    surface: Surface,
    texture: GlesTexture,
    max_attribs: u32,
}

/// A Freya app rendered into a GL texture the compositor samples.
pub struct Embedded {
    runner: Runner,
    tree: Tree,
    nodes_state: NodesState<NodeId>,
    accessibility: AccessibilityTree,
    platform: Platform,
    ticker: RenderingTickerSender,
    events_sender: UnboundedSender<EventsChunk>,
    events_receiver: UnboundedReceiver<EventsChunk>,
    user_events: Rc<RefCell<Vec<UserEvent>>>,
    font_collection: FontCollection,
    font_manager: FontMgr,
    fonts: Vec<Cow<'static, str>>,
    scale: f64,
    size: Size2D,
    cursor: CursorPoint,
    layout_dirty: bool,
    a11y_dirty: bool,
    needs_draw: bool,
    gpu: Option<Gpu>,
}

impl Embedded {
    /// `model` runs in the root scope after the platform contexts exist.
    pub fn new<M: Clone + 'static>(
        root: impl Fn() -> Element + 'static,
        model: impl FnOnce() -> M,
    ) -> (Self, M) {
        let (events_sender, events_receiver) = unbounded();
        let mut runner = Runner::new(root);
        let tree = Tree::default();
        let user_events = Rc::new(RefCell::new(Vec::new()));

        runner.provide_root_context(GlobalContexts::default);
        runner.provide_root_context(ScreenReader::new);
        let (ticker, ticker_receiver) = RenderingTicker::new();
        runner.provide_root_context(|| ticker_receiver);
        runner.provide_root_context(AnimationClock::new);
        runner.provide_root_context(TargetPlatform::detect);
        runner.provide_root_context(|| tree.accessibility_generator.clone());
        let platform = runner.provide_root_context({
            let user_events = user_events.clone();
            move || Platform {
                focused_accessibility_id: State::create(ACCESSIBILITY_ROOT_ID),
                focused_accessibility_node: State::create(accesskit::Node::new(
                    accesskit::Role::Window,
                )),
                root_size: State::create(Size2D::default()),
                scale_factor: State::create(1.0),
                custom_scale_factor: State::create(1.0),
                navigation_mode: State::create(NavigationMode::NotKeyboard),
                preferred_theme: State::create(PreferredTheme::Dark),
                is_app_focused: State::create(true),
                accent_color: State::create(AccentColor::default()),
                sender: Rc::new(move |event| user_events.borrow_mut().push(event)),
            }
        });
        let model = runner.provide_root_context(model);

        let mut font_collection = FontCollection::new();
        font_collection.set_default_font_manager(FontMgr::default(), None);
        let font_manager: FontMgr = TypefaceFontProvider::new().into();
        font_collection.set_dynamic_font_manager(font_manager.clone());
        runner.provide_root_context(|| font_collection.clone());

        let embedded = Self {
            runner,
            tree,
            nodes_state: NodesState::default(),
            accessibility: AccessibilityTree::default(),
            platform,
            ticker,
            events_sender,
            events_receiver,
            user_events,
            font_collection,
            font_manager,
            fonts: ["Inter", "Noto Sans", "Noto Color Emoji", "Noto Sans CJK JP"]
                .map(Cow::Borrowed)
                .to_vec(),
            scale: 1.0,
            size: Size2D::default(),
            cursor: CursorPoint::default(),
            layout_dirty: true,
            a11y_dirty: true,
            needs_draw: true,
            gpu: None,
        };
        (embedded, model)
    }

    /// One pass of Freya's frame: run handlers, poll tasks, diff, lay out,
    /// update focus. False when the pass found nothing to do.
    fn step(&mut self) -> bool {
        let mut worked = false;
        while let Ok(chunk) = self.events_receiver.try_recv() {
            worked = true;
            match chunk {
                EventsChunk::Processed(events) => EventsExecutorAdapter {
                    runner: &mut self.runner,
                }
                .run(&mut self.nodes_state, events),
                EventsChunk::Batch(events) => {
                    for e in events {
                        self.runner
                            .handle_event(e.node_id, e.name, e.data, e.bubbles);
                    }
                }
            }
        }
        for event in self.user_events.take() {
            match event {
                UserEvent::RequestRedraw => self.needs_draw = true,
                UserEvent::FocusAccessibilityNode(strategy) => {
                    if let AccessibilityFocusStrategy::Node(id) = strategy {
                        self.platform.focused_accessibility_id.set_if_modified(id);
                    }
                    self.tree.accessibility_diff.request_focus(strategy);
                    self.a11y_dirty = true;
                }
                _ => {}
            }
        }

        let mutations = self.runner.sync_and_update();
        if !mutations.is_empty() {
            worked = true;
            let scale = self.scale as f32;
            let result = self
                .runner
                .run_in(|| self.tree.apply_mutations(mutations, scale));
            if let Some(strategy) = result.auto_focus {
                self.tree.accessibility_diff.request_focus(strategy);
            }
            self.layout_dirty |= result.needs_render;
            self.needs_draw |= result.needs_render;
            self.a11y_dirty |= result.needs_accessibility;
        }

        if self.layout_dirty {
            self.layout_dirty = false;
            self.tree.measure_layout(
                self.size,
                &mut self.font_collection,
                &self.font_manager,
                &self.events_sender,
                &mut self.nodes_state,
                self.scale,
                &self.fonts,
            );
        }
        if self.a11y_dirty {
            self.a11y_dirty = false;
            let update =
                self.accessibility
                    .process_updates(&mut self.tree, &self.events_sender, "island");
            self.platform
                .focused_accessibility_id
                .set_if_modified(update.focus);
        }
        worked
    }

    fn platform_event(&mut self, event: PlatformEvent) {
        let processed = EventsMeasurerAdapter {
            tree: &mut self.tree,
            scale_factor: self.scale,
        }
        .run(
            &mut vec![event],
            &mut self.nodes_state,
            self.accessibility.focused_node_id(),
        );
        let _ = self
            .events_sender
            .unbounded_send(EventsChunk::Processed(processed));
    }

    pub fn init(&mut self, renderer: &mut GlesRenderer, size: Size<i32, Physical>, scale: f64) {
        self.scale = scale;
        self.size = Size2D::new(size.w as f32, size.h as f32);
        self.platform.scale_factor.set(scale);
        self.platform.root_size.set(self.size);

        let (tex, fbo, max_attribs, interface) = renderer
            .with_context(|gl| unsafe {
                let mut tex = 0;
                gl.GenTextures(1, &mut tex);
                gl.BindTexture(ffi::TEXTURE_2D, tex);
                gl.TexImage2D(
                    ffi::TEXTURE_2D,
                    0,
                    ffi::RGBA8 as i32,
                    size.w,
                    size.h,
                    0,
                    ffi::RGBA,
                    ffi::UNSIGNED_BYTE,
                    std::ptr::null(),
                );
                for (param, value) in [
                    (ffi::TEXTURE_MIN_FILTER, ffi::LINEAR),
                    (ffi::TEXTURE_MAG_FILTER, ffi::LINEAR),
                    (ffi::TEXTURE_WRAP_S, ffi::CLAMP_TO_EDGE),
                    (ffi::TEXTURE_WRAP_T, ffi::CLAMP_TO_EDGE),
                ] {
                    gl.TexParameteri(ffi::TEXTURE_2D, param, value as i32);
                }
                gl.BindTexture(ffi::TEXTURE_2D, 0);

                let mut stencil = 0;
                gl.GenRenderbuffers(1, &mut stencil);
                gl.BindRenderbuffer(ffi::RENDERBUFFER, stencil);
                gl.RenderbufferStorage(ffi::RENDERBUFFER, ffi::STENCIL_INDEX8, size.w, size.h);
                gl.BindRenderbuffer(ffi::RENDERBUFFER, 0);

                let mut fbo = 0;
                gl.GenFramebuffers(1, &mut fbo);
                gl.BindFramebuffer(ffi::FRAMEBUFFER, fbo);
                gl.FramebufferTexture2D(
                    ffi::FRAMEBUFFER,
                    ffi::COLOR_ATTACHMENT0,
                    ffi::TEXTURE_2D,
                    tex,
                    0,
                );
                gl.FramebufferRenderbuffer(
                    ffi::FRAMEBUFFER,
                    ffi::STENCIL_ATTACHMENT,
                    ffi::RENDERBUFFER,
                    stencil,
                );
                let status = gl.CheckFramebufferStatus(ffi::FRAMEBUFFER);
                gl.BindFramebuffer(ffi::FRAMEBUFFER, 0);
                assert_eq!(
                    status,
                    ffi::FRAMEBUFFER_COMPLETE,
                    "content framebuffer incomplete"
                );

                let mut max_attribs = 0;
                gl.GetIntegerv(ffi::MAX_VERTEX_ATTRIBS, &mut max_attribs);

                // Skia looks up its GL entry points while Smithay's context is current.
                let interface = Interface::new_load_with(|name| {
                    if name == "eglGetCurrentDisplay" {
                        return std::ptr::null();
                    }
                    egl::get_proc_address(name)
                })
                .expect("no Skia GL interface");
                (tex, fbo, max_attribs as u32, interface)
            })
            .expect("GL context");

        let mut context = direct_contexts::make_gl(interface, None).expect("no Skia GL context");
        let target = backend_render_targets::make_gl(
            (size.w, size.h),
            0,
            8,
            FramebufferInfo {
                fboid: fbo,
                format: ffi::RGBA8,
                ..Default::default()
            },
        );
        let surface = wrap_backend_render_target(
            &mut context,
            &target,
            SurfaceOrigin::TopLeft,
            ColorType::RGBA8888,
            None,
            // Freya's own text contrast and gamma on Linux.
            Some(&SurfaceProps::new_with_text_properties(
                SurfacePropsFlags::default(),
                PixelGeometry::Unknown,
                0.2,
                1.0,
            )),
        )
        .expect("no Skia surface over the content framebuffer");
        let texture = unsafe {
            GlesTexture::from_raw(
                renderer,
                Some(ffi::RGBA8),
                false,
                tex,
                (size.w, size.h).into(),
            )
        };
        renderer
            .with_context(|gl| unsafe { restore_gl(gl, max_attribs) })
            .expect("GL context");
        self.gpu = Some(Gpu {
            context,
            surface,
            texture,
            max_attribs,
        });

        while self.step() {}
    }

    pub fn event(&mut self, event: UiEvent) {
        let event = match event {
            UiEvent::PointerMove { x, y } => {
                self.cursor = CursorPoint::new(x, y);
                PlatformEvent::Mouse {
                    name: MouseEventName::MouseMove,
                    cursor: self.cursor,
                    button: None,
                }
            }
            UiEvent::PointerButton { pressed } => {
                self.platform
                    .navigation_mode
                    .set_if_modified(NavigationMode::NotKeyboard);
                PlatformEvent::Mouse {
                    name: if pressed {
                        MouseEventName::MouseDown
                    } else {
                        MouseEventName::MouseUp
                    },
                    cursor: self.cursor,
                    button: Some(MouseButton::Left),
                }
            }
            UiEvent::PointerLeave => PlatformEvent::PointerExit {
                cursor: self.cursor,
            },
            UiEvent::Key {
                keysym,
                text,
                pressed,
            } => PlatformEvent::Keyboard {
                name: if pressed {
                    KeyboardEventName::KeyDown
                } else {
                    KeyboardEventName::KeyUp
                },
                key: map_key(keysym, text),
                code: Code::Unidentified,
                modifiers: Modifiers::empty(),
            },
        };
        self.platform_event(event);
    }

    pub fn tick(&mut self) -> bool {
        // Handlers and tasks can dirty more state; settle it within the frame.
        for _ in 0..4 {
            if !self.step() {
                break;
            }
        }
        self.needs_draw
    }

    pub fn render(&mut self, renderer: &mut GlesRenderer) -> GlesTexture {
        let gpu = self.gpu.as_mut().expect("render before init");
        if self.needs_draw {
            self.needs_draw = false;
            renderer
                .with_context(|gl| {
                    // Smithay has moved GL state since Skia last drew.
                    gpu.context.reset(None);
                    RenderPipeline {
                        font_collection: &mut self.font_collection,
                        font_manager: &self.font_manager,
                        canvas: gpu.surface.canvas(),
                        tree: &self.tree,
                        scale_factor: self.scale,
                        background: Color::TRANSPARENT,
                    }
                    .render();
                    gpu.context.flush_and_submit();
                    unsafe { restore_gl(gl, gpu.max_attribs) };
                })
                .expect("GL context");
            self.ticker.notify();
        }
        gpu.texture.clone()
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }

    /// Moves keyboard focus to `id` on the next tick.
    pub fn request_focus(&mut self, id: AccessibilityId) {
        self.tree
            .accessibility_diff
            .request_focus(AccessibilityFocusStrategy::Node(id));
        self.a11y_dirty = true;
    }
}

impl Drop for Embedded {
    fn drop(&mut self) {
        // The base drops the GL context before the content, so Skia must not touch it.
        if let Some(gpu) = &mut self.gpu {
            gpu.context.abandon();
        }
    }
}

/// Puts back the GL state Smithay's renderer expects and Skia may have changed.
unsafe fn restore_gl(gl: &ffi::Gles2, max_attribs: u32) {
    unsafe {
        gl.BindFramebuffer(ffi::FRAMEBUFFER, 0);
        gl.BindRenderbuffer(ffi::RENDERBUFFER, 0);
        gl.BindVertexArray(0);
        gl.BindBuffer(ffi::ARRAY_BUFFER, 0);
        gl.BindBuffer(ffi::ELEMENT_ARRAY_BUFFER, 0);
        gl.BindBuffer(ffi::PIXEL_PACK_BUFFER, 0);
        gl.BindBuffer(ffi::PIXEL_UNPACK_BUFFER, 0);
        for i in 0..max_attribs {
            gl.DisableVertexAttribArray(i);
        }
        gl.UseProgram(0);
        gl.ActiveTexture(ffi::TEXTURE0);
        gl.BindTexture(ffi::TEXTURE_2D, 0);
        gl.BindSampler(0, 0);
        gl.Disable(ffi::STENCIL_TEST);
        gl.Disable(ffi::DEPTH_TEST);
        gl.Disable(ffi::CULL_FACE);
        gl.Disable(ffi::SCISSOR_TEST);
        gl.Enable(ffi::BLEND);
        gl.BlendEquation(ffi::FUNC_ADD);
        gl.BlendFunc(ffi::ONE, ffi::ONE_MINUS_SRC_ALPHA);
        gl.ColorMask(ffi::TRUE, ffi::TRUE, ffi::TRUE, ffi::TRUE);
        gl.PixelStorei(ffi::PACK_ALIGNMENT, 4);
        gl.PixelStorei(ffi::UNPACK_ALIGNMENT, 4);
        gl.PixelStorei(ffi::PACK_ROW_LENGTH, 0);
        gl.PixelStorei(ffi::UNPACK_ROW_LENGTH, 0);
    }
}

fn map_key(keysym: Keysym, text: Option<String>) -> Key {
    let named = match keysym {
        Keysym::BackSpace => NamedKey::Backspace,
        Keysym::Return | Keysym::KP_Enter => NamedKey::Enter,
        Keysym::Tab => NamedKey::Tab,
        Keysym::Escape => NamedKey::Escape,
        Keysym::Delete => NamedKey::Delete,
        Keysym::Left => NamedKey::ArrowLeft,
        Keysym::Right => NamedKey::ArrowRight,
        Keysym::Up => NamedKey::ArrowUp,
        Keysym::Down => NamedKey::ArrowDown,
        Keysym::Home => NamedKey::Home,
        Keysym::End => NamedKey::End,
        Keysym::Shift_L | Keysym::Shift_R => NamedKey::Shift,
        Keysym::Control_L | Keysym::Control_R => NamedKey::Control,
        Keysym::Alt_L | Keysym::Alt_R => NamedKey::Alt,
        Keysym::Super_L | Keysym::Super_R => NamedKey::Meta,
        _ => match text {
            Some(text) if !text.chars().any(char::is_control) => return Key::Character(text),
            _ => NamedKey::Unidentified,
        },
    };
    Key::Named(named)
}
