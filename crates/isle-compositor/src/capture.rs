//! Pictures of the screen: a screen, part of one or a window rendered apart from what the
//! capture tool draws, and written to a file or the clipboard.

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use smithay::{
    backend::renderer::gles::GlesTexture,
    output::Output,
    utils::{Logical, Physical, Rectangle, Size},
};
use tracing::warn;

use crate::{backend::Pixels, clipboard::Pick, desktop_notes::Note, state::State};

/// What a still holds.
#[derive(Clone, Debug)]
pub enum Target {
    /// An output whole.
    Screen(Output),
    /// Part of an output, in points from its top left.
    Region(Output, Rectangle<i32, Logical>),
    /// A window alone on a transparent ground, its shadow round it when `shadow`.
    Window { id: u64, shadow: bool },
}

/// Where stills go: a folder, or the clipboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Folder(PathBuf),
    Clipboard,
}

impl State {
    /// `target` rendered and read back, the pointer in it when `pointer`.
    pub fn capture_pixels(&mut self, target: &Target, pointer: bool) -> Result<Pixels, String> {
        match target {
            Target::Screen(output) => {
                let size = output.current_mode().ok_or("the output has no mode")?.size;
                let mut texture = self.backend.capture_screen(&self.isle, output, pointer)?;
                self.backend
                    .read_back(&mut texture, Rectangle::from_size(size), false)
            }
            Target::Region(output, region) => {
                let size = output.current_mode().ok_or("the output has no mode")?.size;
                let scale = output.current_scale().fractional_scale();
                let rect = region
                    .to_f64()
                    .to_physical(scale)
                    .to_i32_round()
                    .intersection(Rectangle::from_size(size))
                    .ok_or("the region is off the screen")?;
                let mut texture = self.backend.capture_screen(&self.isle, output, pointer)?;
                self.backend.read_back(&mut texture, rect, false)
            }
            Target::Window { id, shadow } => {
                let (mut texture, size) = self.window_texture(*id, *shadow)?;
                self.backend
                    .read_back(&mut texture, Rectangle::from_size(size), true)
            }
        }
    }

    /// Window `id` drawn alone into a texture of its own at its output's scale: its
    /// surfaces, popups and title bar, with its shadow when `shadow`.
    pub(crate) fn window_texture(
        &mut self,
        id: u64,
        shadow: bool,
    ) -> Result<(GlesTexture, Size<i32, Physical>), String> {
        if self.isle.lock.locked {
            return Err("the screen is locked".into());
        }
        if !self.isle.wm.is_shown(id) {
            return Err("the window is not on screen".into());
        }
        let frame = self.isle.frame(id).ok_or("no such window")?;
        let scale = self
            .isle
            .output_of(id)
            .map_or(1.0, |o| o.current_scale().fractional_scale());
        // The shadow reaches past the frame; the picture holds all of it.
        let mut bounds = Rectangle::<i32, Logical>::from_size(frame.size);
        if shadow {
            let style = self.isle.window_style(id);
            for layer in &style.shadows {
                let shape = crate::wm::draw::ShadowShape {
                    size: frame.size,
                    radius: style.corners[2],
                    layer: *layer,
                };
                bounds = bounds.merge(shape.area());
            }
        }
        let origin = frame.loc + bounds.loc;
        let size = bounds.size.to_f64().to_physical(scale).to_i32_round();
        let renderer = self.backend.renderer_opt().ok_or("no renderer")?;
        let mut pieces = Vec::new();
        self.isle
            .window_pieces(renderer, id, origin, scale, shadow, 1.0, &mut |p| {
                pieces.push(p)
            });
        let texture = crate::wm::motion::render_to_texture(renderer, size, scale, &pieces)?;
        Ok((texture, size))
    }

    /// Where stills go by the `capture` settings.
    pub fn still_destination(&self) -> Destination {
        destination(&self.isle.settings.capture.save_to, Kind::Still)
    }

