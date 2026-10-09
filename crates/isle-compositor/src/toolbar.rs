//! The capture toolbar of Shift+Cmd+5 over the live screen, and the countdown before a shot
//! (`p1/structure/capture.md`, The toolbar and The countdown). The bar is a shell surface;
//! the portion, the lifted window and the marked screen are compositor elements, never in a
//! shot.

use std::{cell::RefCell, collections::HashMap, time::Duration};

use isle_shell::capture::toolbar::{Bar, Control, Mode, ToolbarAction, ToolbarSurface};
use smithay::{
    backend::{
        input::ButtonState,
        renderer::{
            element::{
                Kind,
                solid::{SolidColorBuffer, SolidColorRenderElement},
            },
            gles::GlesRenderer,
        },
    },
    input::pointer::{CursorIcon, CursorImageStatus},
    output::Output,
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::{Logical, Point, Rectangle},
};
use tracing::{info, warn};

use crate::{
    capture::{Destination, Target},
    render::IsleElement,
    state::{Isle, State},
};

const BTN_LEFT: u32 = 0x110;
const KEY_ESC: u32 = 1;
const KEY_TAB: u32 = 15;
const KEY_ENTER: u32 = 28;
const KEY_SPACE: u32 = 57;
const KEY_KPENTER: u32 = 96;
const KEY_UP: u32 = 103;
const KEY_LEFT: u32 = 105;
const KEY_RIGHT: u32 = 106;
const KEY_DOWN: u32 = 108;

/// The hit area round a handle, in points.
const HANDLE_HIT: f64 = 12.0;
/// The portion a first open offers, centred, as a share of the screen.
const FIRST_PORTION: f64 = 0.5;

const MENU_SAVE_DEFAULT: u64 = 100;
const MENU_SAVE_DOCUMENTS: u64 = 101;
const MENU_SAVE_CLIPBOARD: u64 = 102;
const MENU_TIMER: u64 = 110;
const MENU_MIC_NONE: u64 = 120;
const MENU_MIC: u64 = 121;
const MENU_SYSTEM_AUDIO: u64 = 140;
const MENU_THUMBNAIL: u64 = 150;
const MENU_REMEMBER: u64 = 151;
const MENU_POINTER: u64 = 152;
const MENU_CLICKS: u64 = 153;

/// What a press on the live screen moves in portion mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Grip {
    Body,
    /// A corner or an edge: which of left, top, right and bottom it moves.
    Edge {
        left: bool,
        top: bool,
        right: bool,
        bottom: bool,
    },
    New,
}

#[derive(Clone, Copy, Debug)]
struct PortionDrag {
    grip: Grip,
    from: Point<f64, Logical>,
    start: Rectangle<f64, Logical>,
}

pub struct Toolbar {
    pub output: Output,
    pub mode: Mode,
    /// Ctrl+Shift+Cmd+5: shots go to the clipboard.
    pub copy: bool,
    /// The bar the user sees, in the global space.
    pub rect: Rectangle<i32, Logical>,
    /// The portion, in the global space on `output`.
    pub portion: Rectangle<f64, Logical>,
    drag: Option<PortionDrag>,
    /// The bar following the pointer, from this offset.
    grab: Option<Point<f64, Logical>>,
    pub focus: Option<Control>,
    /// Window mode: the shown window under the pointer.
    pub lifted: Option<u64>,
    menu_open: bool,
    /// Dragged this time: its place is kept.
    moved: bool,
    /// A portal's interactive Screenshot: the shot goes to the app.
    request: Option<PortalShot>,
    solids: RefCell<HashMap<(String, usize), SolidColorBuffer>>,
    /// Each output's round handles and the rounded line round a lifted window.
    shapes: crate::wm::draw::Shapes,
}

/// An app's screenshot request the toolbar answers.
pub struct PortalShot {
    app: String,
    icon: Option<std::path::PathBuf>,
    reply: Option<tokio::sync::oneshot::Sender<Option<std::path::PathBuf>>>,
}

/// A shot or recording waiting for its timer.
pub struct Countdown {
    pub left: u32,
    timer: Option<RegistrationToken>,
    then: Pending,
}

#[derive(Debug)]
enum Pending {
    Shot { target: Target, copy: bool },
    Record { source: crate::cast::Source },
    /// An app's shot, its request answered with the file; dropped with the countdown, it is
    /// answered cancel.
    ForApp {
        target: Target,
        reply: tokio::sync::oneshot::Sender<Option<std::path::PathBuf>>,
    },
}

