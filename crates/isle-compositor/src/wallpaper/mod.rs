//! The picture behind everything on each screen, drawn by the compositor as the bottom of
//! the stack (`p6-island-and-menus.md` choices 1 and 2).

mod backdrop;
pub mod decode;

use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use isle_settings::schema::{Change, Fit, Settings, Wallpaper as Choice};
use isle_state::{
    Loaded, StateDir,
    domains::{WallpaperScreen, Wallpapers as Saved},
};
use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            element::{
                Kind,
                solid::{SolidColorBuffer, SolidColorRenderElement},
                texture::{TextureBuffer, TextureRenderElement},
            },
            gles::{GlesPixelProgram, GlesRenderer, GlesTexture},
        },
    },
    output::Output,
    reexports::calloop::timer::{TimeoutAction, Timer},
    utils::{Logical, Rectangle, Scale, Size, Transform},
};
use tracing::{info, warn};

use self::backdrop::{BackdropElement, Look};
use crate::{state::Isle, state_writer::Saver};

smithay::backend::renderer::element::render_elements! {
    pub WallpaperElement<=GlesRenderer>;
    Backdrop = BackdropElement,
    Solid = SolidColorRenderElement,
    Picture = TextureRenderElement<GlesTexture>,
}

/// What a `source` setting names.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Frost,
    Colour([u8; 3]),
    /// A picture, or a folder of them; which is the worker's to find out.
    Path(PathBuf),
}

impl Source {
    pub fn parse(text: &str) -> Self {
        let text = text.trim();
        if text.is_empty() || text.eq_ignore_ascii_case("frost") {
            Source::Frost
        } else if let Some(colour) = parse_colour(text) {
            Source::Colour(colour)
        } else if let Some(rest) = text.strip_prefix("~/") {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default();
            Source::Path(home.join(rest))
        } else {
            Source::Path(PathBuf::from(text))
        }
    }
}

