//! What every view's items share beyond their tab: the name being edited, the items cut, and
//! a drag over the window with the folder it would drop in.

use std::path::{Path, PathBuf};

use freya::prelude::*;
use isle_ui::{TextField, colour};

/// The drop target the sidebar's Trash stands for; no folder has this name.
pub const TRASH: &str = "\0trash";

/// A name being edited in place (`p1/surfaces/files.md` § Rename in place).
#[derive(Clone, PartialEq)]
pub struct Rename {
    pub path: PathBuf,
    pub text: State<String>,
    pub on_commit: EventHandler<String>,
    pub on_cancel: EventHandler<()>,
    /// "“{name}” is already taken. Choose a different name."
    pub error: Option<String>,
    /// A folder: its whole name is selected, not all but the extension.
    pub dir: bool,
}

#[derive(Clone, PartialEq)]
pub struct Interact {
    pub rename: Option<Rename>,
    pub cut: Vec<PathBuf>,
    /// A drag is over the window.
    pub dragging: bool,
    /// The folder a drop would go into, as the drag's pointer finds it.
    pub drop_target: State<Option<PathBuf>>,
    /// A button went down on this item: a drag may start from it.
    pub press: State<Option<(PathBuf, f64, f64)>>,
    /// In a panel: what can be chosen; the rest is dimmed.
    pub chooses: Option<Choosable>,
    /// How the view lays items out, as it last measured.
    pub layout: State<crate::keys::Layout>,
    /// Items a context menu acts on outside the selection, ringed while it is open.
    pub ringed: Vec<PathBuf>,
    /// A spring-load's blink: the drop ring is off for this beat.
    pub blink: bool,
    /// The share of the jobs running that is done, for items being written.
    pub writing: f32,
    /// The keyboard cursor's item: its leading edge and foot in the window, where a menu
    /// opened from the keyboard hangs.
    pub cursor_at: State<Option<(f32, f32)>>,
    /// A menu a view draws itself is open, a divider's: the window's keys stand aside.
    pub view_menu: State<bool>,
}

impl Interact {
    /// Notes where the cursor's item is laid out.
    pub fn cursor_sized(&self) -> impl Fn(Event<SizedEventData>) + 'static {
        let at = self.cursor_at;
        move |e: Event<SizedEventData>| {
            let mut at = at;
            at.set_if_modified(Some((e.area.min_x(), e.area.max_y())));
        }
    }
}

/// What a panel can choose.
#[derive(Clone, Debug, PartialEq)]
pub struct Choosable {
    pub filter: Option<crate::chooser::Filter>,
    pub folders_only: bool,
}

impl Choosable {
    pub fn allows(&self, entry: &isle_fs::Entry) -> bool {
        if self.folders_only {
            return entry.is_dir();
        }
        self.filter.as_ref().is_none_or(|f| f.passes(&entry.name, entry.mime.as_deref(), entry.is_dir()))
    }
}

impl Interact {
    pub fn is_cut(&self, path: &Path) -> bool {
        self.cut.iter().any(|p| p == path)
    }

    /// Drawn dimmed: hidden, cut, or not what the panel can choose.
    pub fn is_dimmed(&self, entry: &isle_fs::Entry) -> bool {
        entry.hidden() || entry.writing || self.is_cut(&entry.path) || self.chooses.as_ref().is_some_and(|c| !c.allows(entry))
    }

    pub fn renaming(&self, path: &Path) -> Option<&Rename> {
        self.rename.as_ref().filter(|r| r.path == path)
    }

    pub fn is_target(&self, path: &Path) -> bool {
        self.dragging && !self.blink && self.drop_target.peek().as_deref() == Some(path)
    }

    /// Takes the drop for a folder while the drag is over it.
    pub fn target_on_enter(&self, path: &Path, folder: bool) -> impl Fn(Event<PointerEventData>) + 'static {
        let (target, dragging, path) = (self.drop_target, self.dragging, path.to_owned());
        move |_| {
            if dragging && folder {
                let mut target = target;
                target.set_if_modified(Some(path.clone()));
            }
        }
    }

    pub fn target_on_leave(&self, path: &Path) -> impl Fn(Event<PointerEventData>) + 'static {
        let (target, path) = (self.drop_target, path.to_owned());
        move |_| {
            if target.peek().as_deref() == Some(path.as_path()) {
                let mut target = target;
                target.set(None);
            }
        }
    }

    /// Notes where the button went down, for a drag to start from.
    pub fn press(&self, path: &Path, x: f64, y: f64) {
        let mut press = self.press;
        press.set(Some((path.to_owned(), x, y)));
    }
}

/// The chars of a name a rename selects: all of a folder's, a file's but its extension.
pub fn name_part(name: &str, dir: bool) -> std::ops::Range<usize> {
    let all = name.chars().count();
    match name.rfind('.') {
        Some(dot) if dot > 0 && !dir => 0..name[..dot].chars().count(),
        _ => 0..all,
    }
}

/// A refused name's reason, as a callout.
pub fn refusal(theme: &isle_tokens::Theme, why: String) -> Element {
    let t = &theme.tokens;
    rect()
        .padding((3., t.space_s))
        .corner_radius(t.radius_small)
        .background(colour(t.colour_window_raised))
        .child(isle_ui::text::line(theme, isle_ui::text::Style::Callout, colour(t.colour_text), why))
        .into_element()
}

/// The name field over an item's name, with its refusal under it when there is one.
pub fn rename_field(theme: &isle_tokens::Theme, rename: &Rename, width: Size) -> Element {
    let t = &theme.tokens;
    rect()
        .width(width.clone())
        .child(
            TextField::new(rename.text, "Name")
                .width(width)
                .editing_part(name_part(&rename.text.peek(), rename.dir))
                .on_submit(rename.on_commit.clone())
                .on_escape(rename.on_cancel.clone()),
        )
        .maybe_child(rename.error.clone().map(|error| {
            rect()
                .position(Position::new_absolute().top(t.size_control_large + t.space_xs))
                .layer(Layer::Overlay)
                .padding((3., t.space_s))
                .corner_radius(t.radius_small)
                .background(colour(t.colour_window_raised))
                .child(isle_ui::text::line(theme, isle_ui::text::Style::Callout, colour(t.colour_text), error))
        }))
        .into_element()
}

/// The ring round a folder that takes the drop.
pub fn drop_ring(theme: &isle_tokens::Theme) -> Border {
    let t = &theme.tokens;
    Border::new()
        .fill(colour(t.colour_accent))
        .width(t.stroke_found)
        .alignment(BorderAlignment::Inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rename_selects_the_name_but_not_its_extension() {
        assert_eq!(name_part("café notes.md", false), 0..10);
        assert_eq!(name_part("archive.tar.gz", false), 0..11);
        assert_eq!(name_part(".bashrc", false), 0..7);
        assert_eq!(name_part("my.folder", true), 0..9);
        assert_eq!(name_part("README", false), 0..6);
    }
}
