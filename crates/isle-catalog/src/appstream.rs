//! AppStream catalogues as Store reads them: one component per app, its English fields only
//! (`store-2026.md` §1 and §3).

use std::io::BufRead;
use std::path::{Path, PathBuf};

use quick_xml::events::{BytesStart, Event};

/// A component's fields that Store shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Component {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub summary: String,
    /// Paragraphs and list items of the English description.
    pub description: Vec<String>,
    /// The package it comes from, in a distribution's catalogue.
    pub package: Option<String>,
    /// `app/<id>/<arch>/<branch>`, in Flathub's.
    pub bundle: Option<String>,
    /// Cached icons by their pixel size, as file names under the catalogue's icon folder.
    pub icons: Vec<(u32, String)>,
    /// An icon theme name, present once installed.
    pub stock_icon: Option<String>,
    pub categories: Vec<String>,
    pub screenshots: Vec<String>,
    pub licence: Option<String>,
    pub homepage: Option<String>,
    pub developer: Option<String>,
    /// The newest release's version and notes.
    pub release: Option<(String, Vec<String>)>,
}

impl Component {
    pub fn is_app(&self) -> bool {
        self.kind == "desktop-application" || self.kind == "desktop"
    }

    /// The largest cached icon no bigger than `size` pixels, else the smallest above it.
    pub fn icon_path(&self, icons: &Path, size: u32) -> Option<PathBuf> {
        let mut sizes: Vec<&(u32, String)> = self.icons.iter().collect();
        sizes.sort_by_key(|(s, _)| *s);
        let pick = sizes
            .iter()
            .rev()
            .find(|(s, _)| *s <= size)
            .or_else(|| sizes.first())?;
        Some(icons.join(format!("{0}x{0}", pick.0)).join(&pick.1))
    }
}

/// Where in a component the reader is, for the text it collects.
#[derive(Clone, Copy, PartialEq)]
enum Field {
    None,
    Id,
    Name,
    Summary,
    Paragraph,
    Package,
    Bundle,
    Icon(u32),
    StockIcon,
    Category,
    Screenshot,
    Licence,
    Homepage,
    Developer,
    ReleaseParagraph,
}

fn attr(e: &BytesStart, name: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == name)
        .map(|a| a.value.to_string())
}