/// `#rrggbb`.
fn parse_colour(text: &str) -> Option<[u8; 3]> {
    let hex = text.strip_prefix('#').filter(|h| h.len() == 6)?;
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// What one screen should show: its choice, resolved for the appearance and its size.
#[derive(Clone, Debug, PartialEq)]
pub struct Wanted {
    /// The setting's text, which the saved state is kept against.
    pub text: String,
    pub source: Source,
    pub fit: Fit,
    pub colour: Option<[u8; 3]>,
    pub change: Change,
    pub shuffle: bool,
    pub look: Look,
    /// Device pixels, and logical points, of the output.
    pub pixels: (u32, u32),
    pub logical: Size<i32, Logical>,
}

impl Wanted {
    /// Whether `self` shows the same thing as `other`, only at another size.
    fn resized(&self, other: &Wanted) -> bool {
        Wanted {
            pixels: other.pixels,
            logical: other.logical,
            ..self.clone()
        } == *other
    }
}

enum Layer {
    Frost(BackdropElement),
    Solid(SolidColorBuffer),
    Picture(TextureBuffer<GlesTexture>),
}

/// One screen's wallpaper, by its output's identity.
#[derive(Default)]
struct Screen {
    wanted: Option<Wanted>,
    /// Drawn on top, fading in while `fade` runs.
    front: Option<Layer>,
    /// Drawn under `front` until its fade ends.
    behind: Option<Layer>,
    fade: Option<Instant>,
    /// The decode in flight.
    loading: Option<u64>,
    /// Decoded pixels waiting for the renderer, and whether they cut in.
    arrived: Option<(decode::Decoded, bool)>,
    /// The picture shown, for a folder's next pick and the status.
    file: Option<PathBuf>,
    /// Why the wanted picture is not shown.
    problem: Option<String>,
    /// A folder's change fell due; it waits until the screen is uncovered.
    due: bool,
    /// The folder has been picked from in this session, so `login` has moved on.
    picked: bool,
    /// What is shown stands in for a picture not yet decoded: its colour from last time, or
    /// the drawn backdrop.
    standing_in: bool,
    /// The picture shown, blurred and saturated at a quarter of its size, for the lock.
    softened: Option<TextureBuffer<GlesTexture>>,
}

#[derive(Default)]
pub struct Wallpapers {
    screens: HashMap<String, Screen>,
    program: Option<GlesPixelProgram>,
    saved: Saved,
    saver: Option<Saver<Saved>>,
    next_request: u64,
    /// Each screen's wash over its softened form, by the output's identity.
    washes: std::cell::RefCell<HashMap<String, SolidColorBuffer>>,
}

/// One screen's wallpaper as the bus reports it.
#[derive(Clone, Debug)]
pub struct Status {
    pub output: String,
    pub source: String,
    pub file: Option<PathBuf>,
    pub problem: Option<String>,
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl Wallpapers {
    pub fn load(&mut self, runtime: &tokio::runtime::Handle) {
        let dir = StateDir::new(StateDir::default_dir());
        let (saved, loaded) = dir.load::<Saved>();
        if !matches!(loaded, Loaded::Read | Loaded::Missing) {
            warn!(?loaded, "wallpapers");
        }
        self.saved = saved;
        self.saver = Some(Saver::spawn(runtime, dir));
    }

    /// Lets the textures of an output that went go; its choice and state stay.
    pub fn forget(&mut self, identity: &str) {
        self.screens.remove(identity);
    }

    pub fn fading(&self, identity: &str) -> bool {
        self.screens.get(identity).is_some_and(|s| s.fade.is_some())
    }

    pub fn status(&self, outputs: &[(String, String)]) -> Vec<Status> {
        outputs
            .iter()
            .map(|(name, identity)| {
                let screen = self.screens.get(identity);
                Status {
                    output: name.clone(),
                    source: screen
                        .and_then(|s| s.wanted.as_ref())
                        .map(|w| w.text.clone())
                        .unwrap_or_default(),
                    file: screen.and_then(|s| s.file.clone()),
                    problem: screen.and_then(|s| s.problem.clone()),
                }
            })
            .collect()
    }

    fn save(&mut self, identity: &str, f: impl FnOnce(&mut WallpaperScreen)) {
        f(self.saved.screens.entry(identity.to_owned()).or_default());
        if let Some(saver) = &self.saver {
            saver.save(self.saved.clone());
        }
    }
}

/// The choice in force for the output `identity`: its own, else the shared one.
fn choice<'a>(settings: &'a Settings, identity: &str) -> &'a Choice {
    settings
        .wallpaper_output
        .get(identity)
        .unwrap_or(&settings.wallpaper)
}

impl Isle {
    fn wanted_wallpaper(&self, output: &Output) -> Option<Wanted> {
        let logical = self.space.output_geometry(output)?.size;
        let mode = output.current_mode()?;
        let pixels = output.current_transform().transform_size(mode.size);
        let theme = self.shell.theme();
        let choice = choice(&self.settings, &Self::output_identity(output));
        let dark = theme.appearance.mode == isle_tokens::Mode::Dark;
        let text = match (&choice.dark_source, dark) {
            (Some(dark), true) if !dark.trim().is_empty() => dark.clone(),
            _ => choice.source.clone(),
        };
        Some(Wanted {
            source: Source::parse(&text),
            text,
            fit: choice.fit,
            colour: choice.colour.as_deref().and_then(parse_colour),
            change: choice.change,
            shuffle: choice.shuffle,
            look: Look::of(theme),
            pixels: (pixels.w.max(1) as u32, pixels.h.max(1) as u32),
            logical,
        })
    }

    /// The output shows a full-screen desktop, which hides the wallpaper entirely.
    pub fn wallpaper_covered(&self, output: &Output) -> bool {
        let identity = Self::output_identity(output);
        let full_screen = self.desktops.rows.get(&identity).is_some_and(|row| {
            row.desktops
                .get(row.current)
                .is_some_and(|d| matches!(d.kind, crate::wm::desktops::Kind::FullScreen { .. }))
        });
        full_screen && !self.windows_moving(output)
    }

