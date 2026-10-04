//! `tokens.kdl` as flat entries, and their resolution for one appearance. Shared by the build
//! script, which types the shipped file, and the library, which reads files at run time.

use std::collections::BTreeMap;

use kdl::{KdlDocument, KdlNode, KdlValue};

#[derive(Clone, Debug, PartialEq)]
pub enum Raw {
    Number(f64),
    Text(String),
}

/// One leaf of the file: its node path, its value, and where the value is.
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: Vec<String>,
    pub value: Raw,
    pub offset: usize,
}

/// A problem at a byte offset of the file.
#[derive(Clone, Debug)]
pub struct Problem {
    pub offset: usize,
    pub message: String,
}

pub fn parse(text: &str) -> Result<Vec<Entry>, Problem> {
    let doc: KdlDocument = text.parse().map_err(|err: kdl::KdlError| {
        let first = err.diagnostics.first();
        Problem {
            offset: first.map_or(0, |d| d.span.offset()),
            message: first
                .and_then(|d| d.message.clone())
                .unwrap_or_else(|| "the file is not KDL".into()),
        }
    })?;
    let mut out = Vec::new();
    for node in doc.nodes() {
        flatten(node, &mut Vec::new(), &mut out)?;
    }
    Ok(out)
}

fn flatten(node: &KdlNode, path: &mut Vec<String>, out: &mut Vec<Entry>) -> Result<(), Problem> {
    path.push(node.name().value().to_string());
    match node.children() {
        Some(children) => {
            if !node.entries().is_empty() {
                return Err(Problem {
                    offset: node.span().offset(),
                    message: format!("{} has both a value and children", path.join("-")),
                });
            }
            for child in children.nodes() {
                flatten(child, path, out)?;
            }
        }
        None => {
            let [entry] = node.entries() else {
                return Err(Problem {
                    offset: node.span().offset(),
                    message: format!("{} needs exactly one value", path.join("-")),
                });
            };
            let value = match entry.value() {
                KdlValue::String(s) => Raw::Text(s.clone()),
                KdlValue::Integer(i) => Raw::Number(*i as f64),
                KdlValue::Float(f) => Raw::Number(*f),
                other => {
                    return Err(Problem {
                        offset: entry.span().offset(),
                        message: format!("{} is {other}, not a number or a string", path.join("-")),
                    });
                }
            };
            out.push(Entry {
                path: path.clone(),
                value,
                offset: entry.span().offset(),
            });
        }
    }
    path.pop();
    Ok(())
}

/// Lays `over` on `base`: an entry with the same path replaces the base's.
pub fn merge(base: &[Entry], over: &[Entry]) -> Vec<Entry> {
    let mut merged = base.to_vec();
    for entry in over {
        match merged.iter_mut().find(|e| e.path == entry.path) {
            Some(slot) => *slot = entry.clone(),
            None => merged.push(entry.clone()),
        }
    }
    merged
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Dark,
    Light,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Dark => "dark",
            Mode::Light => "light",
        }
    }
}

/// The accents the file defines, in order.
pub fn accents(entries: &[Entry]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for e in entries {
        if e.path.len() == 4 && e.path[0] == "accent" && !names.contains(&e.path[1]) {
            names.push(e.path[1].clone());
        }
    }
    names
}