impl Isle {
    /// The toolbar's drawing over the live screen on `output`, front to back, under the bar.
    pub fn toolbar_elements(
        &self,
        renderer: &mut GlesRenderer,
        output: &Output,
        push: &mut dyn FnMut(IsleElement),
    ) {
        let Some(toolbar) = &self.toolbar else {
            return;
        };
        let Some(geometry) = self.space.output_geometry(output) else {
            return;
        };
        let scale = output.current_scale().fractional_scale();
        let t = &self.shell.theme().tokens;
        let mut solids = toolbar.solids.borrow_mut();
        let mut solid =
            |slot: usize, rect: Rectangle<f64, Logical>, colour: isle_tokens::Colour| {
                let buffer = solids.entry((output.name(), slot)).or_default();
                let size = rect.size.to_i32_round::<i32>();
                buffer.update(
                    (size.w.max(0), size.h.max(0)),
                    [
                        colour.r as f32 / 255.0 * colour.a,
                        colour.g as f32 / 255.0 * colour.a,
                        colour.b as f32 / 255.0 * colour.a,
                        colour.a,
                    ],
                );
                let at = (rect.loc - geometry.loc.to_f64())
                    .to_physical(scale)
                    .to_i32_round();
                IsleElement::Solid(SolidColorRenderElement::from_buffer(
                    buffer,
                    at,
                    scale,
                    1.0,
                    Kind::Unspecified,
                ))
            };
        let line = t.size_capture_line as f64;
        let screen = geometry.to_f64();
        let mut slot = 0;
        let mut next = || {
            slot += 1;
            slot
        };
        let ring = |r: Rectangle<f64, Logical>,
                    push: &mut dyn FnMut(IsleElement),
                    solid: &mut dyn FnMut(
            usize,
            Rectangle<f64, Logical>,
            isle_tokens::Colour,
        ) -> IsleElement,
                    next: &mut dyn FnMut() -> usize| {
            for (width, colour) in [
                (line, t.colour_capture_line),
                (2.0 * line, t.colour_capture_line_shadow),
            ] {
                let o = Rectangle::new(
                    (r.loc.x - width, r.loc.y - width).into(),
                    (r.size.w + 2.0 * width, r.size.h + 2.0 * width).into(),
                );
                for side in [
                    Rectangle::new(o.loc, (o.size.w, line).into()),
                    Rectangle::new(
                        (o.loc.x, o.loc.y + o.size.h - line).into(),
                        (o.size.w, line).into(),
                    ),
                    Rectangle::new(
                        (o.loc.x, o.loc.y + line).into(),
                        (line, o.size.h - 2.0 * line).into(),
                    ),
                    Rectangle::new(
                        (o.loc.x + o.size.w - line, o.loc.y + line).into(),
                        (line, o.size.h - 2.0 * line).into(),
                    ),
                ] {
                    push(solid(next(), side, colour));
                }
            }
        };
        let veil_around = |cut: Rectangle<f64, Logical>,
                           push: &mut dyn FnMut(IsleElement),
                           solid: &mut dyn FnMut(
            usize,
            Rectangle<f64, Logical>,
            isle_tokens::Colour,
        ) -> IsleElement,
                           next: &mut dyn FnMut() -> usize| {
            let (l, top, r, b) = (
                screen.loc.x,
                screen.loc.y,
                screen.loc.x + screen.size.w,
                screen.loc.y + screen.size.h,
            );
            let (cl, ct, cr, cb) = (
                cut.loc.x,
                cut.loc.y,
                cut.loc.x + cut.size.w,
                cut.loc.y + cut.size.h,
            );
            for rect in [
                Rectangle::new((l, top).into(), (r - l, ct - top).into()),
                Rectangle::new((l, cb).into(), (r - l, b - cb).into()),
                Rectangle::new((l, ct).into(), (cl - l, cb - ct).into()),
                Rectangle::new((cr, ct).into(), (r - cr, cb - ct).into()),
            ] {
                push(solid(next(), rect, t.colour_veil));
            }
        };

        if toolbar.mode.portion() {
            if &toolbar.output != output {
                push(solid(next(), screen, t.colour_veil));
                return;
            }
            let p = toolbar.portion;
            // Handles over the line, the line over the veil.
            let program = self.window_programs.as_ref().map(|p| &p.shadow);
            let size = t.size_capture_handle as f64;
            for (n, (x, y)) in handle_points(p).into_iter().enumerate() {
                if let Some(program) = program {
                    let at = (x - geometry.loc.x as f64, y - geometry.loc.y as f64);
                    for disc in toolbar.shapes.handle(&output.name(), 2 * n, program, at, t) {
                        push(IsleElement::Window(crate::wm::draw::unplaced(
                            crate::wm::draw::WindowPiece::Shadow(disc),
                            scale,
                        )));
                    }
                    continue;
                }
                let ring_rect = Rectangle::new(
                    (x - size / 2.0 - 1.0, y - size / 2.0 - 1.0).into(),
                    (size + 2.0, size + 2.0).into(),
                );
                let dot =
                    Rectangle::new((x - size / 2.0, y - size / 2.0).into(), (size, size).into());
                push(solid(next(), dot, t.colour_capture_line));
                push(solid(next(), ring_rect, t.colour_capture_line_shadow));
            }
            ring(p, push, &mut solid, &mut next);
            veil_around(p, push, &mut solid, &mut next);
        } else if toolbar.mode.window() {
            if let Some(id) = toolbar.lifted
                && let Some(frame) = self.frame(id)
                && frame.overlaps(geometry)
            {
                match self.window_programs.as_ref() {
                    Some(programs) => {
                        let radius = self.window_style(id).corners[2];
                        for ring in toolbar.shapes.capture_ring(
                            &output.name(),
                            16,
                            &programs.shadow,
                            Rectangle::new(frame.loc - geometry.loc, frame.size),
                            radius,
                            line.round() as i32,
                            t,
                        ) {
                            push(IsleElement::Window(crate::wm::draw::unplaced(
                                crate::wm::draw::WindowPiece::Shadow(ring),
                                scale,
                            )));
                        }
                    }
                    None => ring(frame.to_f64(), push, &mut solid, &mut next),
                }
                let from = Rectangle::new(frame.loc - geometry.loc, frame.size);
                let to = from.to_f64();
                let mut pieces = Vec::new();
                self.window_pieces(renderer, id, geometry.loc, scale, false, 1.0, &mut |p| {
                    pieces.push(p)
                });
                for piece in pieces {
                    push(IsleElement::Window(crate::wm::draw::place(
                        piece, from, to, scale,
                    )));
                }
            }
            push(solid(next(), screen, t.colour_veil));
        } else if self.space.outputs().count() > 1 {
            let pointer = self.seat.get_pointer().map(|p| p.current_location());
            if pointer.is_some_and(|p| screen.contains(p)) {
                let inset = Rectangle::new(
                    (screen.loc.x + 2.0 * line, screen.loc.y + 2.0 * line).into(),
                    (screen.size.w - 4.0 * line, screen.size.h - 4.0 * line).into(),
                );
                ring(inset, push, &mut solid, &mut next);
            } else {
                push(solid(next(), screen, t.colour_veil));
            }
        }
    }
}

