//! The frozen pick of Shift+Cmd+4: every screen held as it was at the chord, each shown
//! window held whole, a region dragged or a window chosen over them (`p1/structure/capture.md`,
//! The pick). The compositor draws the frozen frames, the veil, the cut and the line; the
//! shell draws only the size readout.

use std::{cell::RefCell, collections::HashMap};

use smithay::{
    backend::{
        input::{ButtonState, InputTime},
        renderer::{
            element::{
                Kind,
                solid::{SolidColorBuffer, SolidColorRenderElement},
                texture::{TextureBuffer, TextureRenderElement},
            },
            gles::{GlesRenderer, GlesTexture},
        },
    },
    input::pointer::{CursorIcon, CursorImageStatus},
    output::Output,
    utils::{Logical, Physical, Point, Rectangle, Size, Transform},
};
use tracing::warn;

use crate::{
    backend::Pixels,
    capture::Destination,
    render::{IsleElement, NamedCursor},
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

/// What the pick is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// A shot, to the files or with `copy` the clipboard.
    Shot { copy: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Region,
    Window,
}

/// One screen as it was at the chord.
pub struct FrozenScreen {
    pub output: Output,
    texture: GlesTexture,
    buffer: TextureBuffer<GlesTexture>,
}

/// One shown window as it was at the chord, alone, with and without its shadow.
pub struct FrozenWindow {
    pub id: u64,
    /// Its frame, title bar included, in the global space.
    pub frame: Rectangle<i32, Logical>,
    scale: f64,
    bare: (GlesTexture, TextureBuffer<GlesTexture>),
    shadowed: (GlesTexture, Size<i32, Physical>),
}

/// A region being dragged, in the global space, on the screen it began on.
#[derive(Clone, Copy, Debug)]
struct Drag {
    output: usize,
    start: Point<f64, Logical>,
    at: Point<f64, Logical>,
    /// Where the pointer was when Shift went down: one axis keeps it.
    shift_from: Option<Point<f64, Logical>>,
}

pub struct Freeze {
    pub purpose: Purpose,
    pub mode: Mode,
    pub screens: Vec<FrozenScreen>,
    /// Front to back.
    pub windows: Vec<FrozenWindow>,
    drag: Option<Drag>,
    /// The first corner Return set from the keyboard.
    anchor: Option<Point<f64, Logical>>,
    /// The window under the pointer or stepped to with Tab, by its place in `windows`.
    pub lifted: Option<usize>,
    space_held: bool,
    /// Each output's solids: the veil's four sides and the line's eight edges.
    solids: RefCell<HashMap<(String, usize), SolidColorBuffer>>,
}

impl Freeze {
    /// The selection now, on its screen, with Option growing it from its centre.
    fn selection(&self, option: bool) -> Option<(usize, Rectangle<f64, Logical>)> {
        let drag = self.drag?;
        let mut at = drag.at;
        if let Some(from) = drag.shift_from {
            let (dx, dy) = ((at.x - from.x).abs(), (at.y - from.y).abs());
            if dx > dy {
                at.y = from.y;
            } else {
                at.x = from.x;
            }
        }
        let start = if option {
            drag.start - (at - drag.start)
        } else {
            drag.start
        };
        let (x0, x1) = (start.x.min(at.x), start.x.max(at.x));
        let (y0, y1) = (start.y.min(at.y), start.y.max(at.y));
        Some((
            drag.output,
            Rectangle::new((x0, y0).into(), (x1 - x0, y1 - y0).into()),
        ))
    }
}

impl Isle {
    /// The pick's elements on `output` front to back, in place of the scene; false when no
    /// pick is open there.
    pub fn freeze_elements(
        &self,
        renderer: &mut GlesRenderer,
        output: &Output,
        cursor: Option<NamedCursor<'_>>,
        push: &mut dyn FnMut(IsleElement),
    ) -> bool {
        let Some(freeze) = &self.freeze else {
            return false;
        };
        let Some((index, screen)) = freeze
            .screens
            .iter()
            .enumerate()
            .find(|(_, s)| &s.output == output)
        else {
            return false;
        };
        let Some(geometry) = self.space.output_geometry(output) else {
            return false;
        };
        let scale = output.current_scale().fractional_scale();
        let t = &self.shell.theme().tokens;
        let pointer = self.seat.get_pointer().map(|p| p.current_location());

        if let (Some(cursor), Some(at)) =
            (cursor, pointer.filter(|p| geometry.to_f64().contains(*p)))
        {
            let location = at - geometry.loc.to_f64() - cursor.hotspot.to_f64();
            if let Ok(element) =
                smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    location.to_physical(scale),
                    cursor.buffer,
                    None,
                    None,
                    None,
                    Kind::Cursor,
                )
            {
                push(IsleElement::Memory(element));
            }
        }
        self.shell.elements_of(output, "capture-readout", &mut |e| {
            push(IsleElement::Shell(e))
        });

        let mut solids = freeze.solids.borrow_mut();
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
                SolidColorRenderElement::from_buffer(buffer, at, scale, 1.0, Kind::Unspecified)
            };
        let line = t.size_capture_line as f64;
        let mut outline =
            |first: usize, r: Rectangle<f64, Logical>, push: &mut dyn FnMut(IsleElement)| {
                // The light line just outside the shape, its dark edge outside that.
                for (n, (width, colour)) in [
                    (line, t.colour_capture_line),
                    (2.0 * line, t.colour_capture_line_shadow),
                ]
                .into_iter()
                .enumerate()
                {
                    let o = Rectangle::new(
                        (r.loc.x - width, r.loc.y - width).into(),
                        (r.size.w + 2.0 * width, r.size.h + 2.0 * width).into(),
                    );
                    let sides = [
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
                    ];
                    for (i, side) in sides.into_iter().enumerate() {
                        push(IsleElement::Solid(solid(first + n * 4 + i, side, colour)));
                    }
                }
            };

        let screen_rect = geometry.to_f64();
        let option = self.keyboards.engine.mods().option;
        let selection = freeze
            .selection(option)
            .filter(|(o, _)| *o == index)
            .map(|(_, r)| r);
        let lifted = (freeze.mode == Mode::Window)
            .then_some(freeze.lifted)
            .flatten()
            .and_then(|i| freeze.windows.get(i));

        let mut veil: Vec<Rectangle<f64, Logical>> = Vec::new();
        match (freeze.mode, selection) {
            (Mode::Region, Some(cut)) => {
                outline(4, cut, push);
                let (l, t_, r, b) = (
                    screen_rect.loc.x,
                    screen_rect.loc.y,
                    screen_rect.loc.x + screen_rect.size.w,
                    screen_rect.loc.y + screen_rect.size.h,
                );
                let (cl, ct, cr, cb) = (
                    cut.loc.x,
                    cut.loc.y,
                    cut.loc.x + cut.size.w,
                    cut.loc.y + cut.size.h,
                );
                veil.push(Rectangle::new((l, t_).into(), (r - l, ct - t_).into()));
                veil.push(Rectangle::new((l, cb).into(), (r - l, b - cb).into()));
                veil.push(Rectangle::new((l, ct).into(), (cl - l, cb - ct).into()));
                veil.push(Rectangle::new((cr, ct).into(), (r - cr, cb - ct).into()));
            }
            _ => veil.push(screen_rect),
        }
        if let Some(window) = lifted
            && window.frame.overlaps(geometry)
        {
            outline(4, window.frame.to_f64(), push);
            let at = (window.frame.loc - geometry.loc)
                .to_f64()
                .to_physical(window.scale);
            push(IsleElement::Texture(
                TextureRenderElement::from_texture_buffer(
                    at,
                    &window.bare.1,
                    None,
                    None,
                    Some(window.frame.size),
                    Kind::Unspecified,
                ),
            ));
        }
        for (i, rect) in veil.into_iter().enumerate() {
            push(IsleElement::Solid(solid(i, rect, t.colour_veil)));
        }
        push(IsleElement::Texture(
            TextureRenderElement::from_texture_buffer(
                Point::<f64, Physical>::from((0.0, 0.0)),
                &screen.buffer,
                None,
                None,
                Some(geometry.size),
                Kind::Unspecified,
            ),
        ));
        true
    }
}

