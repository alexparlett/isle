//! The seven templates a provider's answer fills (`p1/surfaces/widgets.md` Providers'
//! widgets; B14).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Template {
    Text,
    Number,
    Gauge,
    Sparkline,
    Graph,
    List,
    Card,
}

/// A provider's answer, read for its template.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "template", rename_all = "lowercase")]
pub enum Data {
    Text { text: String },
    Number { value: String, unit: Option<String>, label: Option<String> },
    /// `value` from 0 to 1.
    Gauge { value: f32, label: Option<String>, text: Option<String> },
    Sparkline { values: Vec<f64>, value: Option<String> },
    Graph { series: Vec<Series> },
    List { items: Vec<Item> },
    Card { title: String, body: Option<String>, image: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Series {
    pub label: String,
    pub values: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub label: String,
    pub value: Option<String>,
    /// A Phosphor glyph's name.
    pub glyph: Option<String>,
}

/// The longest list or series a widget draws; the rest is dropped.
const MOST: usize = 64;

fn text(v: &serde_json::Value, key: &str) -> Option<String> {
    match v.get(key)? {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn numbers(v: &serde_json::Value) -> Option<Vec<f64>> {
    let list = v.as_array()?;
    Some(list.iter().filter_map(|n| n.as_f64()).take(MOST).collect())
}

/// Reads an answer for `template`: a JSON object, with what each template needs. A number
/// may come as a number or as text.
pub fn read(template: Template, answer: &str) -> Result<Data, String> {
    let v: serde_json::Value =
        serde_json::from_str(answer.trim()).map_err(|e| format!("The answer is not JSON: {e}"))?;
    if !v.is_object() {
        return Err("The answer is not a JSON object".into());
    }
    let need = |key: &str| text(&v, key).ok_or_else(|| format!("The answer has no “{key}”"));
    Ok(match template {
        Template::Text => Data::Text { text: need("text")? },
        Template::Number => Data::Number {
            value: need("value")?,
            unit: text(&v, "unit"),
            label: text(&v, "label"),
        },
        Template::Gauge => {
            let value = v.get("value").and_then(|n| n.as_f64()).ok_or("The answer has no number “value”")?;
            let (min, max) = (
                v.get("min").and_then(|n| n.as_f64()).unwrap_or(0.0),
                v.get("max").and_then(|n| n.as_f64()).unwrap_or(1.0),
            );
            Data::Gauge {
                value: (((value - min) / (max - min).max(f64::EPSILON)) as f32).clamp(0.0, 1.0),
                label: text(&v, "label"),
                text: text(&v, "text"),
            }
        }
        Template::Sparkline => Data::Sparkline {
            values: v.get("values").and_then(numbers).ok_or("The answer has no list “values”")?,
            value: text(&v, "value"),
        },
        Template::Graph => Data::Graph {
            series: v
                .get("series")
                .and_then(|s| s.as_array())
                .ok_or("The answer has no list “series”")?
                .iter()
                .take(4)
                .filter_map(|s| {
                    Some(Series {
                        label: text(s, "label").unwrap_or_default(),
                        values: s.get("values").and_then(numbers)?,
                    })
                })
                .collect(),
        },
        Template::List => Data::List {
            items: v
                .get("items")
                .and_then(|s| s.as_array())
                .ok_or("The answer has no list “items”")?
                .iter()
                .take(MOST)
                .filter_map(|i| {
                    Some(Item {
                        label: text(i, "label")?,
                        value: text(i, "value"),
                        glyph: text(i, "glyph"),
                    })
                })
                .collect(),
        },
        Template::Card => Data::Card {
            title: need("title")?,
            body: text(&v, "body"),
            image: text(&v, "image"),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_template_reads_its_answer() {
        assert_eq!(read(Template::Text, r#"{"text":"hi"}"#).unwrap(), Data::Text { text: "hi".into() });
        assert_eq!(
            read(Template::Number, r#"{"value": 42, "unit": "°"}"#).unwrap(),
            Data::Number { value: "42".into(), unit: Some("°".into()), label: None }
        );
        assert!(matches!(read(Template::Gauge, r#"{"value": 5, "max": 10}"#).unwrap(), Data::Gauge { value, .. } if value == 0.5));
        assert!(matches!(read(Template::Sparkline, r#"{"values":[1,2,3]}"#).unwrap(), Data::Sparkline { values, .. } if values.len() == 3));
        assert!(matches!(
            read(Template::List, r#"{"items":[{"label":"a","value":1},{"nolabel":true}]}"#).unwrap(),
            Data::List { items } if items.len() == 1 && items[0].value.as_deref() == Some("1")
        ));
        assert!(matches!(read(Template::Graph, r#"{"series":[{"label":"x","values":[1,2]}]}"#).unwrap(), Data::Graph { series } if series.len() == 1));
        assert!(matches!(read(Template::Card, r#"{"title":"T"}"#).unwrap(), Data::Card { .. }));
    }

    #[test]
    fn a_wrong_answer_says_what_is_wrong() {
        assert_eq!(read(Template::Text, r#"{"txt":"hi"}"#).unwrap_err(), "The answer has no “text”");
        assert!(read(Template::Text, "hello").unwrap_err().starts_with("The answer is not JSON"));
        assert_eq!(read(Template::Text, "[1]").unwrap_err(), "The answer is not a JSON object");
    }
}