/// The portion's eight handles: corners and mid-edges.
fn handle_points(p: Rectangle<f64, Logical>) -> [(f64, f64); 8] {
    let (l, t, r, b) = (p.loc.x, p.loc.y, p.loc.x + p.size.w, p.loc.y + p.size.h);
    let (cx, cy) = ((l + r) / 2.0, (t + b) / 2.0);
    [
        (l, t),
        (cx, t),
        (r, t),
        (r, cy),
        (r, b),
        (cx, b),
        (l, b),
        (l, cy),
    ]
}

/// What a press at `at` takes hold of in portion `p`.
fn grip(p: Rectangle<f64, Logical>, at: Point<f64, Logical>) -> Grip {
    let (l, t, r, b) = (p.loc.x, p.loc.y, p.loc.x + p.size.w, p.loc.y + p.size.h);
    let near = |a: f64, b: f64| (a - b).abs() <= HANDLE_HIT;
    let within_x = at.x >= l - HANDLE_HIT && at.x <= r + HANDLE_HIT;
    let within_y = at.y >= t - HANDLE_HIT && at.y <= b + HANDLE_HIT;
    let (left, right) = (near(at.x, l) && within_y, near(at.x, r) && within_y);
    let (top, bottom) = (near(at.y, t) && within_x, near(at.y, b) && within_x);
    if left || right || top || bottom {
        return Grip::Edge {
            left,
            top,
            right,
            bottom,
        };
    }
    if p.contains(at) {
        Grip::Body
    } else {
        Grip::New
    }
}

/// `start` moved by the pointer's travel `d` as `grip` holds it, kept on `screen`.
fn dragged(
    grip: Grip,
    start: Rectangle<f64, Logical>,
    from: Point<f64, Logical>,
    at: Point<f64, Logical>,
    screen: Rectangle<f64, Logical>,
) -> Rectangle<f64, Logical> {
    let at = Point::from((
        at.x.clamp(screen.loc.x, screen.loc.x + screen.size.w),
        at.y.clamp(screen.loc.y, screen.loc.y + screen.size.h),
    ));
    let d = at - from;
    let (mut l, mut t, mut r, mut b) = (
        start.loc.x,
        start.loc.y,
        start.loc.x + start.size.w,
        start.loc.y + start.size.h,
    );
    match grip {
        Grip::Body => {
            let dx =
                d.x.clamp(screen.loc.x - l, screen.loc.x + screen.size.w - r);
            let dy =
                d.y.clamp(screen.loc.y - t, screen.loc.y + screen.size.h - b);
            l += dx;
            r += dx;
            t += dy;
            b += dy;
        }
        Grip::Edge {
            left,
            top,
            right,
            bottom,
        } => {
            if left {
                l += d.x;
            }
            if right {
                r += d.x;
            }
            if top {
                t += d.y;
            }
            if bottom {
                b += d.y;
            }
        }
        Grip::New => {
            (l, r) = (from.x.min(at.x), from.x.max(at.x));
            (t, b) = (from.y.min(at.y), from.y.max(at.y));
        }
    }
    let (l, r) = (l.min(r), l.max(r));
    let (t, b) = (t.min(b), t.max(b));
    Rectangle::new(
        (l.round(), t.round()).into(),
        ((r - l).round(), (b - t).round()).into(),
    )
}

impl State {
    /// Shift+Cmd+5: the toolbar on the screen with the pointer, on the last mode.
    pub fn open_toolbar(&mut self, copy: bool) {
        if self.isle.lock.locked {
            return;
        }
        if self.isle.toolbar.is_some() {
            return;
        }
        self.close_pick();
        self.cancel_switcher();
        self.close_session_dialog();
        self.close_force_quit();
        self.close_spotlight();
        self.close_menu_bar_group();
        let Some(output) = self
            .isle
            .output_under_pointer()
            .or_else(|| self.isle.main_output())
        else {
            return;
        };
        let Some(screen) = self.isle.space.output_geometry(&output) else {
            return;
        };
        let identity = Isle::output_identity(&output);
        let kept = self.isle.capture_state.clone();
        // Reopened while recording, it opens on the running recording's mode.
        let mode = self.recording_mode().unwrap_or_else(|| {
            Mode::from_key(&kept.mode)
                .filter(|m| !m.records() || crate::recording::available())
                .unwrap_or_default()
        });
        let screen_f = screen.to_f64();
        let portion = kept
            .portion
            .as_ref()
            .filter(|p| p.output == identity && self.isle.settings.capture.remember_selection)
            .map(|p| {
                Rectangle::new(
                    (screen_f.loc.x + p.x, screen_f.loc.y + p.y).into(),
                    (p.width, p.height).into(),
                )
            })
            .unwrap_or_else(|| {
                let (w, h) = (
                    screen_f.size.w * FIRST_PORTION,
                    screen_f.size.h * FIRST_PORTION,
                );
                Rectangle::new(
                    (
                        (screen_f.loc.x + (screen_f.size.w - w) / 2.0).round(),
                        (screen_f.loc.y + (screen_f.size.h - h) / 2.0).round(),
                    )
                        .into(),
                    (w.round(), h.round()).into(),
                )
            });
        self.isle.toolbar = Some(Toolbar {
            output: output.clone(),
            mode,
            copy,
            rect: Rectangle::default(),
            portion,
            drag: None,
            grab: None,
            focus: None,
            lifted: None,
            menu_open: false,
            moved: false,
            request: None,
            solids: RefCell::default(),
            shapes: Default::default(),
        });
        self.place_toolbar();
        self.toolbar_changed();
        info!(mode = mode.key(), "capture toolbar");
    }

    pub fn close_toolbar(&mut self) {
        let Some(mut toolbar) = self.isle.toolbar.take() else {
            return;
        };
        // An app's request closed without a shot is answered cancel.
        if let Some(reply) = toolbar.request.as_mut().and_then(|r| r.reply.take()) {
            let _ = reply.send(None);
        }
        if toolbar.menu_open {
            self.close_surface("window-menu");
        }
        self.isle.shell.close("capture-toolbar");
        self.isle.shell.close("capture-readout");
        self.isle.cursor_status = CursorImageStatus::default_named();
        self.backend.update_cursor(&self.isle);
        self.keep_capture_state();
        self.isle.queue_redraw_all();
    }