impl State {
    /// Shift+Cmd+4: freezes every screen and opens the pick.
    pub fn open_pick(&mut self, purpose: Purpose) {
        if self.isle.freeze.is_some() || self.isle.lock.locked {
            return;
        }
        let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
        let mut screens = Vec::new();
        for output in &outputs {
            self.draw_shell(output);
            match self.backend.capture_screen(&self.isle, output, false) {
                Ok(texture) => {
                    let Some(renderer) = self.backend.renderer_opt() else {
                        return;
                    };
                    let buffer = TextureBuffer::from_texture(
                        renderer,
                        texture.clone(),
                        1,
                        Transform::Normal,
                        None,
                    );
                    screens.push(FrozenScreen {
                        output: output.clone(),
                        texture,
                        buffer,
                    });
                }
                Err(err) => warn!(output = %output.name(), "no frozen frame: {err}"),
            }
        }
        if screens.is_empty() {
            return;
        }
        let ids: Vec<u64> = self
            .isle
            .space
            .elements()
            .rev()
            .map(crate::wm::window_id)
            .filter(|id| self.isle.wm.is_shown(*id))
            .collect();
        let mut windows = Vec::new();
        for id in ids {
            let Some(frame) = self.isle.frame(id) else {
                continue;
            };
            let scale = self
                .isle
                .output_of(id)
                .map_or(1.0, |o| o.current_scale().fractional_scale());
            let (Ok((bare, _)), Ok(shadowed)) = (
                self.window_texture(id, false),
                self.window_texture(id, true),
            ) else {
                continue;
            };
            let Some(renderer) = self.backend.renderer_opt() else {
                return;
            };
            let buffer =
                TextureBuffer::from_texture(renderer, bare.clone(), 1, Transform::Normal, None);
            windows.push(FrozenWindow {
                id,
                frame,
                scale,
                bare: (bare, buffer),
                shadowed,
            });
        }
        self.isle.freeze = Some(Freeze {
            purpose,
            mode: Mode::Region,
            screens,
            windows,
            drag: None,
            anchor: None,
            lifted: None,
            space_held: false,
            solids: RefCell::default(),
        });
        // The pointer leaves whatever it was over; nothing below sees it until the pick ends.
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        self.pick_pointer_to(at);
        self.set_pick_cursor();
        self.isle.queue_redraw_all();
    }

