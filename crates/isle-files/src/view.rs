//! How a folder is shown: the view, its sort and grouping and its icon size, remembered per
//! folder with a default for the rest (`p1/structure/files.md` § The content: views).

use isle_fs::{Entry, SortBy};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    Icons,
    #[default]
    List,
    Columns,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GroupBy {
    Kind,
    Modified,
    Size,
}

/// Icons' sizes on ⌘+ and ⌘−.
pub const ICON_STEPS: [u32; 8] = [16, 32, 48, 64, 96, 128, 192, 256];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shown {
    pub view: View,
    pub sort: SortBy,
    pub reverse: bool,
    pub group: Option<GroupBy>,
    pub icon_size: u32,
    /// A line of facts under each icon's name.
    pub item_info: bool,
}

impl Default for Shown {
    fn default() -> Self {
        Self {
            view: View::List,
            sort: SortBy::Name,
            reverse: false,
            group: None,
            icon_size: 64,
            item_info: false,
        }
    }
}

impl Shown {
    pub fn order(&self) -> isle_fs::Order {
        isle_fs::Order {
            by: self.sort,
            reverse: self.reverse,
            folders_first: true,
        }
    }

    /// The next icon size up or down the steps.
    pub fn step_icons(&mut self, up: bool) {
        let at = ICON_STEPS.iter().position(|s| *s >= self.icon_size).unwrap_or(3);
        let to = if up { (at + 1).min(ICON_STEPS.len() - 1) } else { at.saturating_sub(1) };
        self.icon_size = ICON_STEPS[to];
    }
}

/// The group an item falls in, in the order groups are shown, with its heading.
pub fn group_of(by: GroupBy, entry: &Entry, now: i64) -> (u8, String) {
    match by {
        GroupBy::Kind => {
            let family = match entry.mime.as_deref() {
                Some(isle_apps::mime::DIRECTORY) => (0, "Folders"),
                Some(m) if m.starts_with("image/") => (2, "Images"),
                Some(m) if m.starts_with("video/") => (3, "Movies"),
                Some(m) if m.starts_with("audio/") => (4, "Music"),
                Some(m) if m.starts_with("text/") || m.contains("script") || m.contains("json") => (1, "Documents"),
                Some(m) if m.contains("zip") || m.contains("tar") || m.contains("compressed") || m.contains("7z") => {
                    (5, "Archives")
                }
                _ => (6, "Other"),
            };
            (family.0, family.1.to_owned())
        }
        GroupBy::Modified => {
            let age = now - entry.modified;
            let day = 86_400;
            match age {
                a if a < day => (0, "Today".into()),
                a if a < 2 * day => (1, "Yesterday".into()),
                a if a < 7 * day => (2, "Previous 7 Days".into()),
                a if a < 30 * day => (3, "Previous 30 Days".into()),
                a if a < 365 * day => (4, "This Year".into()),
                _ => (5, "Earlier".into()),
            }
        }
        GroupBy::Size => {
            if entry.is_dir() {
                return (5, "Folders".into());
            }
            match entry.size {
                s if s >= 1_000_000_000 => (0, "Larger than 1 GB".into()),
                s if s >= 100_000_000 => (1, "100 MB to 1 GB".into()),
                s if s >= 1_000_000 => (2, "1 MB to 100 MB".into()),
                s if s >= 1_000 => (3, "1 KB to 1 MB".into()),
                _ => (4, "Smaller than 1 KB".into()),
            }
        }
    }
}

/// A view's lines: group headings and the items under them, as indexes into the folder,
/// each knowing whether it is an odd one of its group for the stripes.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    Heading(String),
    Item { index: usize, odd: bool },
}

/// The folder's items as lines: in order, or gathered under headings, each group in the sort.
pub fn lines(entries: &[Entry], group: Option<GroupBy>, now: i64) -> Vec<Line> {
    let Some(by) = group else {
        return (0..entries.len()).map(|index| Line::Item { index, odd: index % 2 == 1 }).collect();
    };
    let mut groups: std::collections::BTreeMap<u8, (String, Vec<usize>)> = Default::default();
    for (i, e) in entries.iter().enumerate() {
        let (rank, name) = group_of(by, e, now);
        groups.entry(rank).or_insert_with(|| (name, Vec::new())).1.push(i);
    }
    groups
        .into_values()
        .flat_map(|(name, items)| {
            std::iter::once(Line::Heading(name)).chain(
                items
                    .into_iter()
                    .enumerate()
                    .map(|(n, index)| Line::Item { index, odd: n % 2 == 1 }),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_sizes_step_through_finders_sizes() {
        let mut shown = Shown::default();
        shown.step_icons(true);
        assert_eq!(shown.icon_size, 96);
        shown.icon_size = 16;
        shown.step_icons(false);
        assert_eq!(shown.icon_size, 16);
        shown.icon_size = 256;
        shown.step_icons(true);
        assert_eq!(shown.icon_size, 256);
    }
}