    /// The bar's place: where it was dragged to on this screen, else centred above the
    /// work area's foot.
    fn place_toolbar(&mut self) {
        let Some(toolbar) = &self.isle.toolbar else {
            return;
        };
        let output = toolbar.output.clone();
        let Some(screen) = self.isle.space.output_geometry(&output) else {
            return;
        };
        let theme = self.isle.shell.theme().clone();
        let bar = self.toolbar_bar();
        let size = ToolbarSurface::size_for(&theme, &bar);
        let margin = ToolbarSurface::margin(&theme) as i32;
        let (w, h) = (size.w - 2 * margin, size.h - 2 * margin);
        let identity = Isle::output_identity(&output);
        let kept = self
            .isle
            .capture_state
            .toolbar
            .clone()
            .filter(|s| s.output == identity);
        let work = self.isle.work_area(&output);
        let loc: Point<i32, Logical> = match kept {
            Some(spot) => (screen.loc.x + spot.x, screen.loc.y + spot.y).into(),
            None => (
                screen.loc.x + (screen.size.w - w) / 2,
                work.loc.y + work.size.h - theme.tokens.space_xxxl as i32 - h,
            )
                .into(),
        };
        let loc = Point::from((
            loc.x.clamp(screen.loc.x, screen.loc.x + screen.size.w - w),
            loc.y.clamp(screen.loc.y, screen.loc.y + screen.size.h - h),
        ));
        if let Some(toolbar) = self.isle.toolbar.as_mut() {
            toolbar.rect = Rectangle::new(loc, (w, h).into());
        }
    }

    fn toolbar_bar(&self) -> Bar {
        let Some(toolbar) = &self.isle.toolbar else {
            return Bar::default();
        };
        let theme = self.isle.shell.theme();
        let game = self.game_in_front();
        if let Some(request) = &toolbar.request {
            return Bar {
                mode: toolbar.mode,
                allowed: Mode::ALL.into_iter().filter(|m| !m.records()).collect(),
                recording: false,
                request: Some((request.app.clone(), request.icon.clone())),
                focus: toolbar.focus,
                solid: theme.appearance.reduce_transparency || game,
                contrast: theme.appearance.increase_contrast,
            };
        }
        Bar {
            mode: toolbar.mode,
            allowed: Mode::ALL
                .into_iter()
                .filter(|m| {
                    // Recording needs the recorder; while one runs, only its mode and shots.
                    if !m.records() {
                        return true;
                    }
                    match self.recording_mode() {
                        Some(running) => *m == running,
                        None => crate::recording::available(),
                    }
                })
                .collect(),
            recording: self.recording(),
            request: None,
            focus: toolbar.focus,
            solid: theme.appearance.reduce_transparency || game,
            contrast: theme.appearance.increase_contrast,
        }
    }

    /// The bar, the cursor and the drawing follow the toolbar's state.
    fn toolbar_changed(&mut self) {
        let Some(toolbar) = &self.isle.toolbar else {
            return;
        };
        let output = toolbar.output.clone();
        let rect = toolbar.rect;
        let mode = toolbar.mode;
        let portion = toolbar.portion;
        let Some(screen) = self.isle.space.output_geometry(&output) else {
            return;
        };
        let theme = self.isle.shell.theme().clone();
        let bar = self.toolbar_bar();
        let size = ToolbarSurface::size_for(&theme, &bar);
        let margin = ToolbarSurface::margin(&theme) as i32;
        let surface = self.isle.capture_toolbar.clone();
        surface.set_model(bar.clone());
        self.isle
            .shell
            .with_root("capture-toolbar", || surface.set(bar.clone()));
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        let geometry = Rectangle::new(rect.loc - screen.loc - Point::from((margin, margin)), size);
        self.isle
            .shell
            .open_placed("capture-toolbar", &place, geometry);
        if mode.portion() {
            self.show_readout(&output, portion);
        } else {
            self.isle.shell.close("capture-readout");
        }
        let multi = self.isle.space.outputs().count() > 1;
        self.isle.cursor_status =
            CursorImageStatus::Named(if mode.window() || (multi && !mode.portion()) {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            });
        self.backend.update_cursor(&self.isle);
        self.dispatch_shell();
        self.isle.queue_redraw_all();
    }

    pub fn toolbar_action(&mut self, action: ToolbarAction) {
        if self.isle.toolbar.is_none() {
            return;
        }
        match action {
            ToolbarAction::Close => self.close_toolbar(),
            ToolbarAction::Mode(mode) => self.set_toolbar_mode(mode),
            ToolbarAction::PickColour => {
                self.close_toolbar();
                self.open_pick(crate::freeze::Purpose::Colour);
            }
            ToolbarAction::Options => self.open_toolbar_options(),
            ToolbarAction::Act => self.toolbar_act(),
            ToolbarAction::Grab => {
                let at = self.isle.seat.get_pointer().unwrap().current_location();
                if let Some(toolbar) = self.isle.toolbar.as_mut() {
                    toolbar.grab = Some(at - toolbar.rect.loc.to_f64());
                }
            }
        }
    }

    fn set_toolbar_mode(&mut self, mode: Mode) {
        if !self.toolbar_bar().allowed.contains(&mode) {
            return;
        }
        let Some(toolbar) = self.isle.toolbar.as_mut() else {
            return;
        };
        toolbar.mode = mode;
        toolbar.lifted = None;
        self.isle.capture_state.mode = mode.key().into();
        self.place_toolbar();
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        self.toolbar_pointer(at);
        self.toolbar_changed();
    }

