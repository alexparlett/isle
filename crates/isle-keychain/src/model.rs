//! What the window lists: items of every source, the sidebar's rows and their counts, and
//! search (`p19c-keychain.md` choices 7, 21 and 22). No item holds a secret.

use std::{collections::HashMap, path::PathBuf};

use isle_network::SavedWifi;
use isle_secrets::{kinds::Kind, ssh::SshKey};

/// An item by where it lives.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemId {
    /// A Secret Service item by its object path.
    Secret(String),
    /// A saved Wi-Fi network by its settings path.
    Wifi(String),
    /// An SSH key by its name in `~/.ssh`.
    Ssh(String),
}

/// What the detail shows beyond the title.
#[derive(Clone, Debug, PartialEq)]
pub enum Detail {
    Secret {
        user: Option<String>,
        website: Option<String>,
        notes: Option<String>,
        /// The storing app's name and icon, for an Apps item.
        stored_by: Option<String>,
        icon: Option<PathBuf>,
        storage_key: bool,
        created: u64,
        modified: u64,
        attributes: HashMap<String, String>,
    },
    Wifi {
        saved: SavedWifi,
        /// The keyring item an agent-owned profile's password is in.
        agent_item: Option<String>,
    },
    Ssh {
        key: SshKey,
        in_agent: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: ItemId,
    pub kind: Kind,
    pub title: String,
    /// The second line: a user name or the item's main attribute.
    pub second: String,
    /// The source's name.
    pub source: String,
    /// The collection's path, for a Secret Service item.
    pub collection: Option<String>,
    pub detail: Detail,
}

impl Item {
    pub fn user(&self) -> Option<&str> {
        match &self.detail {
            Detail::Secret { user, .. } => user.as_deref().filter(|u| !u.is_empty()),
            Detail::Wifi { saved, .. } => saved.identity.as_deref(),
            Detail::Ssh { .. } => None,
        }
    }

    /// Whether the item has a secret to show or copy.
    pub fn has_secret(&self) -> bool {
        match &self.detail {
            Detail::Secret { .. } => true,
            Detail::Wifi { saved, .. } => {
                matches!(
                    saved.keeps,
                    isle_network::Keeps::System | isle_network::Keeps::Agent
                )
            }
            Detail::Ssh { .. } => false,
        }
    }

    pub fn editable(&self) -> bool {
        matches!(self.detail, Detail::Secret { .. })
    }

    /// The words search reads: title, user name, website or server, the storing app and the
    /// source, never a secret.
    fn words(&self) -> Vec<&str> {
        let mut out = vec![
            self.title.as_str(),
            self.second.as_str(),
            self.source.as_str(),
        ];
        if let Detail::Secret {
            user,
            website,
            stored_by,
            ..
        } = &self.detail
        {
            out.extend(user.as_deref());
            out.extend(website.as_deref());
            out.extend(stored_by.as_deref());
        }
        if let Detail::Wifi { saved, .. } = &self.detail {
            out.push(saved.ssid.as_str());
        }
        out
    }
}

/// A Secret Service collection as the sidebar shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub path: String,
    pub label: String,
    pub locked: bool,
    pub default: bool,
}

/// How the Secret Service answered.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Keyring {
    #[default]
    Loading,
    Ready,
    /// Nothing owns the Secret Service's name.
    Missing,
    /// It did not answer in time, or failed: why.
    Failed(String),
}

/// Everything listed, as the store last read it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub keyring: Keyring,
    pub sources: Vec<Source>,
    pub items: Vec<Item>,
    /// Whether the store has read every source once.
    pub loaded: bool,
    /// The keyring offers gnome-keyring's change-password call.
    pub can_change_password: bool,
}

/// A sidebar row.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Row {
    All,
    Logins,
    Wifi,
    SshKeys,
    Apps,
    /// A collection by its path.
    Source(String),
}

impl Default for Row {
    fn default() -> Self {
        Row::All
    }
}

impl Row {
    pub const KINDS: [Row; 5] = [Row::All, Row::Logins, Row::Wifi, Row::SshKeys, Row::Apps];

    pub fn kind(&self) -> Option<Kind> {
        match self {
            Row::Logins => Some(Kind::Login),
            Row::Wifi => Some(Kind::WiFi),
            Row::SshKeys => Some(Kind::SshKey),
            Row::Apps => Some(Kind::App),
            _ => None,
        }
    }

    pub fn title(&self, snapshot: &Snapshot) -> String {
        match self {
            Row::All => "All".into(),
            Row::Logins => "Logins".into(),
            Row::Wifi => "Wi-Fi".into(),
            Row::SshKeys => "SSH keys".into(),
            Row::Apps => "Apps".into(),
            Row::Source(path) => snapshot
                .sources
                .iter()
                .find(|s| &s.path == path)
                .map_or_else(|| "Keyring".into(), |s| s.label.clone()),
        }
    }

    /// The Phosphor glyph and pane colour name of its tile (`surfaces/keychain.md`).
    pub fn tile(&self) -> (&'static str, &'static str) {
        match self {
            Row::All => ("key", "blue"),
            Row::Logins => ("globe", "indigo"),
            Row::Wifi => ("wifi-high", "teal"),
            Row::SshKeys => ("terminal-window", "graphite"),
            Row::Apps => ("app-window", "purple"),
            Row::Source(_) => ("vault", "grey"),
        }
    }