    /// Writes `stills` as PNG files into a folder, or puts the first on the clipboard,
    /// encoding on the worker. `done` has each file written and, with `preview`, the first
    /// still made small for its thumbnail.
    pub fn keep_stills(
        &mut self,
        stills: Vec<Pixels>,
        to: Destination,
        preview: bool,
        done: impl FnOnce(&mut State, Vec<PathBuf>, Option<Pixels>) + Send + 'static,
    ) {
        let (Some(worker), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone())
        else {
            return;
        };
        let now = jiff::Zoned::now();
        worker.spawn_blocking(move || {
            let folder = match to {
                Destination::Clipboard => {
                    let png = stills.first().map(encode_png);
                    to_loop.post(move |state| {
                        match png {
                            Some(Ok(png)) => state.copy_still(png),
                            Some(Err(err)) => warn!("could not copy the screenshot: {err}"),
                            None => {}
                        }
                        done(state, Vec::new(), None);
                    });
                    return;
                }
                Destination::Folder(folder) => folder,
            };
            let mut written = Vec::new();
            let mut failed = None;
            for (n, still) in stills.iter().enumerate() {
                match write_still(&folder, &now, n, still) {
                    Ok(path) => written.push(path),
                    Err(err) => {
                        failed = Some((n, err));
                        break;
                    }
                }
            }
            let small = preview.then(|| stills.first().map(thumbnail_of)).flatten();
            let copied = failed.as_ref().map(|(n, _)| encode_png(&stills[*n]));
            drop(stills);
            trim();
            match (failed, copied) {
                (Some((_, err)), Some(png)) => {
                    warn!(folder = %folder.display(), "could not save a screenshot: {err}");
                    to_loop.post(move |state| {
                        if let Ok(png) = png {
                            state.copy_still_quietly(png);
                        }
                        state.still_not_saved(&folder, &err);
                        done(state, written, None);
                    });
                }
                _ => to_loop.post(move |state| done(state, written, small)),
            }
        });
    }

    /// Makes `png` the clipboard and says so in the island.
    pub fn copy_still(&mut self, png: Vec<u8>) {
        self.copy_still_quietly(png);
        use isle_shell::island::model::{Event, EventKind, Form};
        self.isle.island.model.post(
            Event {
                kind: EventKind::Copied,
                form: Form::Line {
                    symbol: "camera".into(),
                    fill: false,
                    text: "Screenshot copied".into(),
                },
                output: None,
                app: None,
                urgent: false,
            },
            std::time::Instant::now(),
        );
        self.isle.refresh_island();
    }

    fn copy_still_quietly(&mut self, png: Vec<u8>) {
        let pick = Pick {
            types: vec![("image/png".to_owned(), Arc::new(png))],
        };
        self.keep_copy(&pick, String::new());
        self.set_clipboard(pick);
    }

    fn still_not_saved(&mut self, folder: &Path, err: &std::io::Error) {
        let name = folder.file_name().map_or_else(
            || folder.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let summary = if err.raw_os_error() == Some(libc::ENOSPC) {
            format!("Couldn't save the screenshot. “{name}” is full. It was copied instead.")
        } else {
            format!("Couldn't save the screenshot to “{name}”. It was copied instead.")
        };
        self.post_desktop(
            "",
            "Screenshot",
            summary,
            String::new(),
            Vec::new(),
            Note::Capture,
        );
    }
}

impl State {
    /// Every screen, the main one first, written to the files or with `copy` put on the
    /// clipboard.
    pub fn shoot_screens(&mut self, copy: bool) {
        let main = self.isle.main_output();
        let mut outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
        outputs.sort_by_key(|o| Some(o) != main.as_ref());
        let pointer = self.isle.settings.capture.show_pointer;
        let stills: Vec<Pixels> = outputs
            .iter()
            .filter_map(
                |o| match self.capture_pixels(&Target::Screen(o.clone()), pointer) {
                    Ok(pixels) => Some(pixels),
                    Err(err) => {
                        warn!(output = %o.name(), "no screenshot: {err}");
                        None
                    }
                },
            )
            .collect();
        if stills.is_empty() {
            return;
        }
        self.shutter();
        let to = if copy {
            Destination::Clipboard
        } else {
            self.still_destination()
        };
        let quiet = self.game_in_front();
        let preview = !quiet && self.isle.settings.capture.floating_thumbnail;
        let shown_on = main.or_else(|| outputs.first().cloned());
        self.keep_stills(stills, to, preview, move |state, written, small| {
            if quiet && !written.is_empty() {
                state.island_shot_saved();
            }
            if let (Some(path), Some(small), Some(output)) = (written.first(), small, shown_on) {
                state.show_thumbnail(path.clone(), &output, small);
            }
        });
    }

