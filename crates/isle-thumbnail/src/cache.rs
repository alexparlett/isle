//! The freedesktop thumbnail cache, spec 0.9.0: `$XDG_CACHE_HOME/thumbnails/<size>/` named by
//! the URI's MD5, valid while its `Thumb::URI` and `Thumb::MTime` (and `Thumb::Size` when
//! present) match the file; failures under `fail/<app>-<version>/`.

use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

use crate::{Flavour, uri};

/// The name failures are kept under.
const FAILED_AS: &str = concat!("isle-files-", env!("CARGO_PKG_VERSION"));

pub fn root() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("thumbnails")
}

fn name(uri: &str) -> String {
    format!("{:x}.png", md5::compute(uri.as_bytes()))
}

pub fn path(root: &Path, file: &Path, flavour: Flavour) -> PathBuf {
    root.join(flavour.dir()).join(name(&uri(file)))
}

pub fn failed_path(root: &Path, file: &Path) -> PathBuf {
    root.join("fail").join(FAILED_AS).join(name(&uri(file)))
}

/// A PNG's text chunks, read without decoding the image.
pub fn text_chunks(png: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if png.len() < 8 || &png[..8] != b"\x89PNG\r\n\x1a\n" {
        return out;
    }
    let mut at = 8;
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        let kind = &png[at + 4..at + 8];
        let body = at + 8;
        if body + len > png.len() {
            break;
        }
        match kind {
            b"tEXt" => {
                let data = &png[body..body + len];
                if let Some(nul) = data.iter().position(|b| *b == 0) {
                    out.push((
                        String::from_utf8_lossy(&data[..nul]).into_owned(),
                        String::from_utf8_lossy(&data[nul + 1..]).into_owned(),
                    ));
                }
            }
            b"IDAT" | b"IEND" => break,
            _ => {}
        }
        at = body + len + 4;
    }
    out
}

/// Reads at most the head of a thumbnail, where its text chunks are.
fn head(path: &Path) -> Option<Vec<u8>> {
    let mut buf = Vec::with_capacity(4096);
    fs::File::open(path).ok()?.take(64 * 1024).read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// Whether the thumbnail at `thumb` still stands for `file` as it is now.
pub fn valid(thumb: &Path, file: &Path, mtime: i64, size: u64) -> bool {
    let Some(head) = head(thumb) else { return false };
    let chunks = text_chunks(&head);
    let get = |key: &str| chunks.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
    get("Thumb::URI") == Some(&uri(file))
        && get("Thumb::MTime").and_then(|m| m.parse::<i64>().ok()) == Some(mtime)
        && get("Thumb::Size").is_none_or(|s| s.parse::<u64>().ok() == Some(size))
}

/// The thumbnail for `file` if one in the cache is good: the size asked for, or a larger one.
pub fn find(root: &Path, file: &Path, flavour: Flavour, mtime: i64, size: u64) -> Option<PathBuf> {
    let larger = [Flavour::Normal, Flavour::Large, Flavour::XLarge]
        .into_iter()
        .filter(|f| f.pixels() >= flavour.pixels());
    larger
        .map(|f| path(root, file, f))
        .find(|p| valid(p, file, mtime, size))
}

/// Whether making one already failed for the file as it is now.
pub fn failed(root: &Path, file: &Path, mtime: i64) -> bool {
    let Some(head) = head(&failed_path(root, file)) else { return false };
    let chunks = text_chunks(&head);
    chunks.iter().any(|(k, v)| k == "Thumb::MTime" && v.parse::<i64>().ok() == Some(mtime))
}

/// Writes an RGBA image as the thumbnail for `file`: to a name of its own beside it, renamed
/// into place, readable by the user alone.
pub fn write(
    root: &Path,
    target: &Path,
    file: &Path,
    rgba: &image::RgbaImage,
    mtime: i64,
    size: u64,
    mime: &str,
) -> std::io::Result<()> {
    let dir = target.parent().unwrap_or(root);
    fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, rgba.width(), rgba.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        for (k, v) in [
            ("Thumb::URI", uri(file)),
            ("Thumb::MTime", mtime.to_string()),
            ("Thumb::Size", size.to_string()),
            ("Thumb::Mimetype", mime.to_owned()),
            ("Software", "Isle Files".to_owned()),
        ] {
            encoder.add_text_chunk(k.to_owned(), v).map_err(std::io::Error::other)?;
        }
        let mut writer = encoder.write_header().map_err(std::io::Error::other)?;
        writer.write_image_data(rgba.as_raw()).map_err(std::io::Error::other)?;
    }
    let part = dir.join(format!(".{}.part-{}", target.file_name().unwrap().to_string_lossy(), std::process::id()));
    let mut out = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&part)?;
    out.write_all(&bytes)?;
    drop(out);
    fs::rename(&part, target)
}

/// Marks `file` as one no thumbnail can be made for, until it changes.
pub fn write_failed(root: &Path, file: &Path, mtime: i64) -> std::io::Result<()> {
    let target = failed_path(root, file);
    let blank = image::RgbaImage::new(1, 1);
    write(root, &target, file, &blank, mtime, 0, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_written_thumbnail_is_valid_until_the_file_changes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("thumbnails");
        let file = dir.path().join("shot.png");
        let image = image::RgbaImage::from_pixel(4, 2, image::Rgba([1, 2, 3, 255]));
        let target = path(&root, &file, Flavour::Large);
        write(&root, &target, &file, &image, 1000, 42, "image/png").unwrap();
        assert!(valid(&target, &file, 1000, 42));
        assert!(!valid(&target, &file, 1001, 42), "a newer file");
        assert!(!valid(&target, &file, 1000, 43), "another size");
        assert_eq!(find(&root, &file, Flavour::Normal, 1000, 42), Some(target.clone()), "a larger one serves");
        assert_eq!(find(&root, &file, Flavour::XLarge, 1000, 42), None);
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(fs::metadata(target.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700);

        assert!(!failed(&root, &file, 5));
        write_failed(&root, &file, 5).unwrap();
        assert!(failed(&root, &file, 5));
        assert!(!failed(&root, &file, 6));
    }

    #[test]
    fn names_are_the_uris_md5() {
        // The spec's own example.
        assert_eq!(name("file:///home/jens/photos/me.png"), "c6ee772d9e49320e97ec29a7eb5b1697.png");
    }
}
