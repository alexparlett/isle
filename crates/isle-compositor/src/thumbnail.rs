//! The thumbnail after a shot: shown for a few seconds in the bottom-trailing corner of the
//! screen shot, held while the pointer is over it; a click opens the file, a flick toward
//! the edge dismisses it, a drag carries the file out, and its menu moves, copies or trashes
//! it (`p1/structure/capture.md`, The thumbnail).

use std::{path::PathBuf, sync::Arc, time::Duration};

use smithay::{
    backend::{allocator::Fourcc, input::ButtonState, renderer::ImportMem},
    input::pointer::GrabStartData,
    output::Output,
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::{Logical, Point, Rectangle},
};
use tracing::{info, warn};

use crate::{
    backend::Pixels,
    capture::{Kind, destination},
    clipboard::Pick,
    launch::Launch,
    state::State,
};

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
/// How far a press moves before it is a flick or a drag.
const SLOP: f64 = 8.0;

const MENU_SAVE_SCREENSHOTS: u64 = 1;
const MENU_SAVE_DOCUMENTS: u64 = 2;
const MENU_SAVE_CLIPBOARD: u64 = 3;
const MENU_COPY: u64 = 4;
const MENU_TRASH: u64 = 5;
const MENU_CLOSE: u64 = 6;

pub struct Thumbnail {
    pub path: PathBuf,
    pub output: Output,
    /// The frame the user sees, in the global space; the pointer's place for it.
    pub rect: Rectangle<i32, Logical>,
    timer: Option<RegistrationToken>,
    press: Option<Point<f64, Logical>>,
    hovered: bool,
    pub menu_open: bool,
}

impl State {
    /// Shows the thumbnail of the shot at `path`, replacing any shown.
    pub fn show_thumbnail(&mut self, path: PathBuf, output: &Output, small: Pixels) {
        let Some(screen) = self.isle.space.output_geometry(output) else {
            return;
        };
        let Some(renderer) = self.backend.renderer_opt() else {
            return;
        };
        let texture = match renderer.import_memory(
            &small.rgba,
            Fourcc::Abgr8888,
            (small.width as i32, small.height as i32).into(),
            false,
        ) {
            Ok(texture) => texture,
            Err(err) => {
                warn!("no thumbnail: {err}");
                return;
            }
        };
        let Some(image) = self.isle.shell.image_of(renderer, &texture) else {
            return;
        };
        self.dismiss_thumbnail();
        let theme = self.isle.shell.theme().clone();
        let size = isle_shell::capture::thumbnail::ThumbnailSurface::size_for(
            &theme,
            small.width as f32,
            small.height as f32,
        );
        let margin = isle_shell::capture::thumbnail::ThumbnailSurface::margin(&theme) as i32;
        let inset = theme.tokens.space_l as i32;
        let work = self.isle.work_area(output);
        let frame = Rectangle::new(
            (
                work.loc.x + work.size.w - inset - (size.w - 2 * margin),
                work.loc.y + work.size.h - inset - (size.h - 2 * margin),
            )
                .into(),
            (size.w - 2 * margin, size.h - 2 * margin).into(),
        );
        let geometry = Rectangle::new(frame.loc - screen.loc - Point::from((margin, margin)), size);
        let surface = self.isle.capture_thumbnail.clone();
        surface.set_model(Some(image.clone()));
        self.isle
            .shell
            .with_root("capture-thumbnail", || surface.set(Some(image.clone())));
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        self.isle
            .shell
            .open_placed("capture-thumbnail", &place, geometry);
        self.isle.thumbnail = Some(Thumbnail {
            path,
            output: output.clone(),
            rect: frame,
            timer: None,
            press: None,
            hovered: false,
            menu_open: false,
        });
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        self.thumbnail_pointer(at);
        self.arm_thumbnail_timer();
        self.dispatch_shell();
    }

    pub fn dismiss_thumbnail(&mut self) {
        let Some(thumbnail) = self.isle.thumbnail.take() else {
            return;
        };
        if let Some(token) = thumbnail.timer {
            self.isle.loop_handle.remove(token);
        }
        if thumbnail.menu_open {
            self.close_surface("window-menu");
        }
        self.isle.shell.close("capture-thumbnail");
        self.isle.queue_redraw(&thumbnail.output);
    }

