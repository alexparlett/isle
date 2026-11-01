//! What an item is, by its schema and attributes (`p19c-keychain.md` choice 7).

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Login,
    WiFi,
    SshKey,
    App,
}

/// Who stored an app's secret, as far as the item says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Owner {
    /// The `application` attribute, an app's own name for itself.
    Application(String),
    /// A sandboxed app's id, from a portal secret.
    Portal(String),
    /// A known schema's owner: its name and the desktop id to find its icon by.
    Known {
        name: &'static str,
        id: &'static str,
    },
    /// Nothing says; the item's label stands for it.
    Unknown,
}

/// NetworkManager's secret for one setting of a connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NmSecret {
    pub uuid: String,
    pub setting: String,
    pub key: String,
}

/// An item read for the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Classified {
    pub kind: Kind,
    pub title: String,
    pub user: Option<String>,
    /// The website or server a login is for.
    pub website: Option<String>,
    pub notes: Option<String>,
    pub owner: Owner,
    /// A browser's or Electron app's key to its own saved passwords and cookies.
    pub storage_key: bool,
    /// A NetworkManager connection's secret; the window shows a Wi-Fi one on its network.
    pub nm: Option<NmSecret>,
}

const SCHEMA: &str = "xdg:schema";
pub const NOTES: &str = "isle:notes";

const LOGIN_SCHEMAS: [&str; 3] = [
    "org.gnome.keyring.NetworkPassword",
    "org.epiphany.FormPassword",
    "org.remmina.Password",
];

const USER: [&str; 5] = ["user", "username", "login", "account", "form_username"];
const SITE: [&str; 7] = [
    "server",
    "url",
    "uri",
    "origin",
    "target_origin",
    "host",
    "domain",
];

fn first(attributes: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|k| attributes.get(*k))
        .find(|v| !v.is_empty())
        .cloned()
}

