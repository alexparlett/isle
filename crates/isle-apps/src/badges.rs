//! An app's badge, progress and urgency from `com.canonical.Unity.LauncherEntry`'s Update
//! signal, kept per desktop entry (`platform.md` Notifications and badges 6).

/// One property of an Update, as its D-Bus type gives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
}

/// What one app has said through LauncherEntry; each Update changes only what it names.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LauncherEntry {
    pub count: i64,
    pub count_visible: bool,
    pub progress: f64,
    pub progress_visible: bool,
    pub urgent: bool,
    /// The bus name that sent it, whose leaving clears it.
    pub sender: String,
}

impl LauncherEntry {
    pub fn merge(&mut self, properties: &[(String, Value)]) {
        for (name, value) in properties {
            match (name.as_str(), *value) {
                ("count", Value::Int(n)) => self.count = n,
                ("count", Value::Float(n)) => self.count = n as i64,
                ("count-visible", Value::Bool(b)) => self.count_visible = b,
                ("progress", Value::Float(p)) => self.progress = p,
                ("progress", Value::Int(p)) => self.progress = p as f64,
                ("progress-visible", Value::Bool(b)) => self.progress_visible = b,
                ("urgent", Value::Bool(b)) => self.urgent = b,
                _ => {}
            }
        }
    }

    /// The count to show, when it is visible and above zero.
    pub fn badge(&self) -> Option<i64> {
        (self.count_visible && self.count > 0).then_some(self.count)
    }

    /// The progress to show, from 0 to 1, when it is visible.
    pub fn progress(&self) -> Option<f64> {
        self.progress_visible
            .then_some(self.progress.clamp(0.0, 1.0))
    }
}

/// The desktop id an Update's `app_uri` names: `application://firefox.desktop` is `firefox`.
pub fn desktop_id(app_uri: &str) -> Option<&str> {
    let id = app_uri.strip_prefix("application://")?;
    let id = id.strip_suffix(".desktop").unwrap_or(id);
    (!id.is_empty()).then_some(id)
}

/// A badge's text: the count, or "999+" past three digits.
pub fn badge_text(count: i64) -> String {
    if count > 999 {
        "999+".into()
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updates_merge_and_show_only_what_is_visible() {
        let mut entry = LauncherEntry::default();
        entry.merge(&[("count".into(), Value::Int(4))]);
        assert_eq!(entry.badge(), None, "not visible yet");
        entry.merge(&[("count-visible".into(), Value::Bool(true))]);
        assert_eq!(entry.badge(), Some(4));
        entry.merge(&[
            ("progress".into(), Value::Float(1.4)),
            ("progress-visible".into(), Value::Bool(true)),
        ]);
        assert_eq!(entry.badge(), Some(4), "a partial update keeps the count");
        assert_eq!(entry.progress(), Some(1.0));
        entry.merge(&[("count".into(), Value::Int(0))]);
        assert_eq!(entry.badge(), None);
    }

    #[test]
    fn app_uris_and_badge_text() {
        assert_eq!(desktop_id("application://firefox.desktop"), Some("firefox"));
        assert_eq!(
            desktop_id("application://org.gnome.Nautilus"),
            Some("org.gnome.Nautilus")
        );
        assert_eq!(desktop_id("firefox.desktop"), None);
        assert_eq!(badge_text(12), "12");
        assert_eq!(badge_text(1000), "999+");
    }
}