    pub fn picking(&self) -> bool {
        self.isle.freeze.is_some()
    }

    /// Closes the pick and gives the screens back.
    pub fn close_pick(&mut self) {
        if self.isle.freeze.take().is_none() {
            return;
        }
        self.isle.shell.close("capture-readout");
        self.isle.cursor_status = CursorImageStatus::default_named();
        self.backend.update_cursor(&self.isle);
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        self.on_pointer_to(at, InputTime::now());
        self.isle.queue_redraw_all();
    }

    fn set_pick_cursor(&mut self) {
        let icon = match self.isle.freeze.as_ref().map(|f| f.mode) {
            Some(Mode::Window) => CursorIcon::Default,
            _ => CursorIcon::Crosshair,
        };
        self.isle.cursor_status = CursorImageStatus::Named(icon);
        self.backend.update_cursor(&self.isle);
    }

    /// The pointer moved while the pick is open: the drag follows, or the window under it
    /// lifts.
    pub fn pick_pointer_to(&mut self, at: Point<f64, Logical>) {
        if self.isle.freeze.is_none() {
            return;
        }
        let pointer = self.isle.seat.get_pointer().unwrap();
        pointer.motion(
            self,
            None,
            &smithay::input::pointer::MotionEvent {
                location: at,
                serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                time: InputTime::now(),
            },
        );
        pointer.frame(self);
        let Some(freeze) = self.isle.freeze.as_mut() else {
            return;
        };
        if let Some(drag) = freeze.drag.as_mut() {
            let bounds = freeze.screens[drag.output].output.clone();
            let screen = self
                .isle
                .space
                .output_geometry(&bounds)
                .unwrap_or_default()
                .to_f64();
            let clamped = Point::from((
                at.x.clamp(screen.loc.x, screen.loc.x + screen.size.w),
                at.y.clamp(screen.loc.y, screen.loc.y + screen.size.h),
            ));
            if freeze.space_held {
                let delta = clamped - drag.at;
                drag.start += delta;
            }
            drag.at = clamped;
        } else if freeze.mode == Mode::Window {
            freeze.lifted = freeze
                .windows
                .iter()
                .position(|w| w.frame.to_f64().contains(at));
        }
        self.place_readout();
        self.isle.queue_redraw_all();
    }