    /// The pointer moved with the toolbar open; true when the toolbar took the motion.
    pub fn toolbar_pointer(&mut self, at: Point<f64, Logical>) -> bool {
        let Some(toolbar) = self.isle.toolbar.as_mut() else {
            return false;
        };
        if let Some(offset) = toolbar.grab {
            toolbar.rect.loc = (at - offset).to_i32_round();
            toolbar.moved = true;
            self.toolbar_changed();
            return true;
        }
        if let Some(drag) = toolbar.drag {
            let screen = self
                .isle
                .space
                .output_geometry(&toolbar.output)
                .unwrap_or_default()
                .to_f64();
            toolbar.portion = dragged(drag.grip, drag.start, drag.from, at, screen);
            self.toolbar_changed();
            return true;
        }
        if toolbar.mode.window() {
            let lifted = self
                .isle
                .space
                .element_under(at)
                .map(|(w, _)| crate::wm::window_id(w))
                .filter(|id| self.isle.wm.is_shown(*id));
            let toolbar = self.isle.toolbar.as_mut().unwrap();
            if lifted != toolbar.lifted {
                toolbar.lifted = lifted;
                self.isle.queue_redraw_all();
            }
        } else if !toolbar.mode.portion() {
            self.isle.queue_redraw_all();
        }
        false
    }

    /// A button with the toolbar open, off the bar; true when the toolbar took it.
    pub fn toolbar_button(&mut self, button: u32, state: ButtonState) -> bool {
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        let Some(toolbar) = self.isle.toolbar.as_mut() else {
            return false;
        };
        if state == ButtonState::Released {
            let held = toolbar.drag.take().is_some() | toolbar.grab.take().is_some();
            if held {
                self.keep_capture_state();
            }
            return held;
        }
        if toolbar.rect.to_f64().contains(at) || button != BTN_LEFT {
            return false;
        }
        let mode = toolbar.mode;
        if mode.portion() {
            let on = self
                .isle
                .space
                .output_geometry(&toolbar.output)
                .is_some_and(|g| g.to_f64().contains(at));
            if !on {
                return true;
            }
            toolbar.drag = Some(PortionDrag {
                grip: grip(toolbar.portion, at),
                from: at,
                start: toolbar.portion,
            });
            return true;
        }
        if mode.window() {
            if let Some(id) = toolbar.lifted {
                if mode.records() {
                    if !self.recording() {
                        self.toolbar_record(crate::cast::Source::Window(id));
                    }
                } else {
                    let shadow = !self.held_mods().option;
                    self.toolbar_take(Target::Window { id, shadow });
                }
            }
            return true;
        }
        // A screen: with several, the click names it.
        if let Some(output) = self.isle.output_under_pointer() {
            if mode.records() {
                if !self.recording() {
                    self.toolbar_record(crate::cast::Source::Screen(output));
                }
            } else {
                self.toolbar_take(Target::Screen(output));
            }
        }
        true
    }

    /// A key with the toolbar open; every key but the modifiers is the toolbar's.
    pub fn toolbar_key(&mut self, code: u32) -> bool {
        let mods = self.held_mods();
        let Some(toolbar) = self.isle.toolbar.as_mut() else {
            return false;
        };
        let groups = [
            Control::Close,
            Control::Modes,
            Control::PickColour,
            Control::Options,
            Control::Act,
        ];
        let has_act = !toolbar.mode.window();
        match code {
            KEY_ESC => self.close_toolbar(),
            KEY_TAB => {
                let present: Vec<Control> = groups
                    .into_iter()
                    .filter(|c| *c != Control::Act || has_act)
                    .collect();
                let i = toolbar
                    .focus
                    .and_then(|f| present.iter().position(|c| *c == f));
                let n = present.len();
                toolbar.focus = Some(
                    present[match (i, mods.shift) {
                        (None, false) => 0,
                        (None, true) => n - 1,
                        (Some(i), false) => (i + 1) % n,
                        (Some(i), true) => (i + n - 1) % n,
                    }],
                );
                self.toolbar_changed();
            }
            KEY_LEFT | KEY_RIGHT if toolbar.focus == Some(Control::Modes) => {
                let current = toolbar.mode;
                let allowed = self.toolbar_bar().allowed;
                let i = allowed.iter().position(|m| *m == current).unwrap_or(0);
                let n = allowed.len();
                let j = if code == KEY_LEFT {
                    (i + n - 1) % n
                } else {
                    (i + 1) % n
                };
                self.set_toolbar_mode(allowed[j]);
            }
            KEY_UP | KEY_DOWN | KEY_LEFT | KEY_RIGHT if toolbar.mode.portion() => {
                let step = if mods.shift { 10.0 } else { 1.0 };
                let (dx, dy) = match code {
                    KEY_UP => (0.0, -step),
                    KEY_DOWN => (0.0, step),
                    KEY_LEFT => (-step, 0.0),
                    _ => (step, 0.0),
                };
                let screen = self
                    .isle
                    .space
                    .output_geometry(&toolbar.output)
                    .unwrap_or_default()
                    .to_f64();
                let start = toolbar.portion;
                let from = start.loc;
                let grip = if mods.option {
                    Grip::Edge {
                        left: false,
                        top: false,
                        right: dx != 0.0,
                        bottom: dy != 0.0,
                    }
                } else {
                    Grip::Body
                };
                toolbar.portion = dragged(grip, start, from, from + Point::from((dx, dy)), screen);
                self.toolbar_changed();
            }
            KEY_ENTER | KEY_KPENTER | KEY_SPACE => match toolbar.focus {
                Some(Control::Close) => self.close_toolbar(),
                Some(Control::PickColour) => self.toolbar_action(ToolbarAction::PickColour),
                Some(Control::Options) => self.open_toolbar_options(),
                Some(Control::Modes) if code == KEY_SPACE => {}
                _ => self.toolbar_act(),
            },
            _ => {}
        }
        true
    }