    /// The shot's sound, when shots make one.
    pub(crate) fn shutter(&mut self) {
        if !self.isle.settings.capture.sound {
            return;
        }
        if let Some(file) = isle_audio::sounds::lookup("screen-capture", "freedesktop") {
            self.isle.run_service(
                async move { isle_audio::sounds::play(&file).await },
                |_, result| {
                    if let Err(err) = result {
                        tracing::debug!("the shutter sound: {err}");
                    }
                },
            );
        }
    }

    /// Whether a full-screen game is in front, over which a shot shows nothing of its own.
    pub fn game_in_front(&self) -> bool {
        self.focused_window().is_some_and(|id| {
            self.isle.wm.get(id).is_some_and(|r| r.game)
                && self
                    .isle
                    .output_of(id)
                    .is_some_and(|o| self.isle.output_full_screen(&o))
        })
    }

    /// "Screenshot saved" in the island, in an OSD's place over a full-screen window.
    fn island_shot_saved(&mut self) {
        use isle_shell::island::model::{Event, EventKind, Form};
        self.isle.island.model.post(
            Event {
                kind: EventKind::Osd,
                form: Form::Line {
                    symbol: "camera".into(),
                    fill: false,
                    text: "Screenshot saved".into(),
                },
                output: None,
                app: None,
                urgent: false,
            },
            std::time::Instant::now(),
        );
        self.isle.refresh_island();
    }
}

#[cfg(feature = "test-hooks")]
impl State {
    /// Takes the still a JSON spec names and writes it to the spec's folder: `{"target":
    /// "screen"|"region"|"window", "output", "x", "y", "w", "h", "id", "shadow",
    /// "pointer", "to"}`. The receiver has the files written.
    pub fn test_capture(
        &mut self,
        spec: &str,
    ) -> Result<tokio::sync::oneshot::Receiver<Vec<PathBuf>>, String> {
        let spec: serde_json::Value = serde_json::from_str(spec).map_err(|e| e.to_string())?;
        let output = |state: &State| {
            let name = spec["output"].as_str().unwrap_or_default();
            state
                .isle
                .space
                .outputs()
                .find(|o| o.name() == name)
                .cloned()
                .ok_or_else(|| format!("no output {name}"))
        };
        let int = |key: &str| spec[key].as_i64().unwrap_or_default() as i32;
        let target = match spec["target"].as_str() {
            Some("screen") => Target::Screen(output(self)?),
            Some("region") => Target::Region(
                output(self)?,
                Rectangle::new((int("x"), int("y")).into(), (int("w"), int("h")).into()),
            ),
            Some("window") => Target::Window {
                id: spec["id"].as_u64().unwrap_or_default(),
                shadow: spec["shadow"].as_bool().unwrap_or(true),
            },
            other => return Err(format!("no target {other:?}")),
        };
        for output in self.isle.space.outputs().cloned().collect::<Vec<_>>() {
            self.draw_shell(&output);
        }
        let pixels = self.capture_pixels(&target, spec["pointer"].as_bool().unwrap_or(false))?;
        let to = match spec["to"].as_str() {
            Some("clipboard") => Destination::Clipboard,
            Some(folder) => Destination::Folder(folder.into()),
            None => self.still_destination(),
        };
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.keep_stills(vec![pixels], to, false, move |_, written, _| {
            let _ = tx.send(written);
        });
        Ok(rx)
    }
}

impl State {
    /// A shot for an app through the portal: written where the app may read it, never kept
    /// in Screenshots, with no thumbnail. `reply` has the file.
    pub fn take_for_portal(
        &mut self,
        target: Target,
        reply: tokio::sync::oneshot::Sender<Option<PathBuf>>,
    ) {
        for output in self.isle.space.outputs().cloned().collect::<Vec<_>>() {
            self.draw_shell(&output);
        }
        let pointer = self.isle.settings.capture.show_pointer;
        match self.capture_pixels(&target, pointer) {
            Ok(pixels) => self.portal_still(pixels, reply),
            Err(err) => {
                warn!("no shot for the portal: {err}");
                let _ = reply.send(None);
            }
        }
    }