    pub fn pick_button(&mut self, button: u32, state: ButtonState) {
        let Some(freeze) = self.isle.freeze.as_mut() else {
            return;
        };
        if button != BTN_LEFT {
            return;
        }
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        match (freeze.mode, state) {
            (Mode::Region, ButtonState::Pressed) => {
                let Some(output) = freeze.screens.iter().position(|s| {
                    self.isle
                        .space
                        .output_geometry(&s.output)
                        .is_some_and(|g| g.to_f64().contains(at))
                }) else {
                    return;
                };
                freeze.drag = Some(Drag {
                    output,
                    start: at,
                    at,
                    shift_from: None,
                });
                freeze.anchor = None;
            }
            (Mode::Region, ButtonState::Released) => {
                let option = self.held_mods().option;
                let Some(freeze) = self.isle.freeze.as_mut() else {
                    return;
                };
                let picked = freeze.selection(option);
                freeze.drag = None;
                match picked {
                    Some((output, rect)) if rect.size.w >= 1.0 && rect.size.h >= 1.0 => {
                        self.take_region(output, rect)
                    }
                    _ => {
                        self.isle.shell.close("capture-readout");
                        self.isle.queue_redraw_all();
                    }
                }
            }
            (Mode::Window, ButtonState::Pressed) => {
                if let Some(index) = freeze.lifted {
                    let shadow = !self.held_mods().option;
                    self.take_window(index, shadow);
                }
            }
            (Mode::Window, ButtonState::Released) => {}
        }
    }

