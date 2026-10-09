//! AppImages in `~/Applications` as desktop entries, read out of the image without running it
//! (`platform.md` App index 3, v1 D82). The entries and their keys are v1's, so an entry v1
//! wrote is recognised.

use std::{
    collections::HashMap,
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU32, Ordering},
};

use tracing::{debug, warn};

const PREFIX: &str = "isle-appimage-";
/// The image the entry was written for, and its modification time and size then, so a newer
/// version dropped over the old one is read again.
const PATH_KEY: &str = "X-Isle-AppImage";
const STAMP_KEY: &str = "X-Isle-AppImage-Stamp";
/// What is carried over from the image's own entry.
const KEEP: [&str; 10] = [
    "Name",
    "GenericName",
    "Comment",
    "Categories",
    "Keywords",
    "MimeType",
    "Terminal",
    "StartupWMClass",
    "StartupNotify",
    "NoDisplay",
];

/// Where AppImages are installed by being put there.
pub fn apps_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Applications"))
}

/// Where entries and icons are written: `$XDG_DATA_HOME`, else `~/.local/share`.
pub fn data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
}

/// 1 or 2 for an AppImage of that type, `None` for any other file.
pub fn kind_of(path: &Path) -> Option<u8> {
    let mut head = [0u8; 16];
    fs::File::open(path).ok()?.read_exact(&mut head).ok()?;
    (head[..4] == *b"\x7fELF" && head[8..10] == *b"AI" && matches!(head[10], 1 | 2))
        .then_some(head[10])
}

/// Where a type 2 image's squashfs starts: the end of the ELF, after its section headers.
fn payload_offset(path: &Path) -> Option<u64> {
    let mut head = [0u8; 64];
    fs::File::open(path).ok()?.read_exact(&mut head).ok()?;
    let little = head[5] == 1;
    let u16_at = |at: usize| {
        let b = [head[at], head[at + 1]];
        if little {
            u16::from_le_bytes(b)
        } else {
            u16::from_be_bytes(b)
        }
    };
    let (shoff, entsize, num) = if head[4] == 2 {
        let b: [u8; 8] = head[0x28..0x30].try_into().ok()?;
        let off = if little {
            u64::from_le_bytes(b)
        } else {
            u64::from_be_bytes(b)
        };
        (off, u16_at(0x3A), u16_at(0x3C))
    } else {
        let b: [u8; 4] = head[0x20..0x24].try_into().ok()?;
        let off = if little {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        };
        (off as u64, u16_at(0x2E), u16_at(0x30))
    };
    Some(shoff + entsize as u64 * num as u64)
}

/// The image's root entry and icons, unpacked into `into` by `unsquashfs` (type 2) or `bsdtar`
/// (type 1); the image itself never runs.
fn unpack(path: &Path, into: &Path) -> bool {
    let mut cmd = match kind_of(path) {
        Some(2) => {
            let Some(offset) = payload_offset(path) else {
                return false;
            };
            let mut cmd = Command::new("unsquashfs");
            cmd.args(["-no-progress", "-force", "-dest"])
                .arg(into)
                .args(["-offset", &offset.to_string()])
                .arg(path)
                .args([
                    "/*.desktop",
                    "/.DirIcon",
                    "/*.png",
                    "/*.svg",
                    "/usr/share/icons",
                ]);
            cmd
        }
        Some(1) => {
            fs::create_dir_all(into).ok();
            let mut cmd = Command::new("bsdtar");
            cmd.args(["-x", "-f"]).arg(path).arg("-C").arg(into);
            cmd
        }
        _ => return false,
    };
    let status = cmd.stdout(Stdio::null()).stderr(Stdio::null()).status();
    matches!(status, Ok(s) if s.success()) && into.is_dir()
}

/// The `[Desktop Entry]` group's untranslated keys.
fn entry_keys(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == "[Desktop Entry]";
            continue;
        }
        if !inside || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=')
            && !k.contains('[')
        {
            out.insert(k.trim().to_owned(), v.trim().to_owned());
        }
    }
    out
}

