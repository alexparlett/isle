//! Setting values, and the field types a schema is made of.

use kdl::{KdlNode, KdlValue};
use serde::{Deserialize, Serialize};

use crate::Ctx;

/// A setting's value as the D-Bus interface and the CLI carry it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<String>),
}

impl Value {
    /// Reads a value written as KDL, as the CLI takes it: `true`, `1.25`, `"us"` or `us`.
    pub fn parse_kdl(text: &str) -> Value {
        let doc = format!("v {text}");
        match doc.parse::<kdl::KdlDocument>() {
            Ok(doc) => {
                let entries: Vec<_> = doc.nodes()[0]
                    .entries()
                    .iter()
                    .map(|e| e.value().clone())
                    .collect();
                match entries.as_slice() {
                    [one] => Value::from_kdl(one),
                    many if many.iter().all(|v| v.is_string()) => Value::List(
                        many.iter()
                            .map(|v| v.as_string().unwrap().to_owned())
                            .collect(),
                    ),
                    _ => Value::Str(text.to_owned()),
                }
            }
            Err(_) => Value::Str(text.to_owned()),
        }
    }

    fn from_kdl(value: &KdlValue) -> Value {
        match value {
            KdlValue::Bool(b) => Value::Bool(*b),
            KdlValue::Integer(i) => Value::Int(*i as i64),
            KdlValue::Float(f) => Value::Float(*f),
            KdlValue::String(s) => Value::Str(s.clone()),
            KdlValue::Null => Value::Str("null".into()),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::List(l) => write!(f, "{}", l.join(" ")),
        }
    }
}

/// A type a setting can hold. A leaf setting is a node whose arguments are its value.
pub trait Field: Sized + Clone + PartialEq {
    const KIND: &'static str;
    fn from_node(node: &KdlNode, ctx: &mut Ctx) -> Option<Self>;
    fn from_value(value: Value) -> Result<Self, String>;
    fn to_value(&self) -> Value;
    /// The node's arguments as KDL text.
    fn to_kdl(&self) -> String;
    fn choices() -> &'static [&'static str] {
        &[]
    }
    /// The number a range is checked against.
    fn number(&self) -> Option<f64> {
        None
    }
    /// An optional setting left to the compositor: no node in the file, no value.
    fn is_unset(&self) -> bool {
        false
    }
}

/// The single argument of `node`, or an error.
pub fn single<'a>(node: &'a KdlNode, ctx: &mut Ctx) -> Option<&'a KdlValue> {
    match node.entries() {
        [entry] if entry.name().is_none() => Some(entry.value()),
        _ => {
            ctx.error(
                node.span(),
                format!("`{}` takes one value", node.name().value()),
            );
            None
        }
    }
}

impl Field for bool {
    const KIND: &'static str = "bool";
    fn from_node(node: &KdlNode, ctx: &mut Ctx) -> Option<Self> {
        match single(node, ctx)? {
            KdlValue::Bool(b) => Some(*b),
            _ => {
                ctx.error(
                    node.span(),
                    format!("`{}` is #true or #false", node.name().value()),
                );
                None
            }
        }
    }
    fn from_value(value: Value) -> Result<Self, String> {
        match value {
            Value::Bool(b) => Ok(b),
            Value::Str(s) if s == "true" || s == "false" => Ok(s == "true"),
            other => Err(format!("expected #true or #false, not {other}")),
        }
    }
    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }
    fn to_kdl(&self) -> String {
        if *self { "#true" } else { "#false" }.into()
    }
}

macro_rules! integer {
    ($t:ty) => {
        impl Field for $t {
            const KIND: &'static str = "integer";
            fn from_node(node: &KdlNode, ctx: &mut Ctx) -> Option<Self> {
                match single(node, ctx)? {
                    KdlValue::Integer(i) => match <$t>::try_from(*i) {
                        Ok(v) => Some(v),
                        Err(_) => {
                            ctx.error(
                                node.span(),
                                format!("`{}` is out of range", node.name().value()),
                            );
                            None
                        }
                    },
                    _ => {
                        ctx.error(
                            node.span(),
                            format!("`{}` is a whole number", node.name().value()),
                        );
                        None
                    }
                }
            }
            fn from_value(value: Value) -> Result<Self, String> {
                match value {
                    Value::Int(i) => <$t>::try_from(i).map_err(|_| format!("{i} is out of range")),
                    other => Err(format!("expected a whole number, not {other}")),
                }
            }
            fn to_value(&self) -> Value {
                Value::Int(*self as i64)
            }
            fn to_kdl(&self) -> String {
                self.to_string()
            }
            fn number(&self) -> Option<f64> {
                Some(*self as f64)
            }
        }
    };
}
integer!(i32);
integer!(u32);
integer!(i64);

