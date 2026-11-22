//! An element drawn once to pixels, off any window: a drag's image.

use std::{cell::RefCell, rc::Rc, time::Duration};

use freya::{prelude::*, torin::prelude::Point2D};
use freya_core::integration::*;
use freya_engine::prelude::{
    AlphaType, ColorType, FontCollection, FontMgr, ImageInfo, TypefaceFontProvider,
    raster_n32_premul,
};
use ragnarok::EventsExecutorRunner;

use crate::ThemeContext;

/// Premultiplied pixels, B, G, R, A in memory, rows `width * 4` bytes long.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// The longest a snapshot waits for images that load on a thread.
const SETTLE: Duration = Duration::from_millis(40);

/// Draws `element` in `size` logical pixels at `scale`, in the theme in force where it is
/// called, on a clear ground. Call it from a render or an event of a window.
pub fn snapshot(
    size: (f32, f32),
    scale: f64,
    element: impl Fn() -> Element + 'static,
) -> Option<Snapshot> {
    let theme = crate::theme();
    let (width, height) = (
        (size.0 as f64 * scale).ceil() as i32,
        (size.1 as f64 * scale).ceil() as i32,
    );
    if width <= 0 || height <= 0 {
        return None;
    }
    let root_size = Size2D::new(size.0, size.1);
    let (events_sender, mut events_receiver) = futures_channel::mpsc::unbounded::<EventsChunk>();

    let mut runner = Runner::new(move || {
        rect()
            .width(Size::px(size.0))
            .height(Size::px(size.1))
            .child(element())
            .into_element()
    });
    runner.provide_root_context(GlobalContexts::default);
    runner.provide_root_context(ScreenReader::new);
    let (_ticker_sender, ticker) = RenderingTicker::new();
    runner.provide_root_context(|| ticker);
    runner.provide_root_context(AnimationClock::new);
    runner.provide_root_context(AssetCacher::create);
    runner.provide_root_context(TargetPlatform::detect);
    runner.provide_root_context(|| Platform {
        focused_accessibility_id: State::create(ACCESSIBILITY_ROOT_ID),
        focused_accessibility_node: State::create(accesskit::Node::new(accesskit::Role::Window)),
        root_size: State::create(root_size),
        window_position: State::create(Point2D::default()),
        is_maximized: State::create(false),
        is_fullscreen: State::create(false),
        scale_factor: State::create(scale),
        custom_scale_factor: State::create(1.0),
        navigation_mode: State::create(NavigationMode::NotKeyboard),
        preferred_theme: State::create(PreferredTheme::Light),
        is_app_focused: State::create(true),
        accent_color: State::create(AccentColor::default()),
        sender: Rc::new(|_| {}),
    });
    let tree = Rc::new(RefCell::new(freya_core::integration::Tree::default()));
    runner.provide_root_context(|| tree.borrow().accessibility_generator.clone());
    runner.provide_root_context(|| ThemeContext(State::create(theme.clone())));

    let mut font_collection = FontCollection::new();
    let font_manager: FontMgr = TypefaceFontProvider::new().into();
    font_collection.set_default_font_manager(FontMgr::default(), None);
    font_collection.set_dynamic_font_manager(font_manager.clone());
    runner.provide_root_context(|| font_collection.clone());
    let fonts = default_fonts();
    let mut nodes_state = NodesState::default();

    // Rounds until nothing changes twice running: sizes measured, then images drawn at them.
    let started = std::time::Instant::now();
    let mut quiet = 0;
    while quiet < 2 && started.elapsed() < SETTLE {
        runner.handle_events_immediately();
        while let Ok(chunk) = events_receiver.try_recv() {
            match chunk {
                EventsChunk::Processed(processed) => {
                    EventsExecutorAdapter {
                        runner: &mut runner,
                    }
                    .run(&mut nodes_state, processed);
                }
                EventsChunk::Batch(events) => {
                    for event in events {
                        runner.handle_event(event.node_id, event.name, event.data, event.bubbles);
                    }
                }
            }
        }
        let mutations = runner.sync_and_update();
        quiet = if mutations.is_empty() { quiet + 1 } else { 0 };
        runner.run_in(|| tree.borrow_mut().apply_mutations(mutations, scale as f32));
        tree.borrow_mut().measure_layout(
            root_size,
            &mut font_collection,
            &font_manager,
            &events_sender,
            &mut nodes_state,
            scale,
            &fonts,
        );
        if quiet > 0 {
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    let mut surface = raster_n32_premul((width, height))?;
    RenderPipeline {
        font_collection: &mut font_collection,
        font_manager: &font_manager,
        tree: &tree.borrow(),
        canvas: surface.canvas(),
        scale_factor: scale,
        background: Color::TRANSPARENT,
    }
    .render();
    let info = ImageInfo::new(
        (width, height),
        ColorType::BGRA8888,
        AlphaType::Premul,
        None,
    );
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    surface
        .read_pixels(&info, &mut pixels, (width * 4) as usize, (0, 0))
        .then_some(Snapshot {
            width: width as u32,
            height: height as u32,
            pixels,
        })
}
