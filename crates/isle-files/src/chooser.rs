//! A FileChooser request as the portal hands it to Files, and its answer (`p15-files.md`
//! choices 21–23; the portal's `org.freedesktop.impl.portal.FileChooser`).

use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use zvariant::{OwnedValue, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Open,
    Save,
    /// One folder for several files the app names.
    SaveFiles,
}

impl Kind {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "open" => Some(Kind::Open),
            "save" => Some(Kind::Save),
            "save-files" => Some(Kind::SaveFiles),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rule {
    Glob(String),
    Mime(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filter {
    pub name: String,
    pub rules: Vec<Rule>,
}

impl Filter {
    /// Whether a file of this name and type passes; a folder always does.
    pub fn passes(&self, name: &str, mime: Option<&str>, dir: bool) -> bool {
        dir || self.rules.iter().any(|rule| match rule {
            Rule::Glob(glob) => isle_apps::mime::matches_glob(glob, name),
            Rule::Mime(want) => mime.is_some_and(|m| m == want || isle_apps::mime::types().is_a(m, want)),
        })
    }

    /// The extension its first glob names, as `*.png` names `png`.
    pub fn extension(&self) -> Option<&str> {
        self.rules.iter().find_map(|r| match r {
            Rule::Glob(g) => g.strip_prefix("*.").filter(|e| !e.contains(['*', '?', '['])),
            Rule::Mime(_) => None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub label: String,
    /// A pop-up's options by id and label; none makes it a checkbox.
    pub options: Vec<(String, String)>,
    pub initial: String,
}

/// What the asking app wants.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ask {
    pub id: u64,
    pub kind: Kind,
    pub app_id: String,
    /// `wayland:<handle>`, or empty for none.
    pub parent: String,
    pub title: String,
    pub accept_label: Option<String>,
    pub multiple: bool,
    pub directory: bool,
    pub filters: Vec<Filter>,
    /// The filter chosen at first, an index into `filters`.
    pub current_filter: Option<usize>,
    pub choices: Vec<Choice>,
    pub current_folder: Option<PathBuf>,
    pub current_name: Option<String>,
    /// The names SaveFiles writes.
    pub files: Vec<String>,
}

/// What Files answers: the response code and the results.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Answer {
    pub response: u32,
    pub uris: Vec<String>,
    pub choices: Vec<(String, String)>,
    pub filter: Option<Filter>,
}

pub const SUCCESS: u32 = 0;
pub const CANCELLED: u32 = 1;

impl Answer {
    pub fn cancelled() -> Self {
        Answer { response: CANCELLED, ..Default::default() }
    }

    /// The portal's results: `uris`, `choices`, `current_filter` and `writable`.
    pub fn results(&self) -> HashMap<String, OwnedValue> {
        let mut out = HashMap::new();
        if self.response != SUCCESS {
            return out;
        }
        let owned = |v: Value<'_>| v.try_to_owned().expect("no file descriptors");
        out.insert("uris".into(), owned(Value::from(self.uris.clone())));
        out.insert("choices".into(), owned(Value::from(self.choices.clone())));
        if let Some(filter) = &self.filter {
            out.insert("current_filter".into(), owned(Value::from(filter_tuple(filter))));
        }
        out.insert("writable".into(), owned(Value::from(true)));
        out
    }
}

type FilterTuple = (String, Vec<(u32, String)>);

fn filter_tuple(f: &Filter) -> FilterTuple {
    let rules = f
        .rules
        .iter()
        .map(|r| match r {
            Rule::Glob(g) => (0u32, g.clone()),
            Rule::Mime(m) => (1u32, m.clone()),
        })
        .collect();
    (f.name.clone(), rules)
}

fn filter_of((name, rules): FilterTuple) -> Filter {
    Filter {
        name,
        rules: rules
            .into_iter()
            .map(|(kind, text)| if kind == 1 { Rule::Mime(text) } else { Rule::Glob(text) })
            .collect(),
    }
}

/// A byte path as the portal sends it, NUL-terminated.
fn byte_path(bytes: Vec<u8>) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    let bytes: Vec<u8> = bytes.into_iter().take_while(|b| *b != 0).collect();
    (!bytes.is_empty()).then(|| PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

/// Reads the portal's options for a request.
pub fn ask_of(
    id: u64,
    kind: Kind,
    app_id: &str,
    parent: &str,
    title: &str,
    options: &HashMap<String, OwnedValue>,
) -> Ask {
    fn get<T: TryFrom<OwnedValue>>(options: &HashMap<String, OwnedValue>, key: &str) -> Option<T> {
        options.get(key).and_then(|v| v.try_clone().ok()).and_then(|v| T::try_from(v).ok())
    }
    let filters: Vec<Filter> = get::<Vec<FilterTuple>>(options, "filters").unwrap_or_default().into_iter().map(filter_of).collect();
    let mut filters = filters;
    // A current filter not among the filters joins them, as the portal allows.
    let current_filter = get::<FilterTuple>(options, "current_filter").map(filter_of).map(|f| {
        match filters.iter().position(|g| *g == f) {
            Some(i) => i,
            None => {
                filters.push(f);
                filters.len() - 1
            }
        }
    });
    let choices = get::<Vec<(String, String, Vec<(String, String)>, String)>>(options, "choices")
        .unwrap_or_default()
        .into_iter()
        .map(|(id, label, options, initial)| Choice { id, label, options, initial })
        .collect();
    let current_file = get::<Vec<u8>>(options, "current_file").and_then(byte_path);
    let current_folder = get::<Vec<u8>>(options, "current_folder")
        .and_then(byte_path)
        .or_else(|| current_file.as_ref().and_then(|f| f.parent().map(PathBuf::from)));
    let current_name = get::<String>(options, "current_name")
        .or_else(|| current_file.as_ref().and_then(|f| f.file_name()).map(|n| n.to_string_lossy().into_owned()));
    let files = get::<Vec<Vec<u8>>>(options, "files")
        .unwrap_or_default()
        .into_iter()
        .filter_map(byte_path)
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    Ask {
        id,
        kind,
        app_id: app_id.to_owned(),
        parent: parent.to_owned(),
        title: title.to_owned(),
        accept_label: get::<String>(options, "accept_label").filter(|l| !l.is_empty()),
        multiple: get::<bool>(options, "multiple").unwrap_or(false),
        directory: get::<bool>(options, "directory").unwrap_or(false),
        filters,
        current_filter,
        choices,
        current_folder,
        current_name,
        files,
    }
}

/// `name` with its extension changed to the filter's, as Save's Format does.
pub fn with_extension(name: &str, filter: &Filter) -> String {
    let Some(ext) = filter.extension() else { return name.to_owned() };
    let (stem, _) = isle_fs::jobs::split_ext(name);
    let stem = if stem.is_empty() { name } else { stem };
    format!("{stem}.{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned<'a>(v: impl Into<Value<'a>>) -> OwnedValue {
        v.into().try_to_owned().unwrap()
    }

    #[test]
    fn options_read_with_the_current_filter_joining_the_filters() {
        let mut options = HashMap::new();
        let images: FilterTuple = ("Images".into(), vec![(0, "*.png".into()), (1, "image/*".into())]);
        let text: FilterTuple = ("Text".into(), vec![(1, "text/plain".into())]);
        options.insert("filters".into(), owned(vec![images.clone()]));
        options.insert("current_filter".into(), owned(text));
        options.insert("multiple".into(), owned(true));
        options.insert("current_folder".into(), owned(b"/home/me/Pictures\0".to_vec()));
        options.insert(
            "choices".into(),
            owned(vec![("enc".to_owned(), "Encoding".to_owned(), vec![("utf8".to_owned(), "UTF-8".to_owned())], "utf8".to_owned())]),
        );
        let ask = ask_of(7, Kind::Open, "org.app", "wayland:abc", "Pick", &options);
        assert!(ask.multiple);
        assert_eq!(ask.filters.len(), 2);
        assert_eq!(ask.current_filter, Some(1));
        assert_eq!(ask.current_folder.as_deref(), Some(std::path::Path::new("/home/me/Pictures")));
        assert_eq!(ask.choices[0].options[0].1, "UTF-8");
    }

    #[test]
    fn filters_match_by_glob_and_by_type_with_wildcards() {
        let images = filter_of(("Images".into(), vec![(0, "*.PNG".into()), (1, "image/*".into())]));
        assert!(images.passes("shot.png", None, false));
        assert!(images.passes("photo.jpg", Some("image/jpeg"), false));
        assert!(!images.passes("notes.md", Some("text/markdown"), false));
        assert!(images.passes("src", Some("inode/directory"), true));
        assert_eq!(with_extension("Export.jpg", &filter_of(("PNG".into(), vec![(0, "*.png".into())]))), "Export.png");
    }

    #[test]
    fn an_answer_carries_uris_choices_and_writable() {
        let answer = Answer {
            response: SUCCESS,
            uris: vec!["file:///a".into()],
            choices: vec![("enc".into(), "utf8".into())],
            filter: None,
        };
        let results = answer.results();
        assert!(results.contains_key("uris") && results.contains_key("writable"));
        assert!(Answer::cancelled().results().is_empty());
    }
}