    /// The action button: the mode's shot or recording, after the timer when one is set;
    /// with a recording running, Stop.
    fn toolbar_act(&mut self) {
        let Some(toolbar) = &self.isle.toolbar else {
            return;
        };
        if self.recording() && toolbar.mode.records() {
            self.close_toolbar();
            self.stop_recording();
            return;
        }
        let source = match toolbar.mode {
            Mode::RecordScreen => Some(crate::cast::Source::Screen(
                self.isle
                    .output_under_pointer()
                    .unwrap_or_else(|| toolbar.output.clone()),
            )),
            Mode::RecordPortion => self
                .isle
                .space
                .output_geometry(&toolbar.output)
                .map(|screen| {
                    crate::cast::Source::Portion(
                        toolbar.output.clone(),
                        Rectangle::new(
                            (toolbar.portion.loc - screen.loc.to_f64()).to_i32_round(),
                            toolbar.portion.size.to_i32_round(),
                        ),
                    )
                }),
            _ => None,
        };
        if let Some(source) = source {
            self.toolbar_record(source);
            return;
        }
        let target = match toolbar.mode {
            Mode::ShotPortion => {
                let Some(screen) = self.isle.space.output_geometry(&toolbar.output) else {
                    return;
                };
                let local = Rectangle::new(
                    (toolbar.portion.loc - screen.loc.to_f64()).to_i32_round(),
                    toolbar.portion.size.to_i32_round(),
                );
                Target::Region(toolbar.output.clone(), local)
            }
            Mode::ShotScreen => {
                let output = self
                    .isle
                    .output_under_pointer()
                    .unwrap_or_else(|| toolbar.output.clone());
                Target::Screen(output)
            }
            _ => return,
        };
        self.toolbar_take(target);
    }

    /// An app's interactive Screenshot: the toolbar on the shot modes, naming the app, its
    /// action Share.
    pub fn open_toolbar_for(
        &mut self,
        app_id: &str,
        reply: tokio::sync::oneshot::Sender<Option<std::path::PathBuf>>,
    ) {
        if self.isle.toolbar.is_some() || self.isle.lock.locked {
            let _ = reply.send(None);
            return;
        }
        self.open_toolbar(false);
        let app_id = app_id.trim_end_matches(".desktop");
        let app = self.isle.app_name(app_id);
        let request = PortalShot {
            app: if app.is_empty() { "An app".into() } else { app },
            icon: self.isle.app_icon(app_id, 16),
            reply: Some(reply),
        };
        let Some(toolbar) = self.isle.toolbar.as_mut() else {
            if let Some(reply) = request.reply {
                let _ = reply.send(None);
            }
            return;
        };
        if toolbar.mode.records() {
            toolbar.mode = Mode::ShotScreen;
        }
        toolbar.request = Some(request);
        self.place_toolbar();
        self.toolbar_changed();
    }

    fn toolbar_record(&mut self, source: crate::cast::Source) {
        self.close_toolbar();
        let timer = self.isle.settings.capture.timer.max(0) as u32;
        if timer > 0 {
            self.start_capture_countdown(timer, Pending::Record { source });
        } else {
            self.start_recording(source);
        }
    }

    fn toolbar_take(&mut self, target: Target) {
        if let Some(reply) = self
            .isle
            .toolbar
            .as_mut()
            .and_then(|t| t.request.as_mut())
            .and_then(|r| r.reply.take())
        {
            self.close_toolbar();
            let timer = self.isle.settings.capture.timer.max(0) as u32;
            if timer > 0 {
                self.start_capture_countdown(timer, Pending::ForApp { target, reply });
            } else {
                self.take_for_portal(target, reply);
            }
            return;
        }
        let copy = self.isle.toolbar.as_ref().is_some_and(|t| t.copy);
        self.close_toolbar();
        let timer = self.isle.settings.capture.timer.max(0) as u32;
        if timer > 0 {
            self.start_capture_countdown(timer, Pending::Shot { target, copy });
        } else {
            self.take_still(target, copy);
        }
    }

    /// Shoots `target` live, with the capture tool's parts left out.
    pub fn take_still(&mut self, target: Target, copy: bool) {
        let pointer = self.isle.settings.capture.show_pointer;
        let output = match &target {
            Target::Screen(o) | Target::Region(o, _) => Some(o.clone()),
            Target::Window { id, .. } => self.isle.output_of(*id),
        };
        for output in self.isle.space.outputs().cloned().collect::<Vec<_>>() {
            self.draw_shell(&output);
        }
        let pixels = match self.capture_pixels(&target, pointer) {
            Ok(pixels) => pixels,
            Err(err) => {
                warn!("no shot: {err}");
                return;
            }
        };
        let to = if copy {
            Destination::Clipboard
        } else {
            self.still_destination()
        };
        self.shutter();
        let preview = to != Destination::Clipboard && self.isle.settings.capture.floating_thumbnail;
        self.keep_stills(vec![pixels], to, preview, move |state, written, small| {
            if let (Some(path), Some(small), Some(output)) = (written.first(), small, output) {
                state.show_thumbnail(path.clone(), &output, small);
            }
        });
    }

    fn start_capture_countdown(&mut self, seconds: u32, then: Pending) {
        self.cancel_capture_countdown();
        self.isle.countdown = Some(Countdown {
            left: seconds,
            timer: None,
            then,
        });
        self.tick_capture_countdown();
    }

