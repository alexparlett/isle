//! The look a session publishes for the greeter, and the greeter's reading of it
//! (`p14b-greeter.md` choices 10–12).

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use image::ImageEncoder;
use isle_settings::{
    Section,
    schema::{Fit, Settings},
};
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use tracing::{debug, info, warn};

use crate::{
    state::{Isle, State},
    wallpaper::decode,
};

/// A burst of changes is written once, this long after the last.
const SETTLE: Duration = Duration::from_millis(500);

/// The published picture is refused past this size.
const PICTURE_LIMIT: u64 = 16 << 20;

/// Each user's look, in a directory of their own that `isle setup` makes.
pub fn looks_dir() -> PathBuf {
    std::env::var_os("ISLE_GREETER_LOOKS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib/isle/greeter"))
}

/// The keys the greeter draws with; the glass position is the session's alone.
fn published(key: &str) -> bool {
    (key.starts_with("appearance.") && key != "appearance.glass")
        || key == "island.clock-24-hour"
        || [
            "keyboard.layouts",
            "keyboard.variants",
            "keyboard.options",
            "keyboard.numlock",
            "keyboard.translate",
            "shell.main-screen",
            "wallpaper.source",
            "wallpaper.dark-source",
            "wallpaper.fit",
            "wallpaper.colour",
        ]
        .contains(&key)
        || key.starts_with("keyboard-device[")
        || (key.starts_with("output[")
            && ["enabled", "mode", "scale", "x", "y", "transform"]
                .iter()
                .any(|leaf| key.ends_with(&format!("].{leaf}"))))
}

fn login() -> Option<String> {
    // SAFETY: getpwuid's entry is read before any other call can replace it.
    unsafe {
        let entry = libc::getpwuid(libc::getuid());
        (!entry.is_null()).then(|| {
            std::ffi::CStr::from_ptr((*entry).pw_name)
                .to_string_lossy()
                .into_owned()
        })
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, bytes)?;
    std::fs::rename(&temp, path)
}

/// The main screen's picture, fitted to it, as PNG.
fn picture_png(request: &decode::Request) -> Result<Vec<u8>, String> {
    let decoded = decode::load(request)?;
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            &decoded.rgba,
            decoded.width,
            decoded.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
    if png.len() as u64 > PICTURE_LIMIT {
        return Err("the picture is larger than the greeter takes".into());
    }
    Ok(png)
}

impl Isle {
    /// Something the look holds changed: it is written once things settle.
    pub fn publish_look_soon(&mut self) {
        if crate::greeter::on() {
            return;
        }
        if let Some(token) = self.greeter.look_timer.take() {
            self.loop_handle.remove(token);
        }
        self.greeter.look_timer = self
            .loop_handle
            .insert_source(Timer::from_duration(SETTLE), |_, _, state| {
                state.isle.greeter.look_timer = None;
                state.publish_look();
                TimeoutAction::Drop
            })
            .ok();
    }
}