/// Whether `s` reads as a URL or a host name.
fn site_like(s: &str) -> bool {
    if let Some((scheme, rest)) = s.split_once("://") {
        return !scheme.is_empty() && !rest.is_empty();
    }
    let host = s.split(['/', ':']).next().unwrap_or("");
    host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

/// The host part of a website or server, for a login's title.
fn host_of(site: &str) -> &str {
    let rest = site.split_once("://").map_or(site, |(_, r)| r);
    let rest = rest.rsplit_once('@').map_or(rest, |(_, r)| r);
    rest.split(['/', ':', '?', '#']).next().unwrap_or(rest)
}

fn known_owner(schema: &str, attributes: &HashMap<String, String>) -> Option<Owner> {
    Some(match schema {
        "org.gnome.OnlineAccounts" => Owner::Known {
            name: "Online Accounts",
            id: "gnome-online-accounts",
        },
        "org.freedesktop.NetworkManager.Connection" => Owner::Known {
            name: "NetworkManager",
            id: "nm-connection-editor",
        },
        _ if attributes
            .get("unique")
            .is_some_and(|u| u.starts_with("ssh-store:")) =>
        {
            Owner::Known {
                name: "SSH",
                id: "gcr-ssh-agent",
            }
        }
        _ => return None,
    })
}

/// Reads an item by its label and attributes.
pub fn classify(label: &str, attributes: &HashMap<String, String>) -> Classified {
    let schema = attributes.get(SCHEMA).map_or("", String::as_str);
    let notes = attributes.get(NOTES).filter(|n| !n.is_empty()).cloned();
    let user = first(attributes, &USER);
    let site = first(attributes, &SITE);
    let generic_site = attributes.get("service").filter(|s| site_like(s)).cloned();

    let nm = (schema == "org.freedesktop.NetworkManager.Connection").then(|| NmSecret {
        uuid: attributes
            .get("connection-uuid")
            .cloned()
            .unwrap_or_default(),
        setting: attributes.get("setting-name").cloned().unwrap_or_default(),
        key: attributes.get("setting-key").cloned().unwrap_or_default(),
    });
    if let Some(nm) = nm {
        let wifi = nm.setting == "802-11-wireless-security";
        return Classified {
            kind: if wifi { Kind::WiFi } else { Kind::App },
            title: label.to_owned(),
            user: None,
            website: None,
            notes,
            owner: known_owner(schema, attributes).unwrap_or(Owner::Unknown),
            storage_key: false,
            nm: Some(nm),
        };
    }

    let login = LOGIN_SCHEMAS.contains(&schema)
        || (user.is_some() && site.is_some())
        || (schema == "org.freedesktop.Secret.Generic" && user.is_some() && generic_site.is_some());
    if login {
        let website = site.or(generic_site);
        let title = if label.is_empty() {
            website
                .as_deref()
                .map(host_of)
                .unwrap_or_default()
                .to_owned()
        } else {
            label.to_owned()
        };
        return Classified {
            kind: Kind::Login,
            title,
            user,
            website,
            notes,
            owner: Owner::Unknown,
            storage_key: false,
            nm: None,
        };
    }

    let storage_key = schema.ends_with("_libsecret_os_crypt_password_v2")
        || schema.ends_with("_libsecret_os_crypt_password");
    let owner = if let Some(app) = attributes.get("application").filter(|a| !a.is_empty()) {
        Owner::Application(app.clone())
    } else if let Some(id) = attributes.get("app_id").filter(|a| !a.is_empty()) {
        Owner::Portal(id.clone())
    } else {
        known_owner(schema, attributes).unwrap_or(Owner::Unknown)
    };
    Classified {
        kind: Kind::App,
        title: label.to_owned(),
        user,
        website: None,
        notes,
        owner,
        storage_key,
        nm: None,
    }
}

/// The attributes a new login is saved with, so libsecret's network lookups find it.
pub fn login_attributes(user: &str, website: &str, notes: &str) -> HashMap<String, String> {
    let mut out = HashMap::from([
        (
            SCHEMA.to_owned(),
            "org.gnome.keyring.NetworkPassword".to_owned(),
        ),
        ("user".to_owned(), user.to_owned()),
    ]);
    let (protocol, rest) = website
        .split_once("://")
        .map_or((None, website), |(p, r)| (Some(p), r));
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let authority = authority.rsplit_once('@').map_or(authority, |(_, a)| a);
    let (server, port) = match authority.rsplit_once(':') {
        Some((h, p)) if p.parse::<u16>().is_ok() => (h, Some(p)),
        _ => (authority, None),
    };
    out.insert("server".to_owned(), server.to_owned());
    if let Some(p) = protocol {
        out.insert("protocol".to_owned(), p.to_owned());
    }
    if let Some(p) = port {
        out.insert("port".to_owned(), p.to_owned());
    }
    if !notes.is_empty() {
        out.insert(NOTES.to_owned(), notes.to_owned());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect()
    }

    // The shapes of the host keyring's twelve items (apps-2026.md §6), and the schemas' own.
    #[test]
    fn browser_and_electron_storage_keys_are_apps_with_a_warning() {
        for (schema, app) in [
            ("chrome_libsecret_os_crypt_password_v2", "chrome"),
            ("chromium_libsecret_os_crypt_password_v2", "chromium"),
            ("chromium_libsecret_os_crypt_password_v2", "claude"),
            ("chromium_libsecret_os_crypt_password_v2", "Slack"),
        ] {
            let c = classify(
                "Chrome Safe Storage",
                &attrs(&[("xdg:schema", schema), ("application", app)]),
            );
            assert_eq!(c.kind, Kind::App);
            assert!(c.storage_key);
            assert_eq!(c.owner, Owner::Application(app.into()));
        }
    }

    #[test]
    fn network_passwords_are_logins() {
        let c = classify(
            "",
            &attrs(&[
                ("xdg:schema", "org.gnome.keyring.NetworkPassword"),
                ("user", "alex"),
                ("server", "github.com"),
                ("protocol", "https"),
            ]),
        );
        assert_eq!(c.kind, Kind::Login);
        assert_eq!(c.title, "github.com");
        assert_eq!(c.user.as_deref(), Some("alex"));
        assert_eq!(c.website.as_deref(), Some("github.com"));
    }

    #[test]
    fn epiphany_and_remmina_are_logins() {
        let c = classify(
            "Password for alex on https://example.org",
            &attrs(&[
                ("xdg:schema", "org.epiphany.FormPassword"),
                ("uri", "https://example.org"),
                ("username", "alex"),
                ("form_username", "email"),
            ]),
        );
        assert_eq!(c.kind, Kind::Login);
        assert_eq!(c.website.as_deref(), Some("https://example.org"));
        assert_eq!(c.user.as_deref(), Some("alex"));
        let c = classify(
            "Remmina: box",
            &attrs(&[("xdg:schema", "org.remmina.Password")]),
        );
        assert_eq!(c.kind, Kind::Login);
    }

    #[test]
    fn generic_items_are_logins_only_with_a_site_and_a_user() {
        let python = classify(
            "Password for 'alex' on 'pypi.org'",
            &attrs(&[
                ("xdg:schema", "org.freedesktop.Secret.Generic"),
                ("service", "pypi.org"),
                ("username", "alex"),
            ]),
        );
        assert_eq!(python.kind, Kind::Login);
        assert_eq!(python.website.as_deref(), Some("pypi.org"));
        let gh = classify(
            "gh:github.com ''",
            &attrs(&[
                ("xdg:schema", "org.freedesktop.Secret.Generic"),
                ("service", "gh:github.com"),
                ("username", ""),
            ]),
        );
        assert_eq!(gh.kind, Kind::App);
        assert_eq!(gh.owner, Owner::Unknown);
        let plain = classify(
            "token",
            &attrs(&[("xdg:schema", "org.freedesktop.Secret.Generic")]),
        );
        assert_eq!(plain.kind, Kind::App);
    }

    #[test]
    fn portal_secrets_are_named_by_their_app() {
        let c = classify(
            "Application key for org.gnome.Fractal",
            &attrs(&[
                ("xdg:schema", "org.freedesktop.portal.Secret"),
                ("app_id", "org.gnome.Fractal"),
            ]),
        );
        assert_eq!(c.kind, Kind::App);
        assert_eq!(c.owner, Owner::Portal("org.gnome.Fractal".into()));
    }

    #[test]
    fn known_schemas_are_named_by_their_owner() {
        let goa = classify(
            "GOA google account",
            &attrs(&[("xdg:schema", "org.gnome.OnlineAccounts")]),
        );
        assert_eq!(goa.kind, Kind::App);
        assert!(matches!(
            goa.owner,
            Owner::Known {
                name: "Online Accounts",
                ..
            }
        ));
        let gcr = classify(
            "Unlock password for: id_ed25519",
            &attrs(&[("unique", "ssh-store:/home/a/.ssh/id_ed25519")]),
        );
        assert_eq!(gcr.kind, Kind::App);
        assert!(matches!(gcr.owner, Owner::Known { name: "SSH", .. }));
    }

    #[test]
    fn network_manager_secrets_go_to_wifi_or_apps() {
        let wifi = classify(
            "Network secret for Home/802-11-wireless-security/psk",
            &attrs(&[
                ("xdg:schema", "org.freedesktop.NetworkManager.Connection"),
                ("connection-uuid", "u-1"),
                ("setting-name", "802-11-wireless-security"),
                ("setting-key", "psk"),
            ]),
        );
        assert_eq!(wifi.kind, Kind::WiFi);
        assert_eq!(wifi.nm.unwrap().uuid, "u-1");
        let vpn = classify(
            "Network secret for Work/vpn/password",
            &attrs(&[
                ("xdg:schema", "org.freedesktop.NetworkManager.Connection"),
                ("connection-uuid", "u-2"),
                ("setting-name", "vpn"),
                ("setting-key", "password"),
            ]),
        );
        assert_eq!(vpn.kind, Kind::App);
        assert!(matches!(
            vpn.owner,
            Owner::Known {
                name: "NetworkManager",
                ..
            }
        ));
    }

    #[test]
    fn a_new_login_is_found_by_network_lookups() {
        let a = login_attributes("alex", "https://git.example.org:8443/repo", "");
        assert_eq!(a["xdg:schema"], "org.gnome.keyring.NetworkPassword");
        assert_eq!(a["server"], "git.example.org");
        assert_eq!(a["protocol"], "https");
        assert_eq!(a["port"], "8443");
        assert!(!a.contains_key(NOTES));
        let c = classify("Example", &a);
        assert_eq!(c.kind, Kind::Login);
        let b = login_attributes("me", "example.org", "spare key");
        assert_eq!(b["server"], "example.org");
        assert!(!b.contains_key("protocol"));
        assert_eq!(classify("x", &b).notes.as_deref(), Some("spare key"));
    }
}
