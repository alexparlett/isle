//! Adding to the recently-used list, `recently-used.xbel`, as GTK's file chooser does.

use std::path::{Path, PathBuf};

/// `$XDG_DATA_HOME/recently-used.xbel`.
pub fn xbel() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .map(|d| d.join("recently-used.xbel"))
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// A `file://` URI with the bytes a URI cannot hold escaped.
pub fn uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut out = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~!$'()*+,;=:@".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `text` with the bookmark for `path` added or brought to now, used by `app`.
pub fn with_item(text: &str, path: &Path, mime: &str, app: &str, exec: &str, now: &str) -> String {
    let href = escape(&uri(path));
    let base = if text.contains("</xbel>") {
        text.to_owned()
    } else {
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<xbel version=\"1.0\"\n      xmlns:bookmark=\"http://www.freedesktop.org/standards/desktop-bookmarks\"\n      xmlns:mime=\"http://www.freedesktop.org/standards/shared-mime-info\"\n>\n</xbel>\n".to_owned()
    };
    // An existing bookmark goes, to come back at the end with this use.
    let marker = format!("<bookmark href=\"{href}\"");
    let base = match base.find(&marker) {
        Some(start) => match base[start..].find("</bookmark>") {
            Some(len) => {
                let end = start + len + "</bookmark>".len();
                let end = if base[end..].starts_with('\n') { end + 1 } else { end };
                let start = base[..start].rfind('\n').map_or(start, |n| n + 1);
                format!("{}{}", &base[..start], &base[end..])
            }
            None => base,
        },
        None => base,
    };
    let entry = format!(
        "  <bookmark href=\"{href}\" added=\"{now}\" modified=\"{now}\" visited=\"{now}\">\n    <info>\n      <metadata owner=\"http://freedesktop.org\">\n        <mime:mime-type type=\"{}\"/>\n        <bookmark:applications>\n          <bookmark:application name=\"{}\" exec=\"{}\" modified=\"{now}\" count=\"1\"/>\n        </bookmark:applications>\n      </metadata>\n    </info>\n  </bookmark>\n",
        escape(mime),
        escape(app),
        escape(exec),
    );
    let at = base.rfind("</xbel>").unwrap_or(base.len());
    format!("{}{entry}{}", &base[..at], &base[at..])
}

/// Adds `path` to the list as used now by `app`.
pub fn add(path: &Path, mime: &str, app: &str, exec: &str) -> std::io::Result<()> {
    let file = xbel().ok_or_else(|| std::io::Error::other("no data folder"))?;
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let now = jiff::Timestamp::now().strftime("%Y-%m-%dT%H:%M:%S%.6fZ").to_string();
    let next = with_item(&text, path, mime, app, exec, &now);
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let part = file.with_extension("xbel.part");
    std::fs::write(&part, next)?;
    std::fs::rename(part, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_item_is_added_once_and_moves_to_the_end() {
        let one = with_item("", Path::new("/home/me/a b.txt"), "text/plain", "Editor", "editor %u", "2026-10-21T08:00:00Z");
        assert!(one.contains("href=\"file:///home/me/a%20b.txt\""));
        let two = with_item(&one, Path::new("/home/me/c.png"), "image/png", "Viewer", "viewer", "2026-10-21T09:00:00Z");
        let again = with_item(&two, Path::new("/home/me/a b.txt"), "text/plain", "Editor", "editor", "2026-10-21T10:00:00Z");
        assert_eq!(again.matches("a%20b.txt").count(), 1);
        assert!(again.find("c.png").unwrap() < again.find("a%20b.txt").unwrap());
        assert!(again.trim_end().ends_with("</xbel>"));
    }
}
