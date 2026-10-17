//! The settings store: the schema, `settings.kdl` read and written, validation (B10, B27).

// The derive names paths through `::isle_settings`, including here.
extern crate self as isle_settings;

mod path;
pub mod schema;
mod shortcuts;
mod store;
mod text;
mod value;

use std::collections::BTreeMap;

pub use isle_settings_derive::{Choice, Section};
pub use kdl;
pub use path::{Seg, format_path, parse_path};
pub use store::{FileError, Reload, SetError, Store};
pub use value::{Field, Value, choice_from_node, quote};

/// One key the schema knows, for introspection.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct KeyInfo {
    /// The key's path; a keyed node's key is written `[*]`.
    pub key: String,
    pub kind: &'static str,
    /// The default as KDL text; empty when the compositor decides.
    pub default: String,
    pub doc: String,
    /// The range or the choices, as text.
    pub constraint: String,
}

/// A problem found while reading a file, at a byte span of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub offset: usize,
}

/// What reading a document collects.
pub struct Ctx<'a> {
    pub source: &'a str,
    pub errors: Vec<Diagnostic>,
}

impl<'a> Ctx<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            errors: Vec::new(),
        }
    }

    pub fn error(&mut self, span: miette::SourceSpan, message: impl Into<String>) {
        self.errors.push(Diagnostic {
            message: message.into(),
            offset: span.offset(),
        });
    }
}

/// A block of settings: the root, a section such as `keyboard { }`, or the body of a
/// keyed node such as `output "AOC U34G2G4R3 0" { }`. Implemented by `#[derive(Section)]`.
pub trait Section: Sized + Clone + PartialEq + Default {
    fn keys(prefix: &str, out: &mut Vec<KeyInfo>);
    fn parse(doc: &kdl::KdlDocument, ctx: &mut Ctx) -> Self;
    fn flatten(&self, prefix: &str, out: &mut BTreeMap<String, Value>);
    /// Validates `value` for the key at `path` and sets it.
    fn set(&mut self, path: &[Seg], value: Value) -> Result<(), String>;
    /// What the leaf at `path` writes into the file.
    fn kdl_args(&self, path: &[Seg]) -> Option<Leaf>;
    /// Rules across keys, run after parsing and before a write.
    fn check(&self) -> Result<(), CheckError> {
        Ok(())
    }
}

/// A leaf as the file holds it.
#[derive(Clone, Debug, PartialEq)]
pub enum Leaf {
    /// The node's arguments as KDL text; empty for an empty list, a bare node.
    Args(String),
    /// An optional setting left unset: no node at all.
    Unset,
}

/// A cross-field rule broken, at the key that breaks it.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckError {
    pub message: String,
    /// The key's path, `shortcuts.hide`, for the place in the file.
    pub key: String,
}

/// Joins a path prefix and a node name.
pub fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}.{name}")
    }
}

/// Every key of `S`, for the D-Bus Schema method.
pub fn schema_of<S: Section>() -> Vec<KeyInfo> {
    let mut out = Vec::new();
    S::keys("", &mut out);
    out
}

/// The 1-based line and column of a byte offset.
pub fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rfind('\n')
        .map_or(before.len(), |n| before.len() - n - 1)
        + 1;
    (line, column)
}

/// The value type of a keyed field's map, for `#[derive(Section)]`.
pub trait KeyedMap {
    type Value: Section;
}

impl<V: Section> KeyedMap for BTreeMap<String, V> {
    type Value = V;
}

/// The key of a keyed node such as `output "AOC U34G2G4R3 0" { }`: its one string argument.
pub fn node_key(node: &kdl::KdlNode, ctx: &mut Ctx) -> Option<String> {
    match node.entries() {
        [entry] if entry.name().is_none() => match entry.value().as_string() {
            Some(key) => Some(key.to_owned()),
            None => {
                ctx.error(
                    entry.span(),
                    format!("`{}` is named by a string", node.name().value()),
                );
                None
            }
        },
        _ => {
            ctx.error(
                node.span(),
                format!(
                    "`{}` takes one name, as in `{} \"name\" {{ }}`",
                    node.name().value(),
                    node.name().value()
                ),
            );
            None
        }
    }
}
