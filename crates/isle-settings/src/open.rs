//! A section whose keys the schema does not know: a widget provider's settings, named by
//! its manifest and checked by whoever reads them.

use std::collections::BTreeMap;

use kdl::KdlValue;

use crate::{Ctx, KeyInfo, Leaf, Section, Seg, Value, join, quote};

/// Any leaf with any value. A one-item list is kept as a string, since the file cannot tell
/// them apart.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Open(pub BTreeMap<String, Value>);

impl Open {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }
}

fn normal(value: Value) -> Value {
    match value {
        Value::List(mut list) if list.len() == 1 => Value::Str(list.remove(0)),
        other => other,
    }
}

fn to_kdl(value: &Value) -> String {
    match value {
        Value::Bool(b) => if *b { "#true" } else { "#false" }.into(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => crate::Field::to_kdl(f),
        Value::Str(s) => quote(s),
        Value::List(l) => l.iter().map(|s| quote(s)).collect::<Vec<_>>().join(" "),
    }
}

impl Section for Open {
    fn keys(prefix: &str, out: &mut Vec<KeyInfo>) {
        out.push(KeyInfo {
            key: join(prefix, "*"),
            kind: "any",
            default: String::new(),
            doc: "Any key, checked by what reads it.".into(),
            constraint: String::new(),
        });
    }

    fn parse(doc: &kdl::KdlDocument, ctx: &mut Ctx) -> Self {
        let mut out = BTreeMap::new();
        for node in doc.nodes() {
            let name = node.name().value().to_owned();
            if node.children().is_some() {
                ctx.error(node.span(), format!("`{name}` takes a value, not a block"));
                continue;
            }
            if out.contains_key(&name) {
                ctx.error(node.span(), format!("`{name}` is set twice"));
                continue;
            }
            let values: Vec<&KdlValue> = node
                .entries()
                .iter()
                .filter(|e| e.name().is_none())
                .map(|e| e.value())
                .collect();
            if values.len() != node.entries().len() {
                ctx.error(node.span(), format!("`{name}` takes values, not properties"));
                continue;
            }
            let value = match values.as_slice() {
                [] => Value::List(Vec::new()),
                [KdlValue::Bool(b)] => Value::Bool(*b),
                [KdlValue::Integer(i)] => Value::Int(*i as i64),
                [KdlValue::Float(f)] => Value::Float(*f),
                [KdlValue::String(s)] => Value::Str(s.clone()),
                many if many.iter().all(|v| v.is_string()) => Value::List(
                    many.iter().map(|v| v.as_string().unwrap().to_owned()).collect(),
                ),
                _ => {
                    ctx.error(node.span(), format!("`{name}` is one value or a list of strings"));
                    continue;
                }
            };
            out.insert(name, value);
        }
        Open(out)
    }

    fn flatten(&self, prefix: &str, out: &mut BTreeMap<String, Value>) {
        for (key, value) in &self.0 {
            out.insert(join(prefix, key), value.clone());
        }
    }

    fn set(&mut self, path: &[Seg], value: Value) -> Result<(), String> {
        match path {
            [seg] if seg.key.is_none() => {
                self.0.insert(seg.name.clone(), normal(value));
                Ok(())
            }
            _ => Err("is not a setting".into()),
        }
    }

    fn kdl_args(&self, path: &[Seg]) -> Option<Leaf> {
        match path {
            [seg] if seg.key.is_none() => Some(match self.0.get(&seg.name) {
                Some(value) => Leaf::Args(to_kdl(value)),
                None => Leaf::Unset,
            }),
            _ => None,
        }
    }
}