/// Every token for one appearance, keyed by its path joined with `-` without the appearance.
/// The chosen accent's set becomes `colour-accent`, `colour-on-accent` and `colour-accent-text`;
/// with `contrast`, `colour.contrast` replaces the roles it names.
pub fn resolve(
    entries: &[Entry],
    mode: Mode,
    accent: &str,
    contrast: bool,
) -> BTreeMap<String, Raw> {
    let other = match mode {
        Mode::Dark => Mode::Light.name(),
        Mode::Light => Mode::Dark.name(),
    };
    let accents = accents(entries);
    let mut out = BTreeMap::new();
    let mut overrides = BTreeMap::new();
    for e in entries {
        if e.path.iter().any(|s| s == other) {
            continue;
        }
        let path: Vec<&str> = e
            .path
            .iter()
            .map(String::as_str)
            .filter(|s| *s != mode.name())
            .collect();
        match path.as_slice() {
            ["colour", "contrast", rest @ ..] => {
                overrides.insert(format!("colour-{}", rest.join("-")), e.value.clone());
            }
            ["accent", name, role] if accents.iter().any(|a| a == name) => {
                if *name == accent {
                    let key = match *role {
                        "fill" => "colour-accent".to_string(),
                        "on" => "colour-on-accent".to_string(),
                        "text" => "colour-accent-text".to_string(),
                        other => format!("colour-accent-{other}"),
                    };
                    out.insert(key, e.value.clone());
                }
            }
            _ => {
                out.insert(path.join("-"), e.value.clone());
            }
        }
    }
    if contrast {
        out.extend(overrides);
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

impl Colour {
    pub const TRANSPARENT: Colour = Colour {
        r: 0,
        g: 0,
        b: 0,
        a: 0.0,
    };

    /// `#rrggbb` or `rgba(r,g,b,a)`.
    pub fn parse(text: &str) -> Option<Colour> {
        let text = text.trim();
        if let Some(hex) = text.strip_prefix('#') {
            if hex.len() != 6 {
                return None;
            }
            let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
            return Some(Colour {
                r: byte(0)?,
                g: byte(2)?,
                b: byte(4)?,
                a: 1.0,
            });
        }
        let inner = text.strip_prefix("rgba(")?.strip_suffix(')')?;
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        let [r, g, b, a] = parts.as_slice() else {
            return None;
        };
        Some(Colour {
            r: r.parse().ok()?,
            g: g.parse().ok()?,
            b: b.parse().ok()?,
            a: a.parse().ok().filter(|a: &f32| (0.0..=1.0).contains(a))?,
        })
    }

    /// `self` moved toward `other` by `t`, 0 to 1.
    pub fn mix(self, other: Colour, t: f32) -> Colour {
        let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Colour {
            r: channel(self.r, other.r),
            g: channel(self.g, other.g),
            b: channel(self.b, other.b),
            a: self.a + (other.a - self.a) * t,
        }
    }
}

/// One layer of a CSS box shadow, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub colour: Colour,
    pub inset: bool,
}

/// A CSS shadow list: `0 22px 60px rgba(0,0,0,0.55), inset 0 0 0 0.5px rgba(...)`.
pub fn parse_shadows(text: &str) -> Option<Vec<Shadow>> {
    let mut layers = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    let mut pieces = Vec::new();
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                pieces.push(&text[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    pieces.push(&text[start..]);
    for piece in pieces {
        let piece = piece.trim();
        let (inset, piece) = match piece.strip_prefix("inset ") {
            Some(rest) => (true, rest.trim()),
            None => (false, piece),
        };
        let colour_at = piece.find("rgba(").or_else(|| piece.find('#'))?;
        let colour = Colour::parse(&piece[colour_at..])?;
        let lengths: Vec<f32> = piece[..colour_at]
            .split_whitespace()
            .map(|l| l.trim_end_matches("px").parse::<f32>().ok())
            .collect::<Option<_>>()?;
        let at = |i: usize| lengths.get(i).copied().unwrap_or(0.0);
        if lengths.len() < 2 {
            return None;
        }
        layers.push(Shadow {
            x: at(0),
            y: at(1),
            blur: at(2),
            spread: at(3),
            colour,
            inset,
        });
    }
    Some(layers)
}

/// What a resolved token is, from its key and value (`schema.md`, Units).
#[allow(dead_code, reason = "the build script types tokens with it")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Colour,
    Shadows,
    Millis,
    Count,
    Number,
    Text,
}

#[allow(dead_code, reason = "the build script types tokens with it")]
pub fn kind_of(key: &str, value: &Raw) -> Kind {
    match value {
        Raw::Text(_) if key.starts_with("shadow-") => Kind::Shadows,
        Raw::Text(text) if Colour::parse(text).is_some() => Kind::Colour,
        Raw::Text(_) => Kind::Text,
        Raw::Number(_) if key.ends_with("-ms") => Kind::Millis,
        Raw::Number(_) if key.starts_with("limit-") || key == "version" => Kind::Count,
        Raw::Number(_) => Kind::Number,
    }
}