    /// A non-interactive Screenshot, already allowed by the frontend: the front window
    /// for the active window's target, else every screen as laid out.
    pub fn shoot_for_portal(
        &mut self,
        target: u32,
        reply: tokio::sync::oneshot::Sender<Option<PathBuf>>,
    ) {
        if self.isle.lock.locked {
            return self.when_unlocked(move |state| {
                if !reply.is_closed() {
                    state.shoot_for_portal(target, reply);
                }
            });
        }
        let front = self.focused_window();
        if matches!(
            target,
            isle_portal::screenshot::TARGET_ACTIVE_WINDOW | isle_portal::screenshot::TARGET_WINDOW
        ) && let Some(id) = front
        {
            return self.take_for_portal(Target::Window { id, shadow: true }, reply);
        }
        for output in self.isle.space.outputs().cloned().collect::<Vec<_>>() {
            self.draw_shell(&output);
        }
        let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
        let mut parts = Vec::new();
        for output in outputs {
            let Some(geometry) = self.isle.space.output_geometry(&output) else {
                continue;
            };
            let scale = output.current_scale().fractional_scale();
            if let Ok(pixels) = self.capture_pixels(&Target::Screen(output), false) {
                let at = geometry
                    .loc
                    .to_f64()
                    .to_physical(scale)
                    .to_i32_round::<i32>();
                parts.push((at, pixels));
            }
        }
        match compose(&parts) {
            Some(pixels) => self.portal_still(pixels, reply),
            None => {
                let _ = reply.send(None);
            }
        }
    }

    fn portal_still(
        &mut self,
        pixels: Pixels,
        reply: tokio::sync::oneshot::Sender<Option<PathBuf>>,
    ) {
        let folder = portal_folder();
        prune(&folder);
        self.keep_stills(
            vec![pixels],
            Destination::Folder(folder),
            false,
            move |_, written, _| {
                let _ = reply.send(written.into_iter().next());
            },
        );
    }
}

/// Gives the allocator's free pages back: a still's pixels are tens of megabytes, which the
/// allocator otherwise keeps for the thread that freed them.
pub fn trim() {
    // SAFETY: malloc_trim only releases free memory.
    unsafe {
        libc::malloc_trim(0);
    }
}

/// Where shots for apps are written: kept a day, for the document portal to hand on.
pub fn portal_folder() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".cache"))
        .join("isle/portal")
}

/// Removes the portal's shots older than a day.
fn prune(folder: &Path) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let day = std::time::Duration::from_secs(24 * 3600);
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > day);
        if old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Screens' pixels placed at their places in one picture, the gaps transparent.
fn compose(parts: &[(smithay::utils::Point<i32, Physical>, Pixels)]) -> Option<Pixels> {
    let left = parts.iter().map(|(at, _)| at.x).min()?;
    let top = parts.iter().map(|(at, _)| at.y).min()?;
    let right = parts.iter().map(|(at, p)| at.x + p.width as i32).max()?;
    let bottom = parts.iter().map(|(at, p)| at.y + p.height as i32).max()?;
    let (w, h) = ((right - left) as u32, (bottom - top) as u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for (at, p) in parts {
        let (x0, y0) = ((at.x - left) as usize, (at.y - top) as usize);
        let row = p.width as usize * 4;
        for y in 0..p.height as usize {
            let from = y * row;
            let to = ((y0 + y) * w as usize + x0) * 4;
            rgba[to..to + row].copy_from_slice(&p.rgba[from..from + row]);
        }
    }
    Some(Pixels {
        width: w,
        height: h,
        rgba,
    })
}

/// What a file holds, for its folder and name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Still,
    Recording,
}

/// Where `save_to` puts files of `kind`.
pub fn destination(save_to: &str, kind: Kind) -> Destination {
    match save_to {
        "clipboard" if kind == Kind::Still => Destination::Clipboard,
        "documents" => Destination::Folder(user_dir("DOCUMENTS", "Documents")),
        "" | "default" | "clipboard" => Destination::Folder(match kind {
            Kind::Still => user_dir("PICTURES", "Pictures").join("Screenshots"),
            Kind::Recording => user_dir("VIDEOS", "Videos").join("Recordings"),
        }),
        path => Destination::Folder(match path.strip_prefix("~/") {
            Some(rest) => home().join(rest),
            None => PathBuf::from(path),
        }),
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

/// The user's `XDG_<NAME>_DIR` from `user-dirs.dirs`, else `~/<fallback>`.
pub fn user_dir(name: &str, fallback: &str) -> PathBuf {
    let home = home();
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let key = format!("XDG_{name}_DIR=");
    std::fs::read_to_string(config.join("user-dirs.dirs"))
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                let value = line.trim().strip_prefix(&key)?.trim_matches('"');
                Some(match value.strip_prefix("$HOME") {
                    Some(rest) => home.join(rest.trim_start_matches('/')),
                    None => PathBuf::from(value),
                })
            })
        })
        .filter(|dir| dir != &home)
        .unwrap_or_else(|| home.join(fallback))
}