    pub fn holds(&self, item: &Item) -> bool {
        match self {
            Row::All => true,
            Row::Source(path) => item.collection.as_deref() == Some(path.as_str()),
            kind => kind.kind() == Some(item.kind),
        }
    }

    /// Whether the row's list takes items from the keyring, so a locked or missing keyring
    /// heads it.
    pub fn spans_keyring(&self) -> bool {
        !matches!(self, Row::Wifi | Row::SshKeys)
    }

    /// Whether the list spans more than one source, so its rows name theirs.
    pub fn spans_sources(&self) -> bool {
        !matches!(self, Row::Source(_) | Row::Wifi | Row::SshKeys)
    }

    /// What New makes here: a login, an SSH key, or nothing.
    pub fn creates(&self) -> Option<Kind> {
        match self {
            Row::All | Row::Logins | Row::Source(_) => Some(Kind::Login),
            Row::SshKeys => Some(Kind::SshKey),
            _ => None,
        }
    }

    pub fn empty_line(&self) -> &'static str {
        match self {
            Row::Logins => "No saved logins",
            Row::Wifi => "No saved Wi-Fi networks",
            Row::SshKeys => "No SSH keys",
            Row::Apps => "No app secrets",
            _ => "No items",
        }
    }
}

/// Items a row counts: kinds count every unlocked source, a source its own.
pub fn count(snapshot: &Snapshot, row: &Row) -> usize {
    snapshot.items.iter().filter(|i| row.holds(i)).count()
}

/// How well `query` matches an item's words: a word's start before any part, none for no match.
fn rank(item: &Item, query: &str) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }
    let words = item.words();
    let mut best = None;
    for text in words {
        let lower = text.to_lowercase();
        let starts = lower.starts_with(query)
            || lower
                .match_indices(query)
                .any(|(i, _)| lower[..i].ends_with(|c: char| !c.is_alphanumeric()));
        if starts {
            return Some(0);
        }
        if lower.contains(query) {
            best = Some(1);
        }
    }
    best
}

/// The row's items matching `query`, word starts first, then by title.
pub fn listed<'a>(snapshot: &'a Snapshot, row: &Row, query: &str) -> Vec<&'a Item> {
    let query = query.trim().to_lowercase();
    let mut out: Vec<(u8, &Item)> = snapshot
        .items
        .iter()
        .filter(|i| row.holds(i))
        .filter_map(|i| rank(i, &query).map(|r| (r, i)))
        .collect();
    out.sort_by(|(ra, a), (rb, b)| {
        ra.cmp(rb)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
    });
    out.into_iter().map(|(_, i)| i).collect()
}

/// The locked collections a row's list spans.
pub fn locked_in<'a>(snapshot: &'a Snapshot, row: &Row) -> Vec<&'a Source> {
    snapshot
        .sources
        .iter()
        .filter(|s| s.locked)
        .filter(|s| match row {
            Row::Source(p) => *p == s.path,
            r => r.spans_keyring(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn login(title: &str, user: &str, site: &str) -> Item {
        Item {
            id: ItemId::Secret(format!("/s/{title}")),
            kind: Kind::Login,
            title: title.into(),
            second: user.into(),
            source: "Login".into(),
            collection: Some("/c/login".into()),
            detail: Detail::Secret {
                user: Some(user.into()),
                website: Some(site.into()),
                notes: None,
                stored_by: None,
                icon: None,
                storage_key: false,
                created: 0,
                modified: 0,
                attributes: HashMap::new(),
            },
        }
    }

    #[test]
    fn search_reads_words_never_secrets_and_ranks_word_starts_first() {
        let snapshot = Snapshot {
            items: vec![
                login("Zebra", "hubert", "zebra.example"),
                login("GitHub", "alex", "github.com"),
                login("Grafana", "admin", "grafana.lan"),
            ],
            ..Snapshot::default()
        };
        let titles = |q: &str| -> Vec<String> {
            listed(&snapshot, &Row::All, q)
                .iter()
                .map(|i| i.title.clone())
                .collect()
        };
        assert_eq!(titles(""), ["GitHub", "Grafana", "Zebra"]);
        assert_eq!(
            titles("hub"),
            ["Zebra", "GitHub"],
            "a word's start ranks first"
        );
        assert_eq!(titles("lan"), ["Grafana"]);
        assert_eq!(titles("admin"), ["Grafana"]);
        assert!(titles("hunter2").is_empty());
    }

    #[test]
    fn rows_hold_their_kinds_and_sources() {
        let mut wifi = login("Home", "", "");
        wifi.kind = Kind::WiFi;
        wifi.collection = None;
        let snapshot = Snapshot {
            items: vec![login("GitHub", "a", "github.com"), wifi],
            ..Snapshot::default()
        };
        assert_eq!(count(&snapshot, &Row::All), 2);
        assert_eq!(count(&snapshot, &Row::Logins), 1);
        assert_eq!(count(&snapshot, &Row::Wifi), 1);
        assert_eq!(count(&snapshot, &Row::Source("/c/login".into())), 1);
        assert_eq!(Row::Wifi.creates(), None);
        assert_eq!(Row::SshKeys.creates(), Some(Kind::SshKey));
    }
}