    /// A change shows at once rather than crossfading: covered, a game in front, or every
    /// transition off.
    fn wallpaper_cuts(&self, output: &Output) -> bool {
        self.wallpaper_covered(output)
            || !self.shell.animated()
            || self
                .seat
                .get_keyboard()
                .and_then(|k| k.current_focus())
                .and_then(|f| {
                    f.surface()
                        .and_then(|s| self.wm.for_surface(s))
                        .map(|r| r.game)
                })
                .unwrap_or(false)
    }

    /// The wallpaper's elements for `output`, front to back.
    pub fn wallpaper_elements(&self, output: &Output, push: &mut dyn FnMut(WallpaperElement)) {
        if self.wallpaper_covered(output) {
            return;
        }
        let Some(screen) = self.wallpapers.screens.get(&Self::output_identity(output)) else {
            return;
        };
        let scale = Scale::from(output.current_scale().fractional_scale());
        let logical = screen
            .wanted
            .as_ref()
            .map(|w| w.logical)
            .unwrap_or_default();
        let fade = self.theme_fade();
        let front_alpha = screen.fade.map_or(1.0, |start| {
            (start.elapsed().as_secs_f32() / fade.as_secs_f32().max(1e-3)).min(1.0)
        });
        for (layer, alpha) in [(&screen.front, front_alpha), (&screen.behind, 1.0)] {
            let Some(layer) = layer else { continue };
            push(match layer {
                Layer::Frost(element) => WallpaperElement::Backdrop(element.with_alpha(alpha)),
                Layer::Solid(buffer) => {
                    WallpaperElement::Solid(SolidColorRenderElement::from_buffer(
                        buffer,
                        (0, 0),
                        scale,
                        alpha,
                        Kind::Unspecified,
                    ))
                }
                Layer::Picture(buffer) => {
                    WallpaperElement::Picture(TextureRenderElement::from_texture_buffer(
                        (0.0, 0.0),
                        buffer,
                        Some(alpha),
                        None,
                        Some(logical),
                        Kind::Unspecified,
                    ))
                }
            });
        }
    }

    /// The softened form of `output`'s wallpaper with its wash, front to back, for the lock
    /// (`surfaces/wallpaper.md`). A picture is drawn from its quarter-size blur; the drawn
    /// backdrop and a colour hold no shapes, so they are drawn as they are.
    pub fn softened_elements(&self, output: &Output, push: &mut dyn FnMut(WallpaperElement)) {
        let identity = Self::output_identity(output);
        let Some(screen) = self.wallpapers.screens.get(&identity) else {
            return;
        };
        let scale = Scale::from(output.current_scale().fractional_scale());
        let logical = screen
            .wanted
            .as_ref()
            .map(|w| w.logical)
            .unwrap_or_default();
        let c = self.shell.theme().tokens.material_softened;
        let mut washes = self.wallpapers.washes.borrow_mut();
        let wash = washes.entry(identity).or_default();
        wash.update(logical, premultiplied(c));
        push(WallpaperElement::Solid(SolidColorRenderElement::from_buffer(
            wash,
            (0, 0),
            scale,
            1.0,
            Kind::Unspecified,
        )));
        match (&screen.front, &screen.softened) {
            (Some(Layer::Picture(_)), Some(softened)) => {
                push(WallpaperElement::Picture(TextureRenderElement::from_texture_buffer(
                    (0.0, 0.0),
                    softened,
                    None,
                    None,
                    Some(logical),
                    Kind::Unspecified,
                )))
            }
            (Some(Layer::Picture(picture)), None) => {
                push(WallpaperElement::Picture(TextureRenderElement::from_texture_buffer(
                    (0.0, 0.0),
                    picture,
                    None,
                    None,
                    Some(logical),
                    Kind::Unspecified,
                )))
            }
            (Some(Layer::Frost(element)), _) => {
                push(WallpaperElement::Backdrop(element.with_alpha(1.0)))
            }
            (Some(Layer::Solid(buffer)), _) => {
                push(WallpaperElement::Solid(SolidColorRenderElement::from_buffer(
                    buffer,
                    (0, 0),
                    scale,
                    1.0,
                    Kind::Unspecified,
                )))
            }
            (None, _) => {}
        }
    }