impl Field for f64 {
    const KIND: &'static str = "number";
    fn from_node(node: &KdlNode, ctx: &mut Ctx) -> Option<Self> {
        match single(node, ctx)? {
            KdlValue::Float(f) => Some(*f),
            KdlValue::Integer(i) => Some(*i as f64),
            _ => {
                ctx.error(
                    node.span(),
                    format!("`{}` is a number", node.name().value()),
                );
                None
            }
        }
    }
    fn from_value(value: Value) -> Result<Self, String> {
        match value {
            Value::Float(f) => Ok(f),
            Value::Int(i) => Ok(i as f64),
            other => Err(format!("expected a number, not {other}")),
        }
    }
    fn to_value(&self) -> Value {
        Value::Float(*self)
    }
    fn to_kdl(&self) -> String {
        let text = self.to_string();
        if text.contains(['.', 'e', 'E']) || !self.is_finite() {
            text
        } else {
            format!("{text}.0")
        }
    }
    fn number(&self) -> Option<f64> {
        Some(*self)
    }
}

impl Field for String {
    const KIND: &'static str = "string";
    fn from_node(node: &KdlNode, ctx: &mut Ctx) -> Option<Self> {
        match single(node, ctx)? {
            KdlValue::String(s) => Some(s.clone()),
            _ => {
                ctx.error(
                    node.span(),
                    format!("`{}` is a string", node.name().value()),
                );
                None
            }
        }
    }
    fn from_value(value: Value) -> Result<Self, String> {
        match value {
            Value::Str(s) => Ok(s),
            other => Err(format!("expected a string, not {other}")),
        }
    }
    fn to_value(&self) -> Value {
        Value::Str(self.clone())
    }
    fn to_kdl(&self) -> String {
        quote(self)
    }
}

impl Field for Vec<String> {
    const KIND: &'static str = "list";
    fn from_node(node: &KdlNode, ctx: &mut Ctx) -> Option<Self> {
        let mut list = Vec::new();
        for entry in node.entries() {
            match entry.value() {
                KdlValue::String(s) if entry.name().is_none() => list.push(s.clone()),
                _ => {
                    ctx.error(
                        entry.span(),
                        format!("`{}` is a list of strings", node.name().value()),
                    );
                    return None;
                }
            }
        }
        Some(list)
    }
    fn from_value(value: Value) -> Result<Self, String> {
        match value {
            Value::List(l) => Ok(l),
            Value::Str(s) => Ok(vec![s]),
            other => Err(format!("expected a list of strings, not {other}")),
        }
    }
    fn to_value(&self) -> Value {
        Value::List(self.clone())
    }
    fn to_kdl(&self) -> String {
        self.iter().map(|s| quote(s)).collect::<Vec<_>>().join(" ")
    }
}

/// A setting that may be absent; absent means the compositor decides.
impl<T: Field> Field for Option<T> {
    const KIND: &'static str = T::KIND;
    fn from_node(node: &KdlNode, ctx: &mut Ctx) -> Option<Self> {
        T::from_node(node, ctx).map(Some)
    }
    fn from_value(value: Value) -> Result<Self, String> {
        T::from_value(value).map(Some)
    }
    fn to_value(&self) -> Value {
        match self {
            Some(v) => v.to_value(),
            None => Value::Str(String::new()),
        }
    }
    fn to_kdl(&self) -> String {
        self.as_ref().map(T::to_kdl).unwrap_or_default()
    }
    fn choices() -> &'static [&'static str] {
        T::choices()
    }
    fn number(&self) -> Option<f64> {
        self.as_ref().and_then(T::number)
    }
    fn is_unset(&self) -> bool {
        self.is_none()
    }
}

/// A KDL string literal.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Parses a choice's single string argument; used by `#[derive(Choice)]`.
pub fn choice_from_node(node: &KdlNode, ctx: &mut Ctx, choices: &[&str]) -> Option<String> {
    match single(node, ctx)? {
        KdlValue::String(s) if choices.contains(&s.as_str()) => Some(s.clone()),
        _ => {
            ctx.error(
                node.span(),
                format!("`{}` is one of {}", node.name().value(), choices.join(", ")),
            );
            None
        }
    }
}
