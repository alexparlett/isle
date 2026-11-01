//! Saved Wi-Fi networks and their passwords, read where each profile keeps it (choice 11).

use isle_network::{Keeps, SavedWifi};
use zbus::{Connection, zvariant::OwnedObjectPath};

use crate::{
    Revealed, kinds,
    service::{Error, Keyring, Result, Source},
};

fn from_bus(e: isle_bus::Error) -> Error {
    match e.downcast::<zbus::Error>() {
        Ok(z) => match &*z {
            zbus::Error::MethodError(name, _, _)
                if name.as_str().ends_with("PermissionDenied")
                    || name.as_str().ends_with("AccessDenied") =>
            {
                Error::Refused
            }
            zbus::Error::FDO(f) if matches!(**f, zbus::fdo::Error::AccessDenied(_)) => {
                Error::Refused
            }
            _ => Error::from(*z),
        },
        Err(e) => Error::Failed(e.to_string()),
    }
}

/// Every saved Wi-Fi network, without a password.
pub async fn saved(system: &Connection) -> Result<Vec<SavedWifi>> {
    isle_network::saved_wifi_on(system).await.map_err(from_bus)
}

/// The keyring item Isle's agent saved for an agent-owned profile.
pub fn agent_item(sources: &[Source], wifi: &SavedWifi) -> Option<OwnedObjectPath> {
    sources.iter().flat_map(|s| &s.items).find_map(|e| {
        let nm = kinds::classify(&e.label, &e.attributes).nm?;
        (nm.uuid == wifi.uuid && nm.setting == wifi.setting && nm.key == wifi.key)
            .then(|| e.path.clone())
    })
}

/// A saved network's password: from NetworkManager for a system-owned profile, through its
/// own polkit check, or from the keyring for an agent-owned one. `None` when nothing is saved.
pub async fn password(
    system: &Connection,
    keyring: Option<&Keyring>,
    agent_item: Option<&OwnedObjectPath>,
    wifi: &SavedWifi,
) -> Result<Option<Revealed>> {
    match wifi.keeps {
        Keeps::System => Ok(isle_network::wifi_password(system, wifi)
            .await
            .map_err(from_bus)?
            .map(Revealed::from_text)),
        Keeps::Agent => match (keyring, agent_item) {
            (Some(k), Some(item)) => k.fetch(item).await.map(Some),
            (None, _) => Err(Error::Missing),
            (_, None) => Ok(None),
        },
        Keeps::NotSaved | Keeps::Nothing => Ok(None),
    }
}
