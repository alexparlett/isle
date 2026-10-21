//! Thumbnails in the shared freedesktop cache (`files-2026.md` §3): where each lives, whether
//! it is still good, and the helper's requests and answers. The helper process makes them;
//! Files and any reader only look here.

pub mod cache;
pub mod make;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The cache's sizes: Files draws icons up to 128 points from `Normal`, larger or at scale 2
/// from `Large`, the preview and index sheet from `XLarge`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Flavour {
    Normal,
    Large,
    XLarge,
}

impl Flavour {
    pub fn pixels(self) -> u32 {
        match self {
            Flavour::Normal => 128,
            Flavour::Large => 256,
            Flavour::XLarge => 512,
        }
    }

    pub fn dir(self) -> &'static str {
        match self {
            Flavour::Normal => "normal",
            Flavour::Large => "large",
            Flavour::XLarge => "x-large",
        }
    }

    /// The smallest that is at least `pixels` across.
    pub fn for_pixels(pixels: f32) -> Self {
        if pixels <= 128. {
            Flavour::Normal
        } else if pixels <= 256. {
            Flavour::Large
        } else {
            Flavour::XLarge
        }
    }
}

/// What Files asks the helper, one JSON line each.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ask {
    pub id: u64,
    pub path: PathBuf,
    pub mime: String,
    pub flavour: Flavour,
    /// The file's modification time in seconds, written into the thumbnail.
    pub mtime: i64,
    pub size: u64,
}

/// The helper's answer: the thumbnail written, or none to be had.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    pub id: u64,
    pub thumbnail: Option<PathBuf>,
}

/// A file's URI as the cache names it: every byte outside the unreserved set escaped.
pub fn uri(path: &std::path::Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut out = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_escape_as_glib_does() {
        assert_eq!(uri(std::path::Path::new("/home/me/My Shot.png")), "file:///home/me/My%20Shot.png");
        assert_eq!(uri(std::path::Path::new("/a/ü")), "file:///a/%C3%BC");
    }
}
