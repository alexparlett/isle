//! Compiles the shell's glyphs, Phosphor's and Isle's own, into a table by name.

use std::{fmt::Write, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut table = String::from("pub(crate) static GLYPHS: &[(&str, &[u8])] = &[\n");
    let mut names = Vec::new();
    for set in ["phosphor", "isle"] {
        let dir = manifest.join("../../data/icons").join(set);
        println!("cargo::rerun-if-changed={}", dir.display());
        for entry in std::fs::read_dir(&dir).expect("data/icons") {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) == Some("svg") {
                let name = path.file_stem().unwrap().to_string_lossy().into_owned();
                names.push((name, path.canonicalize().unwrap()));
            }
        }
    }
    names.sort();
    for (name, path) in names {
        writeln!(
            table,
            "    ({name:?}, include_bytes!({:?})),",
            path.display().to_string()
        )
        .unwrap();
    }
    table.push_str("];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("glyphs.rs");
    std::fs::write(out, table).unwrap();
}
