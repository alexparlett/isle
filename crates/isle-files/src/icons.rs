//! Each type's icon file, found on the listing's thread and read from the cache by the
//! window, which never searches the icon theme itself.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use isle_apps::icons::Icons;

/// The icon theme's sizes Files draws from: list rows at 16 points, larger views at 64.
pub const SMALL: u32 = 16;
pub const LARGE: u32 = 64;

struct Cache {
    themes: [Icons; 2],
    found: HashMap<(String, bool, u32), Option<PathBuf>>,
}

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(Cache {
            themes: [Icons::files(false), Icons::files(true)],
            found: HashMap::new(),
        })
    })
}

/// Finds and keeps the icon for `mime` at `size` in both appearances; blocks on the icon
/// theme, so call it where the listing runs.
pub fn warm(mime: &str, size: u32) {
    let mut cache = cache().lock().unwrap();
    for dark in [false, true] {
        let key = (mime.to_owned(), dark, size);
        if cache.found.contains_key(&key) {
            continue;
        }
        let names = isle_apps::mime::types().icons(mime);
        let theme = &mut cache.themes[usize::from(dark)];
        let found = names.iter().find_map(|n| theme.lookup(n, size));
        cache.found.insert(key, found);
    }
}

/// The icon found for `mime`, if it has been looked for and there is one.
pub fn cached(mime: &str, dark: bool, size: u32) -> Option<PathBuf> {
    cache()
        .lock()
        .unwrap()
        .found
        .get(&(mime.to_owned(), dark, size))
        .cloned()
        .flatten()
}

#[cfg(test)]
mod tests {
    #[test]
    fn papirus_has_folders_and_documents() {
        super::warm("inode/directory", super::SMALL);
        super::warm("text/markdown", super::SMALL);
        if !std::path::Path::new("/usr/share/icons/Papirus").exists() {
            return;
        }
        let folder = super::cached("inode/directory", true, super::SMALL).unwrap();
        assert!(folder.ends_with("places/folder.svg"), "{folder:?}");
        let markdown = super::cached("text/markdown", false, super::SMALL).unwrap();
        assert!(markdown.to_string_lossy().contains("/Papirus/16x16/mimetypes/"), "{markdown:?}");
    }
}