    fn theme_fade(&self) -> Duration {
        isle_tokens::Theme::millis(self.shell.theme().tokens.motion_wallpaper_fade_ms)
    }

    /// Shows `layer` on the screen, over what was there, crossfading unless `cut`.
    fn show_wallpaper(&mut self, identity: &str, layer: Layer, cut: bool) {
        let screen = self
            .wallpapers
            .screens
            .entry(identity.to_owned())
            .or_default();
        if cut || screen.front.is_none() {
            screen.behind = None;
            screen.fade = None;
        } else {
            screen.behind = screen.front.take();
            screen.fade = Some(Instant::now());
        }
        if !matches!(layer, Layer::Picture(_)) {
            screen.softened = None;
        }
        screen.front = Some(layer);
        screen.standing_in = false;
    }

    /// Asks the worker for the picture `wanted` names, on the screen `identity`.
    fn request_picture(&mut self, output: &Output, wanted: &Wanted, next: bool, cut: bool) {
        let Source::Path(path) = &wanted.source else {
            return;
        };
        let identity = Self::output_identity(output);
        let screen = self.wallpapers.screens.entry(identity.clone()).or_default();
        let current = screen.file.clone().or_else(|| {
            self.wallpapers
                .saved
                .screens
                .get(&identity)
                .filter(|s| s.source == wanted.text)
                .and_then(|s| s.file.as_ref().map(PathBuf::from))
        });
        // A folder set to change at login moves on once a session.
        let next = next || (wanted.change == Change::Login && !screen.picked);
        screen.picked = true;
        screen.due = false;
        let request = decode::Request {
            path: path.clone(),
            size: wanted.pixels,
            fit: wanted.fit,
            colour: wanted.colour,
            folder: if next {
                decode::Pick::Next {
                    current,
                    shuffle: wanted.shuffle,
                }
            } else {
                decode::Pick::Keep(current)
            },
            soften: decode::Soften {
                sigma: self.shell.theme().tokens.material_softened_blur
                    * output.current_scale().fractional_scale() as f32,
                saturation: self.shell.theme().tokens.material_softened_saturation,
            },
        };
        self.wallpapers.next_request += 1;
        let id = self.wallpapers.next_request;
        screen.loading = Some(id);
        let (Some(runtime), Some(to_loop)) = (self.worker.clone(), self.to_loop.clone()) else {
            return;
        };
        let target = output.clone();
        runtime.spawn(async move {
            let decoded = tokio::task::spawn_blocking(move || decode::load(&request))
                .await
                .unwrap_or_else(|_| Err("the picture could not be read".into()));
            to_loop.post(move |state| {
                state.isle.picture_arrived(&target, id, decoded, next, cut);
            });
        });
    }