/// `Screenshot 2026-10-09 at 14.02.11`, the `n`th of a set ending ` (n+1)`.
pub fn file_stem(kind: Kind, at: &jiff::Zoned, n: usize) -> String {
    let what = match kind {
        Kind::Still => "Screenshot",
        Kind::Recording => "Screen Recording",
    };
    let base = format!(
        "{what} {} at {}",
        at.strftime("%Y-%m-%d"),
        at.strftime("%H.%M.%S")
    );
    if n == 0 {
        base
    } else {
        format!("{base} ({})", n + 1)
    }
}

/// Writes `still` into `folder` under the `n`th name of the set taken `at`, or the next
/// free one.
fn write_still(
    folder: &Path,
    at: &jiff::Zoned,
    n: usize,
    still: &Pixels,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(folder)?;
    let png = encode_png(still)?;
    let mut n = n;
    loop {
        let path = folder.join(format!("{}.png", file_stem(Kind::Still, at, n)));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                if let Err(err) = file.write_all(&png).and_then(|_| file.sync_data()) {
                    let _ = std::fs::remove_file(&path);
                    return Err(err);
                }
                return Ok(path);
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
            Err(err) => return Err(err),
        }
    }
}

/// `still` made small enough for its thumbnail at twice its size, premultiplied as the
/// shell draws images.
pub(crate) fn thumbnail_of(still: &Pixels) -> Pixels {
    const W: u32 = 480;
    const H: u32 = 320;
    let f = (W as f64 / still.width.max(1) as f64)
        .min(H as f64 / still.height.max(1) as f64)
        .min(1.0);
    let (w, h) = (
        ((still.width as f64 * f).round() as u32).max(1),
        ((still.height as f64 * f).round() as u32).max(1),
    );
    let mut rgba = image::RgbaImage::from_raw(still.width, still.height, still.rgba.clone())
        .map(|image| {
            image::imageops::resize(&image, w, h, image::imageops::FilterType::Triangle).into_raw()
        })
        .unwrap_or_else(|| vec![0; (w * h * 4) as usize]);
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3] as u32;
        for c in &mut px[..3] {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
    Pixels {
        width: w,
        height: h,
        rgba,
    }
}

/// `pixels` as a PNG, compressed for speed.
pub fn encode_png(pixels: &Pixels) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::with_capacity(pixels.rgba.len() / 8);
    let mut encoder = png::Encoder::new(&mut out, pixels.width, pixels.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(&pixels.rgba))
        .map_err(std::io::Error::other)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_macos() {
        let at: jiff::Zoned = "2026-10-03T14:02:11[Europe/London]".parse().unwrap();
        assert_eq!(
            file_stem(Kind::Still, &at, 0),
            "Screenshot 2026-10-03 at 14.02.11"
        );
        assert_eq!(
            file_stem(Kind::Still, &at, 1),
            "Screenshot 2026-10-03 at 14.02.11 (2)"
        );
        assert_eq!(
            file_stem(Kind::Recording, &at, 0),
            "Screen Recording 2026-10-03 at 14.02.11"
        );
    }

    #[test]
    fn a_taken_name_moves_on() {
        let dir = std::env::temp_dir().join(format!("isle-capture-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let at = jiff::Zoned::now();
        let still = Pixels {
            width: 2,
            height: 1,
            rgba: vec![255; 8],
        };
        let first = write_still(&dir, &at, 0, &still).unwrap();
        let second = write_still(&dir, &at, 0, &still).unwrap();
        assert_ne!(first, second);
        assert!(second.to_string_lossy().ends_with(" (2).png"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clipboard_is_for_stills() {
        assert_eq!(
            destination("clipboard", Kind::Still),
            Destination::Clipboard
        );
        assert!(matches!(
            destination("clipboard", Kind::Recording),
            Destination::Folder(_)
        ));
    }
}
