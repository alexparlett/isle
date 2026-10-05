//! An app's help and website links from its AppStream metainfo (`menu-bar.md`, Help).

use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Links {
    pub help: Option<String>,
    pub homepage: Option<String>,
}

/// `/usr/share/metainfo` and its kin on the data path.
pub fn dirs() -> Vec<PathBuf> {
    let dirs = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    dirs.split(':')
        .filter(|d| !d.is_empty())
        .flat_map(|d| {
            [
                PathBuf::from(d).join("metainfo"),
                PathBuf::from(d).join("appdata"),
            ]
        })
        .collect()
}

/// The links for the app whose desktop id is `id`; blocking, so never on the main loop.
pub fn links(id: &str, dirs: &[PathBuf]) -> Links {
    let names = [format!("{id}.metainfo.xml"), format!("{id}.appdata.xml")];
    dirs.iter()
        .flat_map(|d| names.iter().map(move |n| d.join(n)))
        .find_map(|p| std::fs::read_to_string(p).ok())
        .map(|text| parse(&text))
        .unwrap_or_default()
}

/// The first `<url type="help">` and `<url type="homepage">`.
pub fn parse(text: &str) -> Links {
    let url = |kind: &str| {
        let open = format!("<url type=\"{kind}\">");
        let start = text.find(&open)? + open.len();
        let end = text[start..].find("</url>")? + start;
        Some(text[start..end].trim().to_owned()).filter(|u| !u.is_empty())
    };
    Links {
        help: url("help"),
        homepage: url("homepage"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_read_by_type() {
        let text = r#"<component><url type="homepage">https://example.org/</url>
            <url type="bugtracker">https://bugs</url><url type="help"> https://example.org/help </url></component>"#;
        assert_eq!(
            parse(text),
            Links {
                help: Some("https://example.org/help".into()),
                homepage: Some("https://example.org/".into())
            }
        );
        assert_eq!(parse("<component/>"), Links::default());
    }
}