    /// A key while the pick is open; every key is the pick's.
    pub fn pick_key(&mut self, code: u32, pressed: bool) -> bool {
        let Some(freeze) = self.isle.freeze.as_mut() else {
            return false;
        };
        if !pressed {
            if code == KEY_SPACE {
                freeze.space_held = false;
            }
            self.pick_modifiers_changed();
            return true;
        }
        let mods = self.held_mods();
        let Some(freeze) = self.isle.freeze.as_mut() else {
            return true;
        };
        if freeze.drag.is_some() && freeze.anchor.is_none() {
            match code {
                KEY_SPACE => freeze.space_held = true,
                KEY_ESC => self.close_pick(),
                _ => self.pick_modifiers_changed(),
            }
            return true;
        }
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        let step = if mods.shift { 10.0 } else { 1.0 };
        match code {
            KEY_ESC => self.close_pick(),
            KEY_SPACE => {
                freeze.mode = match freeze.mode {
                    Mode::Region => Mode::Window,
                    Mode::Window => Mode::Region,
                };
                freeze.anchor = None;
                self.set_pick_cursor();
                self.pick_pointer_to(at);
            }
            KEY_UP | KEY_DOWN | KEY_LEFT | KEY_RIGHT => {
                let (dx, dy) = match code {
                    KEY_UP => (0.0, -step),
                    KEY_DOWN => (0.0, step),
                    KEY_LEFT => (-step, 0.0),
                    _ => (step, 0.0),
                };
                let to = self.isle.clamp_to_outputs(at + Point::from((dx, dy)));
                self.pick_pointer_to(to);
                self.keyboard_selection_follows();
            }
            KEY_TAB if freeze.mode == Mode::Window && !freeze.windows.is_empty() => {
                let n = freeze.windows.len();
                freeze.lifted = Some(match (freeze.lifted, mods.shift) {
                    (None, false) => 0,
                    (None, true) => n - 1,
                    (Some(i), false) => (i + 1) % n,
                    (Some(i), true) => (i + n - 1) % n,
                });
                self.isle.queue_redraw_all();
            }
            KEY_ENTER | KEY_KPENTER => match freeze.mode {
                Mode::Window => {
                    if let Some(index) = freeze.lifted {
                        self.take_window(index, !mods.option);
                    }
                }
                Mode::Region => match freeze.anchor.take() {
                    None => {
                        freeze.anchor = Some(at);
                        freeze.drag = None;
                        self.keyboard_selection_follows();
                    }
                    Some(anchor) => {
                        let rect = Rectangle::new(
                            (anchor.x.min(at.x), anchor.y.min(at.y)).into(),
                            ((anchor.x - at.x).abs(), (anchor.y - at.y).abs()).into(),
                        );
                        let output = freeze.screens.iter().position(|s| {
                            self.isle
                                .space
                                .output_geometry(&s.output)
                                .is_some_and(|g| g.to_f64().contains(anchor))
                        });
                        match output {
                            Some(output) if rect.size.w >= 1.0 && rect.size.h >= 1.0 => {
                                self.take_region(output, rect)
                            }
                            _ => {}
                        }
                    }
                },
            },
            _ => {}
        }
        true
    }

    /// Shift pressed or released while dragging locks or frees an axis.
    pub fn pick_modifiers_changed(&mut self) {
        let shift = self.held_mods().shift;
        let Some(drag) = self.isle.freeze.as_mut().and_then(|f| f.drag.as_mut()) else {
            return;
        };
        drag.shift_from = match (shift, drag.shift_from) {
            (true, None) => Some(drag.at),
            (true, kept) => kept,
            (false, _) => None,
        };
        self.place_readout();
        self.isle.queue_redraw_all();
    }

    /// With a corner set from the keyboard, the selection runs from it to the crosshair.
    fn keyboard_selection_follows(&mut self) {
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        let Some(freeze) = self.isle.freeze.as_mut() else {
            return;
        };
        let Some(anchor) = freeze.anchor else {
            return;
        };
        let output = freeze
            .screens
            .iter()
            .position(|s| {
                self.isle
                    .space
                    .output_geometry(&s.output)
                    .is_some_and(|g| g.to_f64().contains(anchor))
            })
            .unwrap_or(0);
        // The keyboard's selection is drawn as a drag that has not been let go.
        freeze.drag = Some(Drag {
            output,
            start: anchor,
            at,
            shift_from: None,
        });
        freeze.anchor = Some(anchor);
        self.place_readout();
        self.isle.queue_redraw_all();
    }

