//! The ten kinds (D88): a folder's name gives its kind to everything under it, a nearer folder
//! winning, and a file's ending counts only where no folder has said.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Kind {
    Games,
    Video,
    Images,
    Audio,
    Documents,
    Archives,
    Developer,
    System,
    Caches,
    Other,
}

pub const COUNT: usize = 10;

impl Kind {
    /// In P1's order, the legend's and the Storage bar's.
    pub const ALL: [Kind; COUNT] = [
        Kind::Games,
        Kind::Video,
        Kind::Images,
        Kind::Audio,
        Kind::Documents,
        Kind::Archives,
        Kind::Developer,
        Kind::System,
        Kind::Caches,
        Kind::Other,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Games => "Games",
            Kind::Video => "Video",
            Kind::Images => "Images",
            Kind::Audio => "Audio",
            Kind::Documents => "Documents",
            Kind::Archives => "Archives and disk images",
            Kind::Developer => "Developer",
            Kind::System => "Apps and system",
            Kind::Caches => "Caches and logs",
            Kind::Other => "Other",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

fn folder(name: &[u8]) -> Option<Kind> {
    use Kind::*;
    Some(match name {
        b"steamapps"
        | b"Steam"
        | b".steam"
        | b"Games"
        | b"lutris"
        | b"heroic"
        | b"Heroic"
        | b"itch"
        | b".wine"
        | b"compatibilitytools.d"
        | b"compatdata"
        | b"pfx"
        | b"drive_c" => Games,
        b".cache" | b"cache" | b"Cache" | b"caches" | b"Caches" | b"CachedData" | b"GPUCache"
        | b"Code Cache" | b"ShaderCache" | b"shadercache" | b"DawnCache" | b"GrShaderCache"
        | b"thumbnails" | b"log" | b"logs" | b"Logs" | b"journal" | b"coredump"
        | b"Crash Reports" => Caches,
        b"node_modules" | b".git" | b".cargo" | b".rustup" | b".npm" | b".pnpm-store"
        | b".yarn" | b".gradle" | b".m2" | b".venv" | b"venv" | b"__pycache__" | b".nuget"
        | b".dotnet" | b".android" | b"Android" | b"docker" | b"containers" | b"target"
        | b"site-packages" | b"dist-packages" => Developer,
        b"flatpak" | b"Applications" | b"pacman" => System,
        _ => return None,
    })
}

/// Folders that are the system only directly under `/`.
fn root_folder(name: &[u8]) -> Option<Kind> {
    matches!(name, b"usr" | b"opt" | b"boot" | b"etc").then_some(Kind::System)
}

/// The kind a folder named `name` gives what is under it: its own, or `inherited`.
/// `at_root` is true for a folder directly under `/`.
pub fn of_folder(name: &[u8], at_root: bool, inherited: Option<Kind>) -> Option<Kind> {
    at_root
        .then(|| root_folder(name))
        .flatten()
        .or_else(|| folder(name))
        .or(inherited)
}

fn ending(ending: &str) -> Kind {
    use Kind::*;
    match ending {
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "m4v" | "wmv" | "flv" | "mpg" | "mpeg" | "ts"
        | "m2ts" => Video,
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "heif" | "avif" | "cr2" | "cr3"
        | "nef" | "arw" | "dng" | "raf" | "orf" | "tif" | "tiff" | "bmp" | "svg" | "psd"
        | "xcf" | "kra" | "exr" => Images,
        "mp3" | "flac" | "wav" | "ogg" | "opus" | "m4a" | "aac" | "wma" | "aiff" | "aif"
        | "alac" => Audio,
        "pdf" | "doc" | "docx" | "odt" | "rtf" | "txt" | "md" | "epub" | "mobi" | "xls"
        | "xlsx" | "ods" | "ppt" | "pptx" | "odp" | "csv" | "tex" | "pages" | "key" | "numbers" => {
            Documents
        }
        "zip" | "tar" | "gz" | "tgz" | "xz" | "zst" | "bz2" | "7z" | "rar" | "iso" | "img"
        | "qcow2" | "vdi" | "vmdk" | "vhd" | "vhdx" | "dmg" => Archives,
        "c" | "h" | "cc" | "cpp" | "hpp" | "rs" | "go" | "py" | "js" | "mjs" | "tsx" | "jsx"
        | "java" | "kt" | "swift" | "rb" | "php" | "cs" | "o" | "a" | "obj" | "rlib" | "rmeta"
        | "class" | "jar" | "pyc" | "wasm" | "pack" | "idx" | "safetensors" | "gguf" | "onnx"
        | "pt" | "ckpt" => Developer,
        "pak" | "ucas" | "utoc" | "vpk" | "bsa" | "ba2" | "gcf" | "forge" | "wad" => Games,
        "so" | "exe" | "dll" | "appimage" | "flatpakref" | "ko" | "efi" => System,
        "log" => Caches,
        _ => Other,
    }
}

/// A file's kind from its name's ending alone.
pub fn of_file(name: &[u8]) -> Kind {
    let Some(dot) = name.iter().rposition(|b| *b == b'.') else {
        return Kind::Other;
    };
    let tail = &name[dot + 1..];
    if dot == 0 || tail.is_empty() || tail.len() > 12 {
        return Kind::Other;
    }
    match std::str::from_utf8(tail) {
        Ok(t) => ending(&t.to_ascii_lowercase()),
        Err(_) => Kind::Other,
    }
}

/// A file's kind where it sits: its folders' first, then its ending.
pub fn of(folder: Option<Kind>, name: &[u8]) -> Kind {
    folder.unwrap_or_else(|| of_file(name))
}

/// The kind of the file at an absolute `path`, from its folders and its name.
pub fn of_path(path: &std::path::Path) -> Kind {
    use std::os::unix::ffi::OsStrExt;
    let mut parts: Vec<&[u8]> = path.iter().skip(1).map(|c| c.as_bytes()).collect();
    let Some(name) = parts.pop() else {
        return Kind::Other;
    };
    let folders = parts
        .iter()
        .enumerate()
        .fold(None, |kind, (i, part)| of_folder(part, i == 0, kind));
    of(folders, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn a_folder_says_before_an_ending() {
        assert_eq!(
            of_path(Path::new(
                "/home/u/.local/share/Steam/steamapps/common/G/art.png"
            )),
            Kind::Games
        );
        assert_eq!(
            of_path(Path::new(
                "/home/u/.local/share/Steam/steamapps/shadercache/1/a.bin"
            )),
            Kind::Caches,
            "a nearer folder wins"
        );
        assert_eq!(
            of_path(Path::new("/home/u/Downloads/film.MKV")),
            Kind::Video
        );
        assert_eq!(of_path(Path::new("/home/u/Pictures/a.png")), Kind::Images);
        assert_eq!(
            of_path(Path::new("/home/u/code/app/node_modules/x/readme.md")),
            Kind::Developer
        );
        assert_eq!(of_path(Path::new("/home/u/notes")), Kind::Other);
        assert_eq!(
            of_path(Path::new("/home/u/.bashrc")),
            Kind::Other,
            "a leading dot is not an ending"
        );
    }

    #[test]
    fn system_folders_only_at_the_root() {
        assert_eq!(of_path(Path::new("/usr/lib/libc.so.6")), Kind::System);
        assert_eq!(of_path(Path::new("/usr/share/doc/a.txt")), Kind::System);
        assert_eq!(of_path(Path::new("/home/u/usr/a.txt")), Kind::Documents);
        assert_eq!(of_path(Path::new("/home/u/x.so")), Kind::System);
    }
}