    /// One second of the countdown: the island shows what is left, and at nought the shot,
    /// once the island has gone back.
    fn tick_capture_countdown(&mut self) {
        let Some(countdown) = self.isle.countdown.as_mut() else {
            return;
        };
        if countdown.left == 0 {
            let then = self.isle.countdown.take().unwrap().then;
            self.island_countdown(None);
            // The island returns to rest before the frame is taken.
            let settle = Duration::from_millis(
                self.isle.shell.theme().tokens.motion_exit_fade_ms as u64 + 100,
            );
            let mut then = Some(then);
            let _ = self.isle.loop_handle.insert_source(
                Timer::from_duration(settle),
                move |_, _, state| {
                    match then.take() {
                        Some(Pending::Shot { target, copy }) => state.take_still(target, copy),
                        Some(Pending::Record { source }) => state.start_recording(source),
                        Some(Pending::ForApp { target, reply }) => {
                            state.take_for_portal(target, reply)
                        }
                        None => {}
                    }
                    TimeoutAction::Drop
                },
            );
            return;
        }
        let left = countdown.left;
        countdown.left -= 1;
        self.island_countdown(Some(left));
        let token = self
            .isle
            .loop_handle
            .insert_source(
                Timer::from_duration(Duration::from_secs(1)),
                |_, _, state| {
                    if let Some(countdown) = state.isle.countdown.as_mut() {
                        countdown.timer = None;
                    }
                    state.tick_capture_countdown();
                    TimeoutAction::Drop
                },
            )
            .ok();
        if let Some(countdown) = self.isle.countdown.as_mut() {
            countdown.timer = token;
        }
    }

    /// Cmd+Ctrl+Escape, a click on it, or the lock: a countdown stops and nothing is taken.
    pub fn cancel_capture_countdown(&mut self) -> bool {
        let Some(countdown) = self.isle.countdown.take() else {
            return false;
        };
        if let Some(token) = countdown.timer {
            self.isle.loop_handle.remove(token);
        }
        self.island_countdown(None);
        true
    }

    fn island_countdown(&mut self, left: Option<u32>) {
        use isle_shell::island::model::{Event, EventKind, Form};
        match left {
            Some(left) => self.isle.island.model.post(
                Event {
                    kind: EventKind::Countdown,
                    form: Form::Line {
                        symbol: "camera".into(),
                        fill: false,
                        text: left.to_string(),
                    },
                    output: None,
                    app: None,
                    urgent: false,
                },
                std::time::Instant::now(),
            ),
            None => self
                .isle
                .island
                .model
                .withdraw(EventKind::Countdown, std::time::Instant::now()),
        }
        self.isle.refresh_island();
    }

    fn open_toolbar_options(&mut self) {
        use isle_ui::{MenuEntry, MenuItem, menu::Mark};
        let Some(toolbar) = self.isle.toolbar.as_mut() else {
            return;
        };
        toolbar.menu_open = true;
        let output = toolbar.output.clone();
        let at: Point<i32, Logical> = (
            toolbar.rect.loc.x + toolbar.rect.size.w / 2,
            toolbar.rect.loc.y - 4,
        )
            .into();
        let c = &self.isle.settings.capture;
        let check = |id, label: &str, on: bool| {
            MenuEntry::Item(MenuItem {
                mark: if on { Mark::Check } else { Mark::None },
                ..MenuItem::new(id, label)
            })
        };
        let save = c.save_to.as_str();
        let mut entries = vec![
            MenuEntry::Section("Save to".into()),
            check(
                MENU_SAVE_DEFAULT,
                "Screenshots",
                matches!(save, "" | "default"),
            ),
            check(MENU_SAVE_DOCUMENTS, "Documents", save == "documents"),
            check(MENU_SAVE_CLIPBOARD, "Clipboard", save == "clipboard"),
            MenuEntry::Section("Timer".into()),
            check(MENU_TIMER, "None", c.timer <= 0),
            check(MENU_TIMER + 1, "5 s", c.timer == 5),
            check(MENU_TIMER + 2, "10 s", c.timer == 10),
            MenuEntry::Section("Microphone".into()),
            check(MENU_MIC_NONE, "None", c.microphone.is_none()),
        ];
        for (i, device) in self
            .isle
            .services
            .audio
            .state
            .inputs
            .iter()
            .enumerate()
            .take(16)
        {
            entries.push(check(
                MENU_MIC + i as u64,
                &device.label,
                c.microphone.as_deref() == Some(device.name.as_str()),
            ));
        }
        entries.extend([
            MenuEntry::Section("Audio".into()),
            check(MENU_SYSTEM_AUDIO, "Record system audio", c.system_audio),
            MenuEntry::Section("Options".into()),
            check(
                MENU_THUMBNAIL,
                "Show floating thumbnail",
                c.floating_thumbnail,
            ),
            check(
                MENU_REMEMBER,
                "Remember last selection",
                c.remember_selection,
            ),
            check(MENU_POINTER, "Show mouse pointer", c.show_pointer),
            check(MENU_CLICKS, "Show mouse clicks", c.show_clicks),
        ]);
        let rows = entries.len();
        let theme = self.isle.shell.theme().clone();
        let height = isle_shell::placed_menu::window_menu_size(&theme, rows).h;
        self.open_menu_at(entries, &output, (at.x, at.y - height).into());
    }

    /// The window menu's surface chose for the toolbar's Options; false when it is not its.
    pub fn toolbar_menu_event(&mut self, event: &isle_ui::MenuEvent) -> bool {
        use isle_settings::Value;
        let Some(toolbar) = self.isle.toolbar.as_mut() else {
            return false;
        };
        if !toolbar.menu_open {
            return false;
        }
        let id = match event {
            isle_ui::MenuEvent::Chosen(id) => *id,
            isle_ui::MenuEvent::Back => {
                toolbar.menu_open = false;
                self.close_surface("window-menu");
                return true;
            }
            _ => return true,
        };
        toolbar.menu_open = false;
        self.close_surface("window-menu");
        let c = self.isle.settings.capture.clone();
        let write =
            |key: &str, value: Value| self.isle.write_setting(&format!("capture.{key}"), value);
        match id {
            MENU_SAVE_DEFAULT => write("save-to", Value::Str("default".into())),
            MENU_SAVE_DOCUMENTS => write("save-to", Value::Str("documents".into())),
            MENU_SAVE_CLIPBOARD => write("save-to", Value::Str("clipboard".into())),
            id if (MENU_TIMER..=MENU_TIMER + 2).contains(&id) => {
                write("timer", Value::Int((id - MENU_TIMER) as i64 * 5))
            }
            MENU_MIC_NONE => write("microphone", Value::Str(String::new())),
            MENU_SYSTEM_AUDIO => write("system-audio", Value::Bool(!c.system_audio)),
            MENU_THUMBNAIL => write("floating-thumbnail", Value::Bool(!c.floating_thumbnail)),
            MENU_REMEMBER => write("remember-selection", Value::Bool(!c.remember_selection)),
            MENU_POINTER => write("show-pointer", Value::Bool(!c.show_pointer)),
            MENU_CLICKS => write("show-clicks", Value::Bool(!c.show_clicks)),
            id if (MENU_MIC..MENU_MIC + 16).contains(&id) => {
                if let Some(device) = self
                    .isle
                    .services
                    .audio
                    .state
                    .inputs
                    .get((id - MENU_MIC) as usize)
                {
                    write("microphone", Value::Str(device.name.clone()));
                }
            }
            _ => {}
        }
        true
    }

