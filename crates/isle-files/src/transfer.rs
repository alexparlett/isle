//! Files on the clipboard and in drags, as other apps read and write them: `text/uri-list`,
//! GNOME's `x-special/gnome-copied-files` (which also says copy or cut), and the paths as text
//! (`p15-files.md` choice 5).

use std::path::PathBuf;

use isle_app::data::Contents;

pub const GNOME: &str = "x-special/gnome-copied-files";
pub const URIS: &str = "text/uri-list";

/// The types a paste or a drop is read in, best first.
pub const READ: [&str; 2] = [GNOME, URIS];

/// Files as the clipboard or a drag offers them.
pub fn contents(paths: &[PathBuf], cut: bool) -> Contents {
    let uris: Vec<String> = paths.iter().map(|p| isle_thumbnail::uri(p)).collect();
    let text = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>().join("\n");
    vec![
        (GNOME.into(), format!("{}\n{}", if cut { "cut" } else { "copy" }, uris.join("\n")).into_bytes()),
        (URIS.into(), format!("{}\r\n", uris.join("\r\n")).into_bytes()),
        ("text/plain;charset=utf-8".into(), text.clone().into_bytes()),
        ("text/plain".into(), text.into_bytes()),
    ]
}

/// The files a paste or a drop carries, and whether they were cut.
pub fn read(kind: &str, data: &[u8]) -> (Vec<PathBuf>, bool) {
    let text = String::from_utf8_lossy(data);
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#'));
    let cut = if kind == GNOME { lines.next() == Some("cut") } else { false };
    (lines.filter_map(crate::path_of).collect(), cut)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_go_out_and_come_back() {
        let paths = vec![PathBuf::from("/home/me/a b.txt"), PathBuf::from("/home/me/c")];
        let out = contents(&paths, true);
        let gnome = &out.iter().find(|(t, _)| t == GNOME).unwrap().1;
        assert_eq!(read(GNOME, gnome), (paths.clone(), true));
        let uris = &out.iter().find(|(t, _)| t == URIS).unwrap().1;
        assert_eq!(String::from_utf8_lossy(uris), "file:///home/me/a%20b.txt\r\nfile:///home/me/c\r\n");
        assert_eq!(read(URIS, uris), (paths, false));
        assert_eq!(read(URIS, b"# comment\nfile:///x\n").0, [PathBuf::from("/x")]);
    }
}