    fn picture_arrived(
        &mut self,
        output: &Output,
        id: u64,
        decoded: Result<decode::Decoded, String>,
        moved_on: bool,
        cut: bool,
    ) {
        let identity = Self::output_identity(output);
        let Some(screen) = self.wallpapers.screens.get_mut(&identity) else {
            return;
        };
        if screen.loading != Some(id) {
            return;
        }
        screen.loading = None;
        match decoded {
            Ok(decoded) => {
                screen.problem = None;
                let text = screen
                    .wanted
                    .as_ref()
                    .map(|w| w.text.clone())
                    .unwrap_or_default();
                let (file, mean) = (decoded.file.clone(), decoded.mean);
                screen.arrived = Some((decoded, cut));
                let now = unix_now();
                self.wallpapers.save(&identity, |saved| {
                    let fresh = saved.source != text;
                    saved.source = text;
                    saved.file = Some(file.to_string_lossy().into_owned());
                    saved.mean = Some(mean);
                    if moved_on || fresh || saved.changed_at == 0 {
                        saved.changed_at = now;
                    }
                });
                self.schedule_wallpaper_change(output);
            }
            Err(problem) => {
                warn!(output = output.name(), "wallpaper: {problem}");
                screen.problem = Some(problem);
                // The last good wallpaper stays; with none, the drawn backdrop.
                if screen.front.is_none() || screen.standing_in {
                    let look = screen.wanted.as_ref().map(|w| w.look);
                    let size = screen.wanted.as_ref().map(|w| w.logical);
                    if let (Some(look), Some(size), Some(program)) =
                        (look, size, self.wallpapers.program.clone())
                    {
                        let cut = self.wallpaper_cuts(output);
                        self.show_wallpaper(
                            &identity,
                            Layer::Frost(BackdropElement::new(program, size, look)),
                            cut,
                        );
                        if let Some(screen) = self.wallpapers.screens.get_mut(&identity) {
                            screen.standing_in = false;
                        }
                    }
                }
            }
        }
        self.queue_redraw(output);
    }

    /// Arms a timer for when a folder's next picture falls due.
    fn schedule_wallpaper_change(&mut self, output: &Output) {
        let identity = Self::output_identity(output);
        let Some(wanted) = self
            .wallpapers
            .screens
            .get(&identity)
            .and_then(|s| s.wanted.clone())
        else {
            return;
        };
        let period = match wanted.change {
            Change::Hourly => 3600,
            Change::Daily => 86_400,
            Change::Never | Change::Login => return,
        };
        let Some(saved) = self.wallpapers.saved.screens.get(&identity) else {
            return;
        };
        let at = saved.changed_at + period;
        let wait = Duration::from_secs(at.saturating_sub(unix_now()));
        let target = output.clone();
        let text = wanted.text;
        let _ = self
            .loop_handle
            .insert_source(Timer::from_duration(wait), move |_, _, state| {
                let identity = Isle::output_identity(&target);
                if let Some(screen) = state.isle.wallpapers.screens.get_mut(&identity)
                    && screen.wanted.as_ref().is_some_and(|w| w.text == text)
                {
                    screen.due = true;
                    state.isle.queue_redraw(&target);
                }
                TimeoutAction::Drop
            });
    }

