//! The apps each catalogue offers, as Store lists them (`p14e-store.md` choices 5–7).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::appstream::{self, Component};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Kind {
    Cachyos,
    Flathub,
    Aur,
    /// An image Isle registered from `~/Applications`; no catalogue lists one.
    AppImage,
}

/// One app a catalogue offers.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub kind: Kind,
    /// The component id, or the AUR package's name.
    pub id: String,
    /// What installs it: the package, the Flatpak app id, the AUR package.
    pub install: String,
    pub name: String,
    pub summary: String,
    pub description: Vec<String>,
    /// The catalogue's own icon, possibly JPEG XL; `icons::drawable` makes it drawable.
    pub icon: Option<PathBuf>,
    pub categories: Vec<String>,
    pub screenshots: Vec<String>,
    pub licence: Option<String>,
    pub homepage: Option<String>,
    pub developer: Option<String>,
    pub version: Option<String>,
    /// The AUR's own facts: votes, popularity, last change, out of date.
    pub aur: Option<AurFacts>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AurFacts {
    pub votes: u64,
    pub popularity: f64,
    pub last_modified: u64,
    pub out_of_date: bool,
    pub maintainer: Option<String>,
}

fn offer(kind: Kind, c: Component, install: String, icons: &Path) -> Offer {
    Offer {
        kind,
        icon: c.icon_path(icons, 128),
        id: c.id,
        install,
        name: c.name,
        summary: c.summary,
        description: c.description,
        categories: c.categories,
        screenshots: c.screenshots,
        licence: c.licence,
        homepage: c.homepage,
        developer: c.developer,
        version: c.release.map(|(v, _)| v),
        aur: None,
    }
}

/// CachyOS's repositories, from `archlinux-appstream-data` under `root`.
pub fn repositories(root: &Path) -> Vec<Offer> {
    let base = root.join("usr/share/swcatalog");
    let mut out = Vec::new();
    for repo in ["core", "extra", "multilib"] {
        let Ok(all) = appstream::read_file(&base.join(format!("xml/{repo}.xml.gz"))) else {
            continue;
        };
        let icons = base.join(format!("icons/archlinux-arch-{repo}"));
        for c in all.into_iter().filter(Component::is_app) {
            if let Some(package) = c.package.clone() {
                out.push(offer(Kind::Cachyos, c, package, &icons));
            }
        }
    }
    out
}

/// Flathub, from the appstream flatpak keeps for the remote, the user's first, then the
/// system's.
pub fn flathub(home: &Path, root: &Path) -> Vec<Offer> {
    let places = [
        home.join(".local/share/flatpak/appstream/flathub/x86_64/active"),
        root.join("var/lib/flatpak/appstream/flathub/x86_64/active"),
    ];
    for dir in places {
        let Ok(all) = appstream::read_file(&dir.join("appstream.xml.gz")) else {
            continue;
        };
        let icons = dir.join("icons");
        return all
            .into_iter()
            .filter(Component::is_app)
            .filter_map(|c| {
                let app = c.bundle.as_deref()?.split('/').nth(1)?.to_string();
                Some(offer(Kind::Flathub, c, app, &icons))
            })
            .collect();
    }
    Vec::new()
}

#[derive(Deserialize)]
struct RpcReply {
    #[serde(default)]
    results: Vec<RpcPackage>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RpcPackage {
    name: String,
    version: String,
    description: Option<String>,
    #[serde(rename = "URL")]
    url: Option<String>,
    num_votes: u64,
    popularity: f64,
    out_of_date: Option<u64>,
    last_modified: u64,
    maintainer: Option<String>,
}

/// AUR packages matching `term` by name and description, most voted first.
pub fn parse_aur(json: &str) -> Result<Vec<Offer>, String> {
    let reply: RpcReply = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if let Some(error) = reply.error {
        return Err(error);
    }
    let mut out: Vec<Offer> = reply
        .results
        .into_iter()
        .map(|p| Offer {
            kind: Kind::Aur,
            id: p.name.clone(),
            install: p.name.clone(),
            summary: p.description.unwrap_or_default(),
            name: p.name,
            description: Vec::new(),
            icon: None,
            categories: Vec::new(),
            screenshots: Vec::new(),
            licence: None,
            homepage: p.url,
            developer: None,
            version: Some(p.version),
            aur: Some(AurFacts {
                votes: p.num_votes,
                popularity: p.popularity,
                last_modified: p.last_modified,
                out_of_date: p.out_of_date.is_some(),
                maintainer: p.maintainer,
            }),
        })
        .collect();
    out.sort_by(|a, b| {
        let v = |o: &Offer| o.aur.as_ref().map_or(0, |f| f.votes);
        v(b).cmp(&v(a))
    });
    Ok(out)
}

/// The AUR's client: one with no User-Agent is shown a bot check in place of JSON.
pub fn aur_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("isle-store/", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap_or_default()
}

pub async fn search_aur(term: &str) -> Result<Vec<Offer>, String> {
    let url = "https://aur.archlinux.org/rpc/v5/search";
    let reply = aur_client()
        .get(format!("{url}/{term}"))
        .query(&[("by", "name-desc")])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    parse_aur(&reply)
}

/// Whether an offer answers a search for `query`, already lowercased.
pub fn matches(offer: &Offer, query: &str) -> bool {
    offer.name.to_lowercase().contains(query)
        || offer.install.to_lowercase().contains(query)
        || offer.summary.to_lowercase().contains(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_aur_s_answer() {
        let json = r#"{"resultcount":2,"type":"search","version":5,"results":[
            {"Name":"obs-studio-git","Version":"r1-1","Description":"OBS from git","URL":null,"NumVotes":51,"Popularity":0.2,"OutOfDate":null,"LastModified":1790000000,"Maintainer":"a","PackageBase":"obs-studio-git","ID":1,"PackageBaseID":1,"FirstSubmitted":1,"URLPath":"x"},
            {"Name":"obs-vkcapture","Version":"1.5.4-1","Description":"Game capture","URL":"https://github.com/nowrep/obs-vkcapture","NumVotes":412,"Popularity":3.1,"OutOfDate":1791000000,"LastModified":1791000000,"Maintainer":null,"PackageBase":"obs-vkcapture","ID":2,"PackageBaseID":2,"FirstSubmitted":1,"URLPath":"y"}]}"#;
        let got = parse_aur(json).unwrap();
        assert_eq!(got[0].name, "obs-vkcapture");
        assert!(got[0].aur.as_ref().unwrap().out_of_date);
        assert_eq!(got[1].aur.as_ref().unwrap().votes, 51);
        assert_eq!(parse_aur(r#"{"error":"Too many package results.","results":[]}"#), Err("Too many package results.".into()));
    }

    #[test]
    fn flathub_s_offers_are_its_app_ids() {
        let home = std::env::temp_dir().join(format!("isle-catalog-flathub-{}", std::process::id()));
        let dir = home.join(".local/share/flatpak/appstream/flathub/x86_64/active");
        std::fs::create_dir_all(&dir).unwrap();
        let xml = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/appstream-flathub.xml")).unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut gz, &xml).unwrap();
        std::fs::write(dir.join("appstream.xml.gz"), gz.finish().unwrap()).unwrap();
        let got = flathub(&home, Path::new("/nonexistent"));
        assert!(got.iter().any(|o| o.install == "com.obsproject.Studio" && o.name == "OBS Studio"));
        assert_eq!(got[0].icon.as_ref().unwrap().parent().unwrap(), dir.join("icons/128x128"));
        std::fs::remove_dir_all(home).unwrap();
    }
}
