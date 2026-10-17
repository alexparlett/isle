//! The user's own account, for the sidebar's account row, from AccountsService.

use std::collections::HashMap;

use zvariant::OwnedValue;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Account {
    pub name: String,
    pub login: String,
    /// The picture's file, when the account has one.
    pub picture: Option<String>,
}

impl Account {
    /// Up to two initials of the name, for an account with no picture.
    pub fn initials(&self) -> String {
        let source = if self.name.is_empty() { &self.login } else { &self.name };
        source
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(2)
            .flat_map(char::to_uppercase)
            .collect()
    }
}

/// This user's account; without AccountsService, the login name alone.
pub async fn mine() -> Account {
    let login = std::env::var("USER").unwrap_or_default();
    let fallback = Account {
        login: login.clone(),
        ..Account::default()
    };
    let Ok(bus) = zbus::Connection::system().await else {
        return fallback;
    };
    let uid = unsafe { libc::getuid() } as i64;
    let Ok(accounts) = zbus::Proxy::new(
        &bus,
        "org.freedesktop.Accounts",
        "/org/freedesktop/Accounts",
        "org.freedesktop.Accounts",
    )
    .await
    else {
        return fallback;
    };
    let Ok(path) = accounts
        .call::<_, _, zvariant::OwnedObjectPath>("FindUserById", &(uid,))
        .await
    else {
        return fallback;
    };
    let Ok(props) = zbus::fdo::PropertiesProxy::builder(&bus)
        .destination("org.freedesktop.Accounts")
        .and_then(|b| b.path(path))
        .map(|b| b.build())
    else {
        return fallback;
    };
    let Ok(props) = props.await else {
        return fallback;
    };
    let Ok(all): Result<HashMap<String, OwnedValue>, _> = props
        .get_all(zbus::names::InterfaceName::from_static_str_unchecked(
            "org.freedesktop.Accounts.User",
        ))
        .await
    else {
        return fallback;
    };
    let text = |key: &str| {
        all.get(key)
            .and_then(|v| String::try_from(v.clone()).ok())
            .unwrap_or_default()
    };
    let picture = Some(text("IconFile")).filter(|p| std::path::Path::new(p).is_file());
    Account {
        name: text("RealName"),
        login: Some(text("UserName")).filter(|l| !l.is_empty()).unwrap_or(login),
        picture,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_come_from_the_name_or_the_login() {
        let named = Account {
            name: "Alex Parlett".into(),
            ..Account::default()
        };
        assert_eq!(named.initials(), "AP");
        let bare = Account {
            login: "alex".into(),
            ..Account::default()
        };
        assert_eq!(bare.initials(), "A");
    }
}
