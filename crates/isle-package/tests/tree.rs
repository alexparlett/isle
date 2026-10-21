//! What the package installs, checked against the tree it is built from.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// A packaged binary runs where its build tree is gone, so none reads a path under it.
#[test]
fn no_installed_binary_reads_the_build_tree() {
    // Never packaged, or a maintainer's command that writes the tree it was built from.
    let allowed = [
        "crates/isle-preview/",
        "crates/isle-system-settings/src/main.rs",
        "crates/isle-files/src/main.rs",
    ];
    let mut files = Vec::new();
    for krate in std::fs::read_dir(root().join("crates")).unwrap() {
        let src = krate.unwrap().path().join("src");
        if src.is_dir() {
            sources(&src, &mut files);
        }
    }
    let mut found = Vec::new();
    for file in files {
        let shown = file
            .strip_prefix(root())
            .unwrap()
            .to_string_lossy()
            .into_owned();
        if allowed.iter().any(|a| shown.starts_with(a)) {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap();
        // Test modules sit at the end of their file.
        let shipped = text.split("#[cfg(test)]").next().unwrap();
        for (n, line) in shipped.lines().enumerate() {
            if line.contains("CARGO_MANIFEST_DIR") {
                found.push(format!("{shown}:{}", n + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "the build tree read at run time: {found:?}"
    );
}