    /// The toolbar's mode, place and portion, written for its next opening.
    fn keep_capture_state(&mut self) {
        if let Some(toolbar) = &self.isle.toolbar
            && let Some(screen) = self.isle.space.output_geometry(&toolbar.output)
        {
            let identity = Isle::output_identity(&toolbar.output);
            let state = &mut self.isle.capture_state;
            state.mode = toolbar.mode.key().into();
            if toolbar.moved {
                state.toolbar = Some(isle_state::domains::Spot {
                    output: identity.clone(),
                    x: toolbar.rect.loc.x - screen.loc.x,
                    y: toolbar.rect.loc.y - screen.loc.y,
                });
            }
            state.portion = Some(isle_state::domains::Portion {
                output: identity,
                x: toolbar.portion.loc.x - screen.loc.x as f64,
                y: toolbar.portion.loc.y - screen.loc.y as f64,
                width: toolbar.portion.size.w,
                height: toolbar.portion.size.h,
            });
        }
        if let Some(saver) = &self.isle.capture_saver {
            saver.save(self.isle.capture_state.clone());
        }
    }

    /// The toolbar's mode for the recording running, if one is.
    fn recording_mode(&self) -> Option<Mode> {
        self.isle.recording_now.as_ref().map(|r| match r.source {
            crate::cast::Source::Screen(_) => Mode::RecordScreen,
            crate::cast::Source::Portion(..) => Mode::RecordPortion,
            crate::cast::Source::Window(_) => Mode::RecordWindow,
        })
    }

    /// The toolbar as JSON, for the Test interface.
    #[cfg(feature = "test-hooks")]
    pub fn toolbar_status(&self) -> String {
        let Some(t) = &self.isle.toolbar else {
            return serde_json::json!({"open": false, "countdown": self.isle.countdown.as_ref().map(|c| c.left)}).to_string();
        };
        serde_json::json!({
            "open": true,
            "mode": t.mode.key(),
            "output": t.output.name(),
            "rect": [t.rect.loc.x, t.rect.loc.y, t.rect.size.w, t.rect.size.h],
            "portion": [t.portion.loc.x, t.portion.loc.y, t.portion.size.w, t.portion.size.h],
            "lifted": t.lifted,
            "focus": t.focus.map(|f| format!("{f:?}")),
            "drawn": self.isle.shell.is_open("capture-toolbar"),
            "request": t.request.as_ref().map(|r| r.app.clone()),
        })
        .to_string()
    }
}

impl Isle {
    /// The output the pointer is on.
    pub fn output_under_pointer(&self) -> Option<Output> {
        let at = self.seat.get_pointer()?.current_location();
        self.space
            .outputs()
            .find(|o| {
                self.space
                    .output_geometry(o)
                    .is_some_and(|g| g.to_f64().contains(at))
            })
            .cloned()
    }

    pub fn load_capture_state(&mut self, runtime: &tokio::runtime::Handle) {
        let dir = isle_state::StateDir::new(isle_state::StateDir::default_dir());
        self.capture_state = dir.load().0;
        self.capture_saver = Some(crate::state_writer::Saver::spawn(runtime, dir));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f64, y: f64, w: f64, h: f64) -> Rectangle<f64, Logical> {
        Rectangle::new((x, y).into(), (w, h).into())
    }

    #[test]
    fn a_press_takes_a_handle_the_body_or_starts_anew() {
        let p = r(100.0, 100.0, 200.0, 100.0);
        assert_eq!(grip(p, (150.0, 150.0).into()), Grip::Body);
        assert_eq!(grip(p, (20.0, 20.0).into()), Grip::New);
        assert_eq!(
            grip(p, (300.0, 200.0).into()),
            Grip::Edge {
                left: false,
                top: false,
                right: true,
                bottom: true
            }
        );
        assert_eq!(
            grip(p, (200.0, 99.0).into()),
            Grip::Edge {
                left: false,
                top: true,
                right: false,
                bottom: false
            }
        );
    }

    #[test]
    fn the_body_moves_within_the_screen() {
        let screen = r(0.0, 0.0, 1000.0, 800.0);
        let p = r(100.0, 100.0, 200.0, 100.0);
        let moved = dragged(
            Grip::Body,
            p,
            (150.0, 150.0).into(),
            (1150.0, 150.0).into(),
            screen,
        );
        assert_eq!(moved, r(800.0, 100.0, 200.0, 100.0));
    }

    #[test]
    fn an_edge_past_its_opposite_turns_over() {
        let screen = r(0.0, 0.0, 1000.0, 800.0);
        let p = r(100.0, 100.0, 200.0, 100.0);
        let grip = Grip::Edge {
            left: true,
            top: false,
            right: false,
            bottom: false,
        };
        let moved = dragged(
            grip,
            p,
            (100.0, 150.0).into(),
            (400.0, 150.0).into(),
            screen,
        );
        assert_eq!(moved, r(300.0, 100.0, 100.0, 100.0));
    }
}