    /// The full time again, unless the pointer or the menu holds it.
    fn arm_thumbnail_timer(&mut self) {
        let Some(thumbnail) = self.isle.thumbnail.as_mut() else {
            return;
        };
        if let Some(token) = thumbnail.timer.take() {
            self.isle.loop_handle.remove(token);
        }
        if thumbnail.hovered || thumbnail.menu_open || thumbnail.press.is_some() {
            return;
        }
        let shown =
            Duration::from_millis(self.isle.shell.theme().tokens.timing_thumbnail_ms as u64);
        thumbnail.timer = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(shown), |_, _, state| {
                if let Some(thumbnail) = state.isle.thumbnail.as_mut() {
                    thumbnail.timer = None;
                }
                state.dismiss_thumbnail();
                TimeoutAction::Drop
            })
            .ok();
    }

    /// The pointer moved: over the thumbnail it holds it; a press that travels becomes a
    /// drag of the file, unless it heads for the screen's edge, which is a flick.
    pub fn thumbnail_pointer(&mut self, at: Point<f64, Logical>) {
        let Some(thumbnail) = self.isle.thumbnail.as_mut() else {
            return;
        };
        let over = thumbnail.rect.to_f64().contains(at);
        if over != thumbnail.hovered {
            thumbnail.hovered = over;
            self.arm_thumbnail_timer();
        }
        let Some(thumbnail) = self.isle.thumbnail.as_ref() else {
            return;
        };
        let Some(from) = thumbnail.press else {
            return;
        };
        let d = at - from;
        if d.x.abs().max(d.y.abs()) < SLOP || flicked(d) {
            return;
        }
        let path = thumbnail.path.clone();
        self.dismiss_thumbnail();
        self.drag_file_out(&path, from);
    }

    /// A button over the thumbnail, or the release of a press that began on it; true when
    /// the thumbnail took it.
    pub fn thumbnail_button(&mut self, button: u32, state: ButtonState) -> bool {
        let at = self.isle.seat.get_pointer().unwrap().current_location();
        let Some(thumbnail) = self.isle.thumbnail.as_mut() else {
            return false;
        };
        let over = thumbnail.rect.to_f64().contains(at);
        match (button, state) {
            (BTN_LEFT, ButtonState::Pressed) if over => {
                thumbnail.press = Some(at);
                true
            }
            (BTN_LEFT, ButtonState::Released) if thumbnail.press.is_some() => {
                let from = thumbnail.press.take().unwrap();
                let d = at - from;
                let path = thumbnail.path.clone();
                if d.x.abs().max(d.y.abs()) < SLOP {
                    self.dismiss_thumbnail();
                    self.launch(Launch::Open(path.to_string_lossy().into_owned()));
                } else if flicked(d) {
                    self.dismiss_thumbnail();
                } else {
                    self.arm_thumbnail_timer();
                }
                true
            }
            (BTN_RIGHT, ButtonState::Pressed) if over => {
                self.open_thumbnail_menu(at);
                true
            }
            (BTN_RIGHT, ButtonState::Released) if over => true,
            _ => false,
        }
    }

    /// Starts a drag of the file at `path` from `from`, as Spotlight drags its files out.
    fn drag_file_out(&mut self, path: &std::path::Path, from: Point<f64, Logical>) {
        let uri = format!("file://{}\r\n", path.display());
        let text = path.display().to_string();
        let pick = Pick {
            types: vec![
                ("text/uri-list".into(), Arc::new(uri.into_bytes())),
                (
                    "text/plain;charset=utf-8".into(),
                    Arc::new(text.clone().into_bytes()),
                ),
                ("text/plain".into(), Arc::new(text.into_bytes())),
            ],
        };
        let start = GrabStartData {
            focus: None,
            button: BTN_LEFT,
            location: from,
        };
        self.drag_out(pick, start);
        info!(path = %path.display(), "a shot dragged out");
    }

    fn open_thumbnail_menu(&mut self, at: Point<f64, Logical>) {
        use isle_ui::{MenuEntry, MenuItem};
        let Some(thumbnail) = self.isle.thumbnail.as_mut() else {
            return;
        };
        thumbnail.menu_open = true;
        let output = thumbnail.output.clone();
        let item = |id, label: &str| MenuEntry::Item(MenuItem::new(id, label));
        let save_to = MenuItem {
            submenu: Some(std::rc::Rc::new(vec![
                item(MENU_SAVE_SCREENSHOTS, "Screenshots"),
                item(MENU_SAVE_DOCUMENTS, "Documents"),
                item(MENU_SAVE_CLIPBOARD, "Clipboard"),
            ])),
            ..MenuItem::new(0, "Save to")
        };
        let entries = vec![
            MenuEntry::Item(save_to),
            item(MENU_COPY, "Copy"),
            MenuEntry::Separator,
            item(MENU_TRASH, "Move to Trash"),
            MenuEntry::Separator,
            item(MENU_CLOSE, "Close"),
        ];
        self.open_menu_at(entries, &output, at.to_i32_round());
        self.arm_thumbnail_timer();
    }

    /// The window menu's surface chose for the thumbnail; false when the menu is not its.
    pub fn thumbnail_menu_event(&mut self, event: &isle_ui::MenuEvent) -> bool {
        let Some(thumbnail) = self.isle.thumbnail.as_mut() else {
            return false;
        };
        if !thumbnail.menu_open {
            return false;
        }
        match event {
            isle_ui::MenuEvent::Chosen(id) => {
                thumbnail.menu_open = false;
                self.close_surface("window-menu");
                self.thumbnail_act(*id);
            }
            isle_ui::MenuEvent::Back => {
                thumbnail.menu_open = false;
                self.close_surface("window-menu");
                self.arm_thumbnail_timer();
            }
            _ => {}
        }
        true
    }

    fn thumbnail_act(&mut self, id: u64) {
        let Some(path) = self.isle.thumbnail.as_ref().map(|t| t.path.clone()) else {
            return;
        };
        match id {
            MENU_SAVE_SCREENSHOTS | MENU_SAVE_DOCUMENTS => {
                let folder = match id {
                    MENU_SAVE_DOCUMENTS => crate::capture::user_dir("DOCUMENTS", "Documents"),
                    _ => match destination("default", Kind::Still) {
                        crate::capture::Destination::Folder(folder) => folder,
                        crate::capture::Destination::Clipboard => return,
                    },
                };
                match move_into(&path, &folder) {
                    Ok(moved) => {
                        if let Some(thumbnail) = self.isle.thumbnail.as_mut() {
                            thumbnail.path = moved;
                        }
                    }
                    Err(err) => warn!(path = %path.display(), "could not move the shot: {err}"),
                }
                self.arm_thumbnail_timer();
            }
            MENU_SAVE_CLIPBOARD | MENU_COPY => {
                match std::fs::read(&path) {
                    Ok(png) => self.copy_still(png),
                    Err(err) => warn!(path = %path.display(), "could not copy the shot: {err}"),
                }
                if id == MENU_SAVE_CLIPBOARD {
                    let _ = std::fs::remove_file(&path);
                    self.dismiss_thumbnail();
                } else {
                    self.arm_thumbnail_timer();
                }
            }
            MENU_TRASH => {
                if let Err(err) = trash(&path) {
                    warn!(path = %path.display(), "could not trash the shot: {err}");
                }
                self.dismiss_thumbnail();
            }
            _ => self.dismiss_thumbnail(),
        }
    }

    /// The thumbnail as JSON, for the Test interface.
    #[cfg(feature = "test-hooks")]
    pub fn thumbnail_status(&self) -> String {
        let Some(t) = &self.isle.thumbnail else {
            return "null".into();
        };
        serde_json::json!({
            "path": t.path,
            "rect": [t.rect.loc.x, t.rect.loc.y, t.rect.size.w, t.rect.size.h],
            "output": t.output.name(),
            "hovered": t.hovered,
            "timing": t.timer.is_some(),
            "drawn": self.isle.shell.is_open("capture-thumbnail"),
        })
        .to_string()
    }

    /// Acts as the thumbnail's menu item `id` would, for the Test interface.
    #[cfg(feature = "test-hooks")]
    pub fn thumbnail_test_act(&mut self, item: &str) -> Result<(), String> {
        let id = match item {
            "save-screenshots" => MENU_SAVE_SCREENSHOTS,
            "save-documents" => MENU_SAVE_DOCUMENTS,
            "save-clipboard" => MENU_SAVE_CLIPBOARD,
            "copy" => MENU_COPY,
            "trash" => MENU_TRASH,
            "close" => MENU_CLOSE,
            other => return Err(format!("no item {other}")),
        };
        self.thumbnail_act(id);
        Ok(())
    }
}