    /// Brings `output`'s wallpaper up to date before its frame: a changed choice, a picture
    /// that arrived, a folder's change that fell due, the end of a crossfade.
    pub fn prepare_wallpaper(&mut self, renderer: &mut GlesRenderer, output: &Output) {
        if self.wallpapers.program.is_none() {
            match backdrop::compile(renderer) {
                Ok(program) => self.wallpapers.program = Some(program),
                Err(err) => warn!("no drawn wallpaper: {err}"),
            }
        }
        let Some(wanted) = self.wanted_wallpaper(output) else {
            return;
        };
        let identity = Self::output_identity(output);
        let cut = self.wallpaper_cuts(output);
        let screen = self.wallpapers.screens.entry(identity.clone()).or_default();

        if screen.wanted.as_ref() != Some(&wanted) {
            let resized = screen.wanted.as_ref().is_some_and(|w| w.resized(&wanted));
            let first = screen.front.is_none();
            screen.wanted = Some(wanted.clone());
            let cut = cut || resized;
            match &wanted.source {
                Source::Frost => {
                    screen.loading = None;
                    screen.file = None;
                    screen.problem = None;
                    if let Some(program) = self.wallpapers.program.clone() {
                        let element = BackdropElement::new(program, wanted.logical, wanted.look);
                        self.show_wallpaper(&identity, Layer::Frost(element), cut);
                    }
                }
                Source::Colour(colour) => {
                    screen.loading = None;
                    screen.file = None;
                    screen.problem = None;
                    let buffer = SolidColorBuffer::new(wanted.logical, rgba(*colour));
                    self.show_wallpaper(&identity, Layer::Solid(buffer), cut);
                }
                Source::Path(_) => {
                    if first {
                        // The session's first frame: the picture's colour from last time,
                        // else the drawn backdrop, until it is decoded.
                        let mean = self
                            .wallpapers
                            .saved
                            .screens
                            .get(&identity)
                            .filter(|s| s.source == wanted.text)
                            .and_then(|s| s.mean);
                        match (mean, self.wallpapers.program.clone()) {
                            (Some(mean), _) => self.show_wallpaper(
                                &identity,
                                Layer::Solid(SolidColorBuffer::new(wanted.logical, rgba(mean))),
                                true,
                            ),
                            (None, Some(program)) => self.show_wallpaper(
                                &identity,
                                Layer::Frost(BackdropElement::new(
                                    program,
                                    wanted.logical,
                                    wanted.look,
                                )),
                                true,
                            ),
                            (None, None) => {}
                        }
                        if let Some(screen) = self.wallpapers.screens.get_mut(&identity) {
                            screen.standing_in = true;
                        }
                    }
                    if resized && let Some(screen) = self.wallpapers.screens.get_mut(&identity) {
                        // Same picture, new size: keep the one shown, not the folder's next.
                        screen.picked = true;
                    }
                    self.request_picture(output, &wanted, false, cut);
                }
            }
        }

        let screen = self.wallpapers.screens.entry(identity.clone()).or_default();
        if screen.due && !self.wallpaper_covered(output) {
            self.request_picture(output, &wanted, true, cut);
        }

        let screen = self.wallpapers.screens.entry(identity.clone()).or_default();
        if let Some((decoded, arrived_cut)) = screen.arrived.take() {
            let picture = TextureBuffer::from_memory(
                renderer,
                &decoded.rgba,
                Fourcc::Abgr8888,
                (decoded.width as i32, decoded.height as i32),
                false,
                1,
                Transform::Normal,
                Some(vec![Rectangle::from_size(
                    (decoded.width as i32, decoded.height as i32).into(),
                )]),
            );
            let softened = TextureBuffer::from_memory(
                renderer,
                decoded.softened.as_raw(),
                Fourcc::Abgr8888,
                (decoded.softened.width() as i32, decoded.softened.height() as i32),
                false,
                1,
                Transform::Normal,
                None,
            );
            match picture {
                Ok(picture) => {
                    info!(output = output.name(), file = %decoded.file.display(), "wallpaper");
                    screen.file = Some(decoded.file);
                    self.show_wallpaper(&identity, Layer::Picture(picture), cut || arrived_cut);
                    if let Some(screen) = self.wallpapers.screens.get_mut(&identity) {
                        match softened {
                            Ok(softened) => screen.softened = Some(softened),
                            Err(err) => warn!(output = output.name(), "softened wallpaper: {err}"),
                        }
                    }
                }
                Err(err) => warn!(output = output.name(), "wallpaper upload failed: {err}"),
            }
        }

        let fade = self.theme_fade();
        let screen = self.wallpapers.screens.entry(identity).or_default();
        if screen.fade.is_some_and(|start| start.elapsed() >= fade) {
            screen.fade = None;
            screen.behind = None;
        }
    }
}

/// A token's colour as a solid buffer takes it, its alpha multiplied in.
pub fn premultiplied(c: isle_tokens::Colour) -> [f32; 4] {
    [
        c.r as f32 / 255.0 * c.a,
        c.g as f32 / 255.0 * c.a,
        c.b as f32 / 255.0 * c.a,
        c.a,
    ]
}

fn rgba([r, g, b]: [u8; 3]) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_is_frost_a_colour_or_a_path() {
        assert_eq!(Source::parse("frost"), Source::Frost);
        assert_eq!(Source::parse(""), Source::Frost);
        assert_eq!(Source::parse("#1a2b3c"), Source::Colour([0x1a, 0x2b, 0x3c]));
        assert_eq!(
            Source::parse("/usr/share/backgrounds/x.png"),
            Source::Path("/usr/share/backgrounds/x.png".into())
        );
        assert!(matches!(Source::parse("#12345"), Source::Path(_)));
    }
}