/// The image's own entry at its root: its file stem and keys.
fn bundled_entry(root: &Path) -> (Option<String>, HashMap<String, String>) {
    let mut names: Vec<String> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.ends_with(".desktop"))
        .collect();
    names.sort();
    for name in &names {
        let keys = fs::read_to_string(root.join(name))
            .map(|t| entry_keys(&t))
            .unwrap_or_default();
        if keys.contains_key("Name") {
            return (Some(name.trim_end_matches(".desktop").to_owned()), keys);
        }
    }
    (
        names
            .first()
            .map(|n| n.trim_end_matches(".desktop").to_owned()),
        HashMap::new(),
    )
}

/// A PNG's width from its header, so its icon lands in the size directory it belongs to.
fn png_size(path: &Path) -> u32 {
    let mut head = [0u8; 24];
    match fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)) {
        Ok(()) if head[..8] == *b"\x89PNG\r\n\x1a\n" => {
            u32::from_be_bytes(head[16..20].try_into().unwrap())
        }
        _ => 0,
    }
}

/// The best picture in the image for the icon its entry names, an SVG over any PNG, else its
/// `.DirIcon`.
fn bundled_icon(root: &Path, named: &str) -> Option<PathBuf> {
    let wanted = Path::new(named)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let mut found: Vec<(bool, u32, PathBuf)> = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let (Some(stem), Some(ext)) = (
                path.file_stem().and_then(|s| s.to_str()),
                path.extension().and_then(|e| e.to_str()),
            ) else {
                continue;
            };
            let ext = ext.to_lowercase();
            if !matches!(ext.as_str(), "png" | "svg")
                || (!wanted.is_empty() && stem != wanted)
                || !path.exists()
            {
                continue;
            }
            let svg = ext == "svg";
            found.push((svg, if svg { 0 } else { png_size(&path) }, path));
        }
    }
    found.sort();
    found.pop().map(|(_, _, p)| p).or_else(|| {
        let dir_icon = root.join(".DirIcon");
        dir_icon.exists().then_some(dir_icon)
    })
}

fn stamp_of(path: &Path) -> String {
    fs::metadata(path)
        .map(|m| format!("{}:{}", m.mtime(), m.size()))
        .unwrap_or_default()
}

/// A registered image: the entry written for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registered {
    pub image: PathBuf,
    pub id: String,
    pub icon: String,
    stamp: String,
}

/// The entries written for images, by image.
pub fn registered(entries: &Path) -> HashMap<PathBuf, Registered> {
    let mut out = HashMap::new();
    for e in fs::read_dir(entries).into_iter().flatten().flatten() {
        let Ok(name) = e.file_name().into_string() else {
            continue;
        };
        let Some(id) = name
            .strip_suffix(".desktop")
            .filter(|id| id.starts_with(PREFIX))
        else {
            continue;
        };
        let keys = fs::read_to_string(e.path())
            .map(|t| entry_keys(&t))
            .unwrap_or_default();
        let Some(image) = keys.get(PATH_KEY) else {
            continue;
        };
        out.insert(
            PathBuf::from(image),
            Registered {
                image: PathBuf::from(image),
                id: id.to_owned(),
                icon: keys.get("Icon").cloned().unwrap_or_default(),
                stamp: keys.get(STAMP_KEY).cloned().unwrap_or_default(),
            },
        );
    }
    out
}

/// The entry's id from the image's own name for itself, kept across versions; a second image
/// with the same name is told apart by a hash of its path.
fn id_for(entries: &Path, image: &Path, stem: &str) -> String {
    let base: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "_.-".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect();
    let base = base.trim_matches('-');
    let mut id = format!("{PREFIX}{}", if base.is_empty() { "app" } else { base });
    let at = entries.join(format!("{id}.desktop"));
    if let Ok(text) = fs::read_to_string(&at)
        && entry_keys(&text)
            .get(PATH_KEY)
            .is_some_and(|p| Path::new(p) != image)
    {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut h = DefaultHasher::new();
        image.hash(&mut h);
        id.push_str(&format!("-{:06x}", h.finish() & 0xff_ffff));
    }
    id
}