impl State {
    fn publish_look(&mut self) {
        let Some(dir) = login().map(|l| looks_dir().join(l)) else {
            return;
        };
        if !dir.is_dir() {
            if !self.isle.greeter.look_unpublished {
                self.isle.greeter.look_unpublished = true;
                debug!(dir = %dir.display(), "no directory for the greeter's look; `isle setup` makes it");
            }
            return;
        }
        let Some(runtime) = self.isle.worker.clone() else {
            return;
        };
        let mut settings: Settings = self.isle.settings.clone();
        let main = self.isle.main_output();
        let identity = main.as_ref().map(Isle::output_identity);
        if let Some(own) = identity
            .as_ref()
            .and_then(|i| settings.wallpaper_output.get(i))
        {
            settings.wallpaper = own.clone();
        }
        let shown = identity.as_ref().and_then(|identity| {
            self.isle
                .wallpapers
                .status(&[(String::new(), identity.clone())])
                .pop()
                .and_then(|s| s.file)
        });
        let picture = dir.join("wallpaper.png");
        let request = match (shown, &main) {
            (Some(file), Some(output)) => {
                let pixels = output
                    .current_mode()
                    .map(|m| output.current_transform().transform_size(m.size))
                    .map(|s| (s.w.max(1) as u32, s.h.max(1) as u32))
                    .unwrap_or((1920, 1080));
                let request = decode::Request {
                    path: file,
                    size: pixels,
                    fit: settings.wallpaper.fit,
                    colour: settings.wallpaper.colour.as_deref().and_then(|c| {
                        let hex = c.strip_prefix('#').filter(|h| h.len() == 6)?;
                        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
                        Some([byte(0)?, byte(2)?, byte(4)?])
                    }),
                    folder: decode::Pick::Keep(None),
                    soften: decode::Soften {
                        sigma: 0.0,
                        saturation: 1.0,
                    },
                };
                settings.wallpaper.source = picture.to_string_lossy().into_owned();
                settings.wallpaper.dark_source = None;
                settings.wallpaper.fit = Fit::Fill;
                settings.wallpaper.colour = None;
                Some(request)
            }
            _ => None,
        };
        let text = isle_settings::text_of(&settings, published);
        runtime.spawn_blocking(move || {
            match &request {
                Some(request) => match picture_png(request) {
                    Ok(png) => {
                        if let Err(err) = write_atomic(&picture, &png) {
                            warn!("the greeter's wallpaper was not written: {err}");
                        }
                    }
                    Err(err) => warn!("the greeter's wallpaper was not made: {err}"),
                },
                None => {
                    let _ = std::fs::remove_file(&picture);
                }
            }
            match write_atomic(&dir.join("look.kdl"), text.as_bytes()) {
                Ok(()) => info!(dir = %dir.display(), "the greeter's look published"),
                Err(err) => warn!("the greeter's look was not written: {err}"),
            }
        });
    }

    /// The greeter takes `login`'s look, or the shipped one when they have published none.
    pub fn greeter_take_look(&mut self, login: Option<&str>) {
        let path = login.map(|l| looks_dir().join(l).join("look.kdl"));
        let store: isle_settings::Store<Settings> = match &path {
            Some(path) => isle_settings::Store::open(path),
            None => isle_settings::Store::open(PathBuf::new()),
        };
        if let Some(path) = &path
            && path.is_file()
            && let Some(err) = store.error()
        {
            warn!(path = %path.display(), "the published look is broken; the shipped one stands: {err}");
        }
        // Only the published keys are taken; anything else in the file is not the greeter's.
        let mut look = Settings::default();
        let mut values = std::collections::BTreeMap::new();
        store.current().flatten("", &mut values);
        for (key, value) in values.into_iter().filter(|(k, _)| published(k)) {
            if let Ok(path) = isle_settings::parse_path(&key) {
                let _ = look.set(&path, value);
            }
        }
        let mut old = std::collections::BTreeMap::new();
        let mut new = std::collections::BTreeMap::new();
        self.isle.settings.flatten("", &mut old);
        look.flatten("", &mut new);
        let changed: Vec<String> = old
            .keys()
            .chain(new.keys())
            .filter(|k| old.get(*k) != new.get(*k))
            .cloned()
            .collect();
        if changed.is_empty() {
            return;
        }
        self.apply_settings(look, Some(&changed));
        self.refresh_lock_face();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_what_the_greeter_draws_is_published() {
        for key in [
            "appearance.mode",
            "appearance.accent",
            "island.clock-24-hour",
            "keyboard.layouts",
            "keyboard-device[Keychron Q6 Max].profile",
            "output[AOC U34 1].scale",
            "shell.main-screen",
            "wallpaper.source",
        ] {
            assert!(published(key), "{key}");
        }
        for key in [
            "appearance.glass",
            "output[AOC U34 1].vrr",
            "keyboard.untranslated-apps",
            "wallpaper.change",
            "lock-screen.lock-after",
            "shortcuts.terminal",
        ] {
            assert!(!published(key), "{key}");
        }
    }
}
