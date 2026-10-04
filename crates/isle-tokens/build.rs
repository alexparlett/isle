//! Types every token of the shipped `tokens.kdl` as a field of `Tokens`.

#[path = "src/flat.rs"]
#[allow(dead_code)]
mod flat;

use std::{collections::BTreeMap, fmt::Write, path::PathBuf};

use flat::{Kind, Mode, Raw};

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let source = manifest.join("../../data/tokens/tokens.kdl");
    println!("cargo::rerun-if-changed={}", source.display());
    println!("cargo::rerun-if-changed=src/flat.rs");
    let text = std::fs::read_to_string(&source).expect("data/tokens/tokens.kdl");
    let entries = flat::parse(&text).unwrap_or_else(|p| {
        panic!("tokens.kdl at byte {}: {}", p.offset, p.message)
    });
    let accent = flat::resolve(&entries, Mode::Dark, "", false)
        .get("accent-default")
        .and_then(|v| match v {
            Raw::Text(t) => Some(t.clone()),
            Raw::Number(_) => None,
        })
        .expect("accent default");

    let mut kinds: BTreeMap<String, Kind> = BTreeMap::new();
    for mode in [Mode::Dark, Mode::Light] {
        for (key, value) in flat::resolve(&entries, mode, &accent, false) {
            let kind = flat::kind_of(&key, &value);
            if let Some(seen) = kinds.insert(key.clone(), kind) {
                assert_eq!(seen, kind, "{key} is a different kind in dark and light");
            }
        }
    }

    let mut out = String::new();
    writeln!(out, "/// Every token, resolved for one appearance.").unwrap();
    writeln!(out, "#[derive(Clone, Debug, PartialEq)]").unwrap();
    writeln!(out, "pub struct Tokens {{").unwrap();
    for (key, kind) in &kinds {
        let ty = match kind {
            Kind::Colour => "Colour",
            Kind::Shadows => "Vec<Shadow>",
            Kind::Millis | Kind::Count => "u32",
            Kind::Number => "f32",
            Kind::Text => "String",
        };
        writeln!(out, "    /// `{key}`").unwrap();
        writeln!(out, "    pub {}: {ty},", key.replace('-', "_")).unwrap();
    }
    writeln!(out, "}}\n").unwrap();
    writeln!(out, "impl Tokens {{").unwrap();
    writeln!(out, "    /// Every token's key, as the file names it.").unwrap();
    writeln!(out, "    pub const KEYS: &[&str] = &[").unwrap();
    for key in kinds.keys() {
        writeln!(out, "        {key:?},").unwrap();
    }
    writeln!(out, "    ];\n").unwrap();
    writeln!(
        out,
        "    pub(crate) fn from_map(map: &BTreeMap<String, Raw>) -> Result<Self, String> {{"
    )
    .unwrap();
    writeln!(out, "        Ok(Self {{").unwrap();
    for (key, kind) in &kinds {
        let getter = match kind {
            Kind::Colour => "colour",
            Kind::Shadows => "shadows",
            Kind::Millis | Kind::Count => "count",
            Kind::Number => "number",
            Kind::Text => "text",
        };
        writeln!(
            out,
            "            {}: get::{getter}(map, {key:?})?,",
            key.replace('-', "_")
        )
        .unwrap();
    }
    writeln!(out, "        }})\n    }}\n}}").unwrap();

    let dest = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("tokens.rs");
    std::fs::write(dest, out).unwrap();
}