/// The picture copied into the hicolor theme under the entry's id, so anything drawing an icon
/// by name finds it.
fn put_icon(icons: &Path, source: &Path, name: &str) -> Option<String> {
    let ext = source.extension()?.to_str()?.to_lowercase();
    let size = if ext == "png" { png_size(source) } else { 0 };
    let dir = match (ext.as_str(), size) {
        ("svg", _) => "scalable".to_owned(),
        (_, 0) => "48x48".to_owned(),
        (_, s) => format!("{s}x{s}"),
    };
    let into = icons.join(dir).join("apps");
    fs::create_dir_all(&into).ok()?;
    fs::copy(source, into.join(format!("{name}.{ext}"))).ok()?;
    Some(name.to_owned())
}

fn drop_icon(icons: &Path, name: &str) {
    if !name.starts_with(PREFIX) {
        return;
    }
    let mut stack = vec![icons.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = e.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_stem().and_then(|s| s.to_str()) == Some(name) {
                let _ = fs::remove_file(path);
            }
        }
    }
}

fn quoted(path: &Path) -> String {
    let p = path.to_string_lossy();
    format!("\"{}\"", p.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The image's entry as Isle's own: the same app, told to run the file where it is.
fn write_entry(
    entries: &Path,
    id: &str,
    image: &Path,
    keys: &HashMap<String, String>,
    icon: Option<&str>,
) -> std::io::Result<()> {
    // What the image's Exec passes after its own command, which is a path inside the image.
    let args = keys
        .get("Exec")
        .and_then(|e| e.split_once(' '))
        .map(|(_, rest)| rest.trim())
        .filter(|r| !r.is_empty())
        .map(|r| format!(" {r}"))
        .unwrap_or_default();
    let mut lines = vec!["[Desktop Entry]".to_owned(), "Type=Application".to_owned()];
    for key in KEEP {
        if let Some(v) = keys.get(key).filter(|v| !v.is_empty()) {
            lines.push(format!("{key}={v}"));
        }
    }
    if !keys.contains_key("Name") {
        let stem = image
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        lines.push(format!("Name={stem}"));
    }
    lines.push(format!("Exec={}{args}", quoted(image)));
    // Hidden the moment the file is gone, before the next sync.
    lines.push(format!("TryExec={}", image.display()));
    lines.push(format!(
        "Icon={}",
        icon.unwrap_or("application-x-executable")
    ));
    lines.push(format!("{PATH_KEY}={}", image.display()));
    lines.push(format!("{STAMP_KEY}={}", stamp_of(image)));
    fs::create_dir_all(entries)?;
    fs::write(
        entries.join(format!("{id}.desktop")),
        lines.join("\n") + "\n",
    )
}

fn scratch_dir() -> PathBuf {
    static COUNT: AtomicU32 = AtomicU32::new(0);
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join(format!(
        "isle-appimage-{}-{}",
        std::process::id(),
        COUNT.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Registers `image`, writing its entry under `entries` and its icon under `icons`; the
/// entry's id. Marks the image executable, as a Mac app is.
fn add(image: &Path, entries: &Path, icons: &Path) -> Option<String> {
    kind_of(image)?;
    if let Ok(meta) = fs::metadata(image) {
        let mut perms = meta.permissions();
        perms.set_mode(perms.mode() | 0o111);
        let _ = fs::set_permissions(image, perms);
    }
    let scratch = scratch_dir();
    let _ = fs::create_dir_all(&scratch);
    let root = scratch.join("squashfs-root");
    let (stem, keys, picture) = if unpack(image, &root) {
        let (stem, keys) = bundled_entry(&root);
        let picture = bundled_icon(&root, keys.get("Icon").map_or("", String::as_str));
        (stem, keys, picture)
    } else {
        warn!(image = %image.display(), "could not read the AppImage's entry");
        (None, HashMap::new(), None)
    };
    let stem = stem.unwrap_or_else(|| {
        image
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    let id = id_for(entries, image, &stem);
    let icon = picture.and_then(|p| put_icon(icons, &p, &id));
    let written = write_entry(entries, &id, image, &keys, icon.as_deref());
    let _ = fs::remove_dir_all(&scratch);
    written.ok().map(|_| id)
}

fn forget(known: &Registered, entries: &Path, icons: &Path) {
    let _ = fs::remove_file(entries.join(format!("{}.desktop", known.id)));
    drop_icon(icons, &known.icon);
}

/// What a sync changed, by entry id.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Synced {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

/// Registers what is in `apps` and forgets what is gone, writing entries under `entries` and
/// icons under `icons`. Blocking, so never on the main loop.
pub fn sync(apps: &Path, entries: &Path, icons: &Path) -> Synced {
    let known = registered(entries);
    let mut here: Vec<PathBuf> = fs::read_dir(apps)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && kind_of(p).is_some())
        .collect();
    here.sort();
    let mut synced = Synced::default();
    for image in &here {
        if let Some(was) = known.get(image) {
            if was.stamp == stamp_of(image) {
                continue;
            }
            forget(was, entries, icons);
        }
        if let Some(id) = add(image, entries, icons) {
            debug!(id, "AppImage registered");
            synced.added.push(id);
        }
    }
    for (image, was) in &known {
        if !here.contains(image) && image.parent() == Some(apps) {
            forget(was, entries, icons);
            synced.removed.push(was.id.clone());
        }
    }
    synced
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An ELF header marked as a type 2 AppImage whose section headers end at 64, then
    /// `payload`.
    fn image(payload: &[u8]) -> Vec<u8> {
        let mut head = vec![0u8; 64];
        head[..4].copy_from_slice(b"\x7fELF");
        head[4] = 2;
        head[5] = 1;
        head[8..11].copy_from_slice(b"AI\x02");
        head[0x28..0x30].copy_from_slice(&64u64.to_le_bytes());
        head.extend_from_slice(payload);
        head
    }

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("isle-appimage-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_header_says_what_it_is() {
        let dir = temp("kind");
        fs::write(dir.join("a"), image(b"")).unwrap();
        fs::write(dir.join("b"), b"#!/bin/sh\n").unwrap();
        assert_eq!(kind_of(&dir.join("a")), Some(2));
        assert_eq!(kind_of(&dir.join("b")), None);
        assert_eq!(payload_offset(&dir.join("a")), Some(64));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_image_becomes_an_entry_and_goes_with_it() {
        if Command::new("mksquashfs").arg("-version").output().is_err() {
            eprintln!("mksquashfs missing; skipped");
            return;
        }
        let dir = temp("sync");
        let (src, apps, entries, icons) = (
            dir.join("src"),
            dir.join("Applications"),
            dir.join("applications"),
            dir.join("icons"),
        );
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&apps).unwrap();
        fs::write(src.join("tool.desktop"), "[Desktop Entry]\nType=Application\nName=Tool\nName[fr]=Outil\nExec=tool %U\nIcon=tool\nCategories=Utility;\n").unwrap();
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&128u32.to_be_bytes());
        png.extend_from_slice(&128u32.to_be_bytes());
        fs::write(src.join("tool.png"), png).unwrap();
        let squash = dir.join("payload.squashfs");
        let made = Command::new("mksquashfs")
            .arg(&src)
            .arg(&squash)
            .args(["-quiet", "-noappend", "-all-root"])
            .status()
            .unwrap();
        assert!(made.success());
        let image_path = apps.join("Tool-1.0.AppImage");
        fs::write(&image_path, image(&fs::read(&squash).unwrap())).unwrap();

        let synced = sync(&apps, &entries, &icons);
        assert_eq!(synced.added, ["isle-appimage-tool"]);
        let written = fs::read_to_string(entries.join("isle-appimage-tool.desktop")).unwrap();
        let entry = crate::parse_in("isle-appimage-tool", &written, &[]).unwrap();
        assert_eq!(entry.name, "Tool");
        assert_eq!(
            entry.exec.as_deref(),
            Some(format!("\"{}\" %U", image_path.display()).as_str())
        );
        assert_eq!(entry.icon.as_deref(), Some("isle-appimage-tool"));
        assert_eq!(entry.categories, ["Utility"]);
        assert!(icons.join("128x128/apps/isle-appimage-tool.png").exists());
        assert!(fs::metadata(&image_path).unwrap().permissions().mode() & 0o111 != 0);

        assert_eq!(sync(&apps, &entries, &icons), Synced::default());

        fs::remove_file(&image_path).unwrap();
        let synced = sync(&apps, &entries, &icons);
        assert_eq!(synced.removed, ["isle-appimage-tool"]);
        assert!(!entries.join("isle-appimage-tool.desktop").exists());
        assert!(!icons.join("128x128/apps/isle-appimage-tool.png").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