fn entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => {
            let n = name.strip_prefix('#')?;
            let code = match n.strip_prefix('x') {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => n.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

/// Reads every component of a catalogue.
pub fn read(input: impl BufRead) -> Result<Vec<Component>, quick_xml::Error> {
    let mut reader = quick_xml::Reader::from_reader(input);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut out = Vec::new();
    let mut current: Option<Component> = None;
    let mut field = Field::None;
    let mut text = String::new();
    // Inside an element in another language, or one Store does not read, by depth.
    let mut skip_depth = 0usize;
    let mut depth = 0usize;
    let mut in_description = false;
    let mut in_releases = false;
    let mut in_release = false;
    let mut screenshot_kind = String::new();
    let mut release_notes: Vec<String> = Vec::new();
    let mut release_version = String::new();
    loop {
        let event = reader.read_event_into(&mut buf)?;
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let empty = matches!(event, Event::Empty(_));
                if !empty {
                    depth += 1;
                }
                if skip_depth > 0 {
                    buf.clear();
                    continue;
                }
                if attr(e, "xml:lang").is_some_and(|l| l != "en" && l != "en-US") {
                    if !empty {
                        skip_depth = depth;
                    }
                    buf.clear();
                    continue;
                }
                let name = e.name().as_ref().to_string();
                match name.as_str() {
                    "component" => {
                        in_description = false;
                        in_release = false;
                        in_releases = false;
                        current = Some(Component {
                            kind: attr(e, "type").unwrap_or_default(),
                            ..Component::default()
                        });
                    }
                    _ if current.is_none() => {}
                    "id" if depth == 3 => field = Field::Id,
                    "name" if depth == 3 => field = Field::Name,
                    "summary" if depth == 3 => field = Field::Summary,
                    "description" if depth == 3 => in_description = true,
                    "description" if in_release => {}
                    "p" | "li" if in_description => field = Field::Paragraph,
                    "p" | "li" if in_release => field = Field::ReleaseParagraph,
                    "pkgname" => field = Field::Package,
                    "bundle" => field = Field::Bundle,
                    "icon" => match attr(e, "type").as_deref() {
                        Some("cached") if attr(e, "scale").is_none_or(|s| s == "1") => {
                            field = Field::Icon(
                                attr(e, "width").and_then(|w| w.parse().ok()).unwrap_or(64),
                            );
                        }
                        Some("stock") => field = Field::StockIcon,
                        _ => {}
                    },
                    "category" => field = Field::Category,
                    "screenshot" => screenshot_kind = attr(e, "type").unwrap_or_default(),
                    "image" if attr(e, "type").as_deref() == Some("source") => {
                        field = Field::Screenshot;
                    }
                    "project_license" => field = Field::Licence,
                    "url" if attr(e, "type").as_deref() == Some("homepage") => {
                        field = Field::Homepage;
                    }
                    "developer_name" => field = Field::Developer,
                    "name"
                        if depth > 3 && current.as_ref().is_some_and(|c| c.developer.is_none()) =>
                    {
                        field = Field::Developer;
                    }
                    "releases" => in_releases = true,
                    "release" if in_releases && release_version.is_empty() => {
                        let version = attr(e, "version").unwrap_or_default();
                        // A release with no notes has no end tag to close it.
                        if empty {
                            if let Some(c) = current.as_mut() {
                                c.release = Some((version, Vec::new()));
                            }
                            release_version = "-".into();
                        } else {
                            in_release = true;
                            release_version = version;
                        }
                    }
                    _ => {}
                }
                text.clear();
                if empty {
                    field = Field::None;
                }
            }
            Event::Text(ref t) if skip_depth == 0 && field != Field::None => {
                text.push_str(t.as_ref());
            }
            Event::GeneralRef(ref r) if skip_depth == 0 && field != Field::None => {
                if let Some(c) = entity(r.as_ref()) {
                    text.push(c);
                }
            }
            Event::CData(ref t) if skip_depth == 0 && field != Field::None => {
                text.push_str(t.as_ref());
            }
            Event::End(ref e) => {
                if skip_depth > 0 {
                    if depth == skip_depth {
                        skip_depth = 0;
                    }
                    depth -= 1;
                    buf.clear();
                    continue;
                }
                let name = e.name().as_ref().to_string();
                let value = text.split_whitespace().collect::<Vec<_>>().join(" ");
                if let Some(c) = current.as_mut() {
                    match field {
                        Field::Id => c.id = value,
                        Field::Name if c.name.is_empty() => c.name = value,
                        Field::Summary if c.summary.is_empty() => c.summary = value,
                        Field::Paragraph if !value.is_empty() => c.description.push(value),
                        Field::Package => c.package = Some(value),
                        Field::Bundle => c.bundle = Some(value),
                        Field::Icon(size) if !c.icons.iter().any(|(s, _)| *s == size) => {
                            c.icons.push((size, value));
                        }
                        Field::StockIcon => c.stock_icon = Some(value),
                        Field::Category => c.categories.push(value),
                        Field::Screenshot if screenshot_kind == "default" => {
                            c.screenshots.insert(0, value);
                        }
                        Field::Screenshot => c.screenshots.push(value),
                        Field::Licence => c.licence = Some(value),
                        Field::Homepage => c.homepage = Some(value),
                        Field::Developer if !value.is_empty() => c.developer = Some(value),
                        Field::ReleaseParagraph if !value.is_empty() => release_notes.push(value),
                        _ => {}
                    }
                }
                field = Field::None;
                match name.as_str() {
                    "description" if !in_release => in_description = false,
                    "release" if in_release => {
                        in_release = false;
                        if let Some(c) = current.as_mut() {
                            c.release = Some((
                                std::mem::take(&mut release_version),
                                std::mem::take(&mut release_notes),
                            ));
                        }
                        // Only the newest release is read; the version stays set so later ones
                        // are passed over.
                        release_version = "-".into();
                    }
                    "releases" => in_releases = false,
                    "component" => {
                        if let Some(c) = current.take() {
                            out.push(c);
                        }
                        release_version.clear();
                        release_notes.clear();
                    }
                    _ => {}
                }
                depth -= 1;
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

/// Reads a catalogue file, gzipped or not.
pub fn read_file(path: &Path) -> std::io::Result<Vec<Component>> {
    let file = std::fs::File::open(path)?;
    let parsed = if path.extension().is_some_and(|e| e == "gz") {
        read(std::io::BufReader::new(flate2::read::GzDecoder::new(file)))
    } else {
        read(std::io::BufReader::new(file))
    };
    parsed.map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");

    #[test]
    fn release_notes_stay_out_of_the_description() {
        let xml = r#"<components><component type="desktop-application"><id>k</id><name>K</name>
<description><p>About it.</p></description>
<releases>
<release version="2"><description xml:lang="de"><p>Neu</p></description><description><p>Two</p></description></release>
<release version="1"><description><p>One</p></description></release>
</releases></component></components>"#;
        let all = read(xml.as_bytes()).unwrap();
        assert_eq!(all[0].description, ["About it."]);
        assert_eq!(all[0].release, Some(("2".into(), vec!["Two".into()])));
        // A notes-less release first, as many components have, then another component.
        let xml = r#"<components><component type="desktop-application"><id>a</id><name>A</name>
<description><p>A.</p></description><releases><release version="3" timestamp="1"/></releases></component>
<component type="desktop-application"><id>b</id><name>B</name><description><p>B.</p></description>
<releases><release version="2"><description><p>Two</p></description></release>
<release version="1"><description><p>One</p></description></release></releases></component></components>"#;
        let all = read(xml.as_bytes()).unwrap();
        assert_eq!(all[0].release, Some(("3".into(), vec![])));
        assert_eq!(all[1].description, ["B."]);
    }

    #[test]
    fn a_repository_catalogue() {
        let all = read_file(Path::new(&format!("{DATA}/appstream-extra.xml"))).unwrap();
        assert_eq!(all.len(), 4);
        let krita = all.iter().find(|c| c.id == "org.kde.krita").unwrap();
        assert!(krita.is_app());
        assert_eq!(krita.name, "Krita");
        assert_eq!(krita.summary, "Digital Painting, Creative Freedom");
        assert_eq!(krita.package.as_deref(), Some("krita"));
        assert!(
            krita.description[0].starts_with("Krita is the full-featured digital art studio."),
            "{:?}",
            krita.description
        );
        assert_eq!(
            krita.icons.iter().map(|(s, _)| *s).collect::<Vec<_>>(),
            [48, 64, 128]
        );
        assert_eq!(krita.stock_icon.as_deref(), Some("krita"));
        assert!(krita.categories.contains(&"Graphics".to_string()));
        assert_eq!(krita.licence.as_deref(), Some("GPL-3.0-only"));
        assert_eq!(krita.homepage.as_deref(), Some("https://www.krita.org/"));
        assert!(!krita.screenshots.is_empty());
        assert_eq!(
            krita.icon_path(Path::new("/icons"), 64).unwrap(),
            Path::new("/icons/64x64/krita_krita.jxl")
        );
        let addon = all.iter().find(|c| c.kind == "addon").unwrap();
        assert!(!addon.is_app());
    }

    #[test]
    fn flathub_s_catalogue() {
        let all = read_file(Path::new(&format!("{DATA}/appstream-flathub.xml"))).unwrap();
        let obs = all
            .iter()
            .find(|c| c.id == "com.obsproject.Studio")
            .unwrap();
        assert_eq!(obs.name, "OBS Studio");
        assert_eq!(
            obs.bundle.as_deref(),
            Some("app/com.obsproject.Studio/x86_64/stable")
        );
        assert_eq!(obs.package, None);
        assert_eq!(
            obs.icons.iter().map(|(s, _)| *s).collect::<Vec<_>>(),
            [64, 128]
        );
        assert!(obs.release.is_some());
    }

    #[test]
    fn entities_and_languages() {
        let xml = r#"<components><component type="desktop-application"><id>a</id>
            <name xml:lang="de">Ein</name><name>Tom &amp; Jerry&#x21;</name>
            <description xml:lang="de"><p>Nein</p></description><description><p>One</p><ul><li>Two</li></ul></description>
            </component></components>"#;
        let all = read(xml.as_bytes()).unwrap();
        assert_eq!(all[0].name, "Tom & Jerry!");
        assert_eq!(all[0].description, ["One", "Two"]);
    }
}

#[cfg(test)]
mod whole {
    /// `ISLE_CATALOG_FULL` names a whole catalogue to read, as a measure.
    #[test]
    #[ignore]
    fn a_whole_catalogue() {
        let path = std::env::var("ISLE_CATALOG_FULL").unwrap();
        let start = std::time::Instant::now();
        let all = super::read_file(std::path::Path::new(&path)).unwrap();
        let apps = all.iter().filter(|c| c.is_app()).count();
        let named = all
            .iter()
            .filter(|c| c.is_app() && !c.name.is_empty() && !c.summary.is_empty())
            .count();
        eprintln!(
            "{} components, {apps} apps, {named} named, in {:?}",
            all.len(),
            start.elapsed()
        );
        assert_eq!(apps, named);
    }
}
