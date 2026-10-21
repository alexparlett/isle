//! An item's picture: its kind's icon until its thumbnail arrives, which replaces it in place
//! with no fade (`p1/surfaces/files.md` § States, Many items).

use std::{cell::RefCell, collections::HashMap, path::PathBuf};

use bytes::Bytes;
use freya::prelude::*;
use isle_fs::Entry;
use isle_thumbnail::Flavour;

use crate::{list_view, thumbs};

/// Thumbnails held for drawing again, by item, size and modification time.
const BUDGET: usize = 48 * 1024 * 1024;

#[derive(Default)]
struct Held {
    by: HashMap<(PathBuf, Flavour, i64), (Bytes, u64)>,
    bytes: usize,
    clock: u64,
}

thread_local! {
    static HELD: RefCell<Held> = RefCell::default();
}

fn held(key: &(PathBuf, Flavour, i64)) -> Option<Bytes> {
    HELD.with_borrow_mut(|h| {
        h.clock += 1;
        let clock = h.clock;
        h.by.get_mut(key).map(|(b, used)| {
            *used = clock;
            b.clone()
        })
    })
}

fn hold(key: (PathBuf, Flavour, i64), bytes: Bytes) {
    HELD.with_borrow_mut(|h| {
        h.clock += 1;
        h.bytes += bytes.len();
        let clock = h.clock;
        if let Some((old, _)) = h.by.insert(key, (bytes, clock)) {
            h.bytes -= old.len();
        }
        // The longest unused go first.
        while h.bytes > BUDGET {
            let Some(oldest) = h.by.iter().min_by_key(|(_, (_, used))| *used).map(|(k, _)| k.clone()) else { break };
            if let Some((gone, _)) = h.by.remove(&oldest) {
                h.bytes -= gone.len();
            }
        }
    });
}

/// A PNG's size from its header, fitted inside `size` points, never cropped.
fn fitted(png: &[u8], size: f32) -> (f32, f32) {
    let dims = png
        .get(16..24)
        .map(|d| (u32::from_be_bytes(d[0..4].try_into().unwrap()), u32::from_be_bytes(d[4..8].try_into().unwrap())));
    match dims {
        Some((w, h)) if w > 0 && h > 0 => {
            let scale = size / w.max(h) as f32;
            (w as f32 * scale, h as f32 * scale)
        }
        _ => (size, size),
    }
}

#[derive(Clone, PartialEq)]
pub struct Thumbnail {
    pub entry: Entry,
    pub size: f32,
    pub dark: bool,
    /// The item is on a remote mount: its icon only.
    pub remote: bool,
}

impl Component for Thumbnail {
    fn render(&self) -> impl IntoElement {
        let t = isle_ui::theme().tokens.clone();
        let flavour = Flavour::for_pixels(self.size * 2.);
        let key = (self.entry.path.clone(), flavour, self.entry.modified);
        let wanted = !self.remote
            && !self.entry.is_dir()
            && self.entry.mime.as_deref().is_some_and(thumbs::worth_asking);
        let mut picture = use_state(|| held(&key));
        let ask = wanted.then(|| (key.clone(), self.entry.mime.as_deref().unwrap_or("").to_owned(), self.entry.size));
        use_side_effect_with_deps(&ask, move |ask| {
            let Some((key, mime, size)) = ask.clone() else { return };
            if let Some(bytes) = held(&key) {
                picture.set(Some(bytes));
                return;
            }
            picture.set(None);
            spawn(async move {
                let (path, flavour, mtime) = key.clone();
                if let Some(bytes) = thumbs::get(path, mime, flavour, mtime, size).await {
                    hold(key, bytes.clone());
                    picture.set(Some(bytes));
                }
            });
        });
        match picture.read().clone().filter(|_| wanted) {
            Some(bytes) => {
                let (w, h) = fitted(&bytes, self.size);
                rect()
                    .width(Size::px(self.size))
                    .height(Size::px(self.size))
                    .center()
                    .child(
                        ImageViewer::new(bytes)
                            .width(Size::px(w))
                            .height(Size::px(h))
                            .corner_radius(t.radius_small),
                    )
                    .into_element()
            }
            None => list_view::icon(&self.entry, self.dark, self.size, t.colour_text_2),
        }
    }
}
