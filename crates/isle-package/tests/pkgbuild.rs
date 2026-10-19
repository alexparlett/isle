//! `pkg/arch/PKGBUILD` against the tree it packages.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pkgbuild() -> String {
    std::fs::read_to_string(root().join("pkg/arch/PKGBUILD")).unwrap()
}

/// Every tree path `package()` installs from.
fn installed_from() -> Vec<String> {
    let text = pkgbuild();
    let body = text.split("package() {").nth(1).unwrap();
    body.split_whitespace()
        .filter(|word| word.starts_with("data/"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_file_the_package_installs_is_in_the_tree() {
    let from = installed_from();
    assert!(from.len() > 5, "{from:?}");
    let missing: Vec<&String> = from.iter().filter(|p| !root().join(p).is_file()).collect();
    assert!(
        missing.is_empty(),
        "the PKGBUILD installs what the tree lacks: {missing:?}"
    );
}

fn crate_of(bin: &str) -> &str {
    match bin {
        "isle" => "isle-cli",
        "isle-settings" => "isle-system-settings",
        other => other,
    }
}

#[test]
fn every_binary_the_package_installs_is_built_by_it() {
    let text = pkgbuild();
    let bins = text
        .split("for bin in ")
        .nth(1)
        .and_then(|rest| rest.split("; do").next())
        .unwrap();
    for bin in bins.split_whitespace() {
        assert!(
            text.contains(&format!("-p {}", crate_of(bin))),
            "{bin} is installed but its crate is not built"
        );
    }
}

/// The prebuilt Skia archive is named for the bindings' version, which a Freya update moves.
#[test]
fn the_skia_archive_follows_the_locked_bindings() {
    let lock = std::fs::read_to_string(root().join("Cargo.lock")).unwrap();
    let version = lock
        .split("name = \"freya-skia-bindings\"\nversion = \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .unwrap();
    assert!(
        pkgbuild().contains(&format!("_skia_tag={version}\n")),
        "pkg/arch/PKGBUILD's _skia_tag is not {version}; update it and its archive's sum"
    );
}