    /// The size readout by the selection's trailing foot, or closed with no selection.
    fn place_readout(&mut self) {
        let option = self.held_mods().option;
        let Some(freeze) = &self.isle.freeze else {
            return;
        };
        let Some((index, rect)) = freeze.selection(option) else {
            self.isle.shell.close("capture-readout");
            return;
        };
        let output = freeze.screens[index].output.clone();
        let Some(screen) = self.isle.space.output_geometry(&output) else {
            return;
        };
        let scale = output.current_scale().fractional_scale();
        let px = rect.size.to_physical(scale).to_i32_round::<i32>();
        let text = format!("{} × {}", px.w, px.h);
        let theme = self.isle.shell.theme().clone();
        let size = isle_shell::capture::readout::ReadoutSurface::size_for(&theme);
        let margin = isle_shell::capture::readout::ReadoutSurface::margin(&theme) as i32;
        let gap = theme.tokens.space_s as i32;
        let right = (rect.loc.x + rect.size.w).round() as i32 - screen.loc.x + margin;
        let below = (rect.loc.y + rect.size.h).round() as i32 - screen.loc.y + gap - margin;
        let mut y = below;
        if y + size.h > screen.size.h {
            // Inside the selection's foot, then above it.
            y = below - gap * 2 - (size.h - 2 * margin);
            if y < 0 {
                y = rect.loc.y.round() as i32 - screen.loc.y - gap - size.h + margin;
            }
        }
        let x = (right - size.w).max(-margin);
        let geometry = Rectangle::new((x, y.max(-margin)).into(), size);
        let surface = self.isle.capture_readout.clone();
        surface.set_model(text.clone());
        self.isle
            .shell
            .with_root("capture-readout", || surface.set(text.clone()));
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale,
        };
        self.isle
            .shell
            .open_placed("capture-readout", &place, geometry);
        self.dispatch_shell();
    }

    fn take_region(&mut self, output: usize, rect: Rectangle<f64, Logical>) {
        let Some(freeze) = self.isle.freeze.as_mut() else {
            return;
        };
        let purpose = freeze.purpose;
        let screen = &mut freeze.screens[output];
        let out = screen.output.clone();
        let mut texture = screen.texture.clone();
        let Some(geometry) = self.isle.space.output_geometry(&out) else {
            return;
        };
        let scale = out.current_scale().fractional_scale();
        let local = Rectangle::new(rect.loc - geometry.loc.to_f64(), rect.size);
        let size = out.current_mode().map(|m| m.size).unwrap_or_default();
        let px = local
            .to_physical(scale)
            .to_i32_round()
            .intersection(Rectangle::from_size(size));
        let pixels = px.map(|px| self.backend.read_back(&mut texture, px, false));
        self.close_pick();
        match pixels {
            Some(Ok(pixels)) => self.keep_shot(pixels, purpose),
            Some(Err(err)) => warn!("no region: {err}"),
            None => {}
        }
    }

    fn take_window(&mut self, index: usize, shadow: bool) {
        let Some(freeze) = self.isle.freeze.as_mut() else {
            return;
        };
        let purpose = freeze.purpose;
        let window = &freeze.windows[index];
        let (mut texture, size) = if shadow {
            window.shadowed.clone()
        } else {
            let size = window
                .frame
                .size
                .to_f64()
                .to_physical(window.scale)
                .to_i32_round();
            (window.bare.0.clone(), size)
        };
        let pixels = self
            .backend
            .read_back(&mut texture, Rectangle::from_size(size), true);
        self.close_pick();
        match pixels {
            Ok(pixels) => self.keep_shot(pixels, purpose),
            Err(err) => warn!("no window: {err}"),
        }
    }

    fn keep_shot(&mut self, pixels: Pixels, purpose: Purpose) {
        let Purpose::Shot { copy } = purpose;
        let to = if copy {
            Destination::Clipboard
        } else {
            self.still_destination()
        };
        self.shutter();
        self.keep_stills(vec![pixels], to, |_, _| {});
    }

    /// The pick as JSON, for the Test interface.
    #[cfg(feature = "test-hooks")]
    pub fn pick_status(&self) -> String {
        let option = self.held_mods().option;
        let Some(freeze) = &self.isle.freeze else {
            return "null".into();
        };
        serde_json::json!({
            "mode": match freeze.mode { Mode::Region => "region", Mode::Window => "window" },
            "screens": freeze.screens.len(),
            "windows": freeze.windows.iter().map(|w| w.id).collect::<Vec<_>>(),
            "lifted": freeze.lifted.map(|i| freeze.windows[i].id),
            "selection": freeze.selection(option).map(|(_, r)| [r.loc.x, r.loc.y, r.size.w, r.size.h]),
            "readout": self.isle.shell.is_open("capture-readout"),
        })
        .to_string()
    }
}