/// A release travelling toward the screen's trailing or bottom edge, faster across than
/// back: the flick that sends the thumbnail away.
fn flicked(d: Point<f64, Logical>) -> bool {
    (d.x >= SLOP && d.x >= d.y.abs()) || (d.y >= SLOP && d.y >= d.x.abs())
}

/// Moves `path` into `folder` under its own name, or the next free one.
fn move_into(path: &std::path::Path, folder: &std::path::Path) -> std::io::Result<PathBuf> {
    if path.parent() == Some(folder) {
        return Ok(path.to_owned());
    }
    std::fs::create_dir_all(folder)?;
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("{stem}{ext}")
        } else {
            format!("{stem} ({n}){ext}")
        };
        let to = folder.join(name);
        if !to.exists() {
            if std::fs::rename(path, &to).is_err() {
                std::fs::copy(path, &to)?;
                std::fs::remove_file(path)?;
            }
            return Ok(to);
        }
        n += 1;
    }
}

/// Moves `path` to its volume's trash, as Files does.
pub fn trash(path: &std::path::Path) -> std::io::Result<()> {
    isle_fs::trash::trash(path).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flicks_head_for_the_trailing_or_bottom_edge() {
        assert!(flicked((40.0, 5.0).into()));
        assert!(flicked((3.0, 40.0).into()));
        assert!(!flicked((-40.0, 5.0).into()));
        assert!(!flicked((10.0, -40.0).into()));
    }
}
