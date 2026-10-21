//! UDisks2: filesystems on removable drives, internal disks that are not the system's, loop
//! images the user set up and encrypted volumes, with mounting, unlocking, unmounting and
//! ejecting. Nothing mounts by itself.

use std::collections::HashMap;

use isle_bus::{Error, Props, Publisher};
use serde::Serialize;
use zbus::{
    Connection,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

const UDISKS: &str = "org.freedesktop.UDisks2";
const PATH: &str = "/org/freedesktop/UDisks2";

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Volume {
    /// The block device's object path, which commands use.
    pub path: String,
    pub label: String,
    pub mount_point: Option<String>,
    /// The drive to eject, or the loop device to detach.
    pub drive: Option<String>,
    pub loop_device: bool,
    /// On a drive that can be taken out: it has an Eject button.
    pub removable: bool,
    /// Bytes.
    pub size: u64,
    /// The filesystem's type, `ext4`, `vfat`.
    pub fs_type: String,
    pub uuid: String,
    /// An encrypted volume still locked: unlocking it shows its filesystem.
    pub locked: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Disks {
    pub volumes: Vec<Volume>,
}

type Objects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

fn text(bytes: Option<Vec<u8>>) -> Option<String> {
    let bytes = bytes?;
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    Some(String::from_utf8_lossy(&bytes[..end]).into_owned()).filter(|s| !s.is_empty())
}

async fn read(conn: &Connection) -> Result<Disks, Error> {
    let objects: Objects = conn
        .call_method(
            Some(UDISKS),
            PATH,
            Some("org.freedesktop.DBus.ObjectManager"),
            "GetManagedObjects",
            &(),
        )
        .await?
        .body()
        .deserialize()?;
    let uid = unsafe { libc::getuid() };
    let mut volumes = Vec::new();
    // An unlocked container shows as its cleartext filesystem alone.
    let unlocked: Vec<String> = objects
        .values()
        .filter_map(|i| i.get("org.freedesktop.UDisks2.Block"))
        .filter_map(|b| Props(b.clone()).str("CryptoBackingDevice"))
        .filter(|p| p != "/")
        .collect();
    for (path, interfaces) in &objects {
        let Some(block) = interfaces.get("org.freedesktop.UDisks2.Block") else {
            continue;
        };
        let fs = interfaces.get("org.freedesktop.UDisks2.Filesystem");
        let locked = fs.is_none()
            && interfaces.contains_key("org.freedesktop.UDisks2.Encrypted")
            && !unlocked.contains(&path.to_string());
        if fs.is_none() && !locked {
            continue;
        }
        let block = Props(block.clone());
        if block.bool("HintIgnore") == Some(true) {
            continue;
        }
        let drive = block.str("Drive").filter(|d| d != "/");
        let removable = drive
            .as_ref()
            .and_then(|d| objects.get(&OwnedObjectPath::try_from(d.as_str()).ok()?))
            .and_then(|i| i.get("org.freedesktop.UDisks2.Drive"))
            .map(|d| Props(d.clone()))
            .is_some_and(|d| {
                d.bool("Removable") == Some(true) || d.bool("MediaRemovable") == Some(true)
            });
        let loop_device = interfaces
            .get("org.freedesktop.UDisks2.Loop")
            .map(|l| Props(l.clone()))
            .is_some_and(|l| l.i64("SetupByUID") == Some(i64::from(uid)));
        // The system's own disks are the computer itself, not volumes of it.
        if block.bool("HintSystem") == Some(true) && !(removable || loop_device) {
            continue;
        }
        let mounts: Vec<Vec<u8>> = fs
            .and_then(|fs| fs.get("MountPoints"))
            .and_then(|v| v.try_clone().ok())
            .and_then(|v| v.try_into().ok())
            .unwrap_or_default();
        volumes.push(Volume {
            path: path.to_string(),
            label: block
                .str("IdLabel")
                .filter(|l| !l.is_empty())
                .or_else(|| text(block.bytes("PreferredDevice")))
                .unwrap_or_else(|| "Untitled".into()),
            mount_point: mounts.into_iter().next().and_then(|m| text(Some(m))),
            drive,
            loop_device,
            removable,
            size: block.i64("Size").unwrap_or(0).max(0) as u64,
            fs_type: block.str("IdType").unwrap_or_default(),
            uuid: block.str("IdUUID").unwrap_or_default(),
            locked,
        });
    }
    volumes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Disks { volumes })
}

/// Follows UDisks2.
pub async fn watch(publisher: Publisher<Disks>) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    isle_bus::follow(&conn, UDISKS, PATH, &publisher, || read(&conn)).await
}

fn options() -> HashMap<&'static str, Value<'static>> {
    HashMap::new()
}

/// Mounts a volume where UDisks2 puts it, and returns the place.
pub async fn mount(volume: &str) -> Result<String, Error> {
    let conn = isle_bus::system().await?;
    Ok(conn
        .call_method(
            Some(UDISKS),
            volume,
            Some("org.freedesktop.UDisks2.Filesystem"),
            "Mount",
            &(options(),),
        )
        .await?
        .body()
        .deserialize()?)
}

pub async fn unmount(volume: &str) -> Result<(), Error> {
    unmount_with(volume, false).await
}

/// Unmounts; `force` even while apps hold files on it (Force Eject).
pub async fn unmount_with(volume: &str, force: bool) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    let mut opts = options();
    if force {
        opts.insert("force", Value::from(true));
    }
    conn.call_method(Some(UDISKS), volume, Some("org.freedesktop.UDisks2.Filesystem"), "Unmount", &(opts,))
        .await?;
    Ok(())
}

/// Whether UDisks2 refused because the volume is in use.
pub fn is_busy(error: &Error) -> bool {
    error.to_string().contains("DeviceBusy") || error.to_string().contains("target is busy")
}

/// Unlocks an encrypted volume with its passphrase; the cleartext filesystem's object path.
pub async fn unlock(volume: &str, passphrase: &str) -> Result<String, Error> {
    let conn = isle_bus::system().await?;
    let cleartext: OwnedObjectPath = conn
        .call_method(Some(UDISKS), volume, Some("org.freedesktop.UDisks2.Encrypted"), "Unlock", &(passphrase, options()))
        .await?
        .body()
        .deserialize()?;
    Ok(cleartext.to_string())
}

/// Passphrases kept in the keyring under GNOME's schema, so either desktop unlocks with them.
pub mod keyring {
    use std::collections::HashMap;

    use zbus::{
        Connection,
        zvariant::{OwnedObjectPath, OwnedValue, Value},
    };

    const SECRETS: &str = "org.freedesktop.secrets";
    const SERVICE: &str = "/org/freedesktop/secrets";
    const SCHEMA: &str = "org.gnome.GVfs.Luks.Password";

    async fn session(bus: &Connection) -> zbus::Result<OwnedObjectPath> {
        let (_, session): (OwnedValue, OwnedObjectPath) = bus
            .call_method(Some(SECRETS), SERVICE, Some("org.freedesktop.Secret.Service"), "OpenSession", &("plain", Value::from("")))
            .await?
            .body()
            .deserialize()?;
        Ok(session)
    }

    /// The passphrase kept for a volume's UUID, while the keyring is unlocked.
    pub async fn lookup(uuid: &str) -> zbus::Result<Option<String>> {
        let bus = Connection::session().await?;
        let attributes = HashMap::from([("gvfs-luks-uuid", uuid)]);
        let (unlocked, _): (Vec<OwnedObjectPath>, Vec<OwnedObjectPath>) = bus
            .call_method(Some(SECRETS), SERVICE, Some("org.freedesktop.Secret.Service"), "SearchItems", &(attributes,))
            .await?
            .body()
            .deserialize()?;
        let Some(item) = unlocked.first() else { return Ok(None) };
        let session = session(&bus).await?;
        let secret: (OwnedObjectPath, Vec<u8>, Vec<u8>, String) = bus
            .call_method(Some(SECRETS), item.as_str(), Some("org.freedesktop.Secret.Item"), "GetSecret", &(&session,))
            .await?
            .body()
            .deserialize()?;
        Ok(Some(String::from_utf8_lossy(&secret.2).into_owned()))
    }

    /// Keeps a volume's passphrase in the default keyring.
    pub async fn save(uuid: &str, label: &str, passphrase: &str) -> zbus::Result<()> {
        let bus = Connection::session().await?;
        let session = session(&bus).await?;
        let attributes = HashMap::from([("gvfs-luks-uuid", uuid.to_owned()), ("xdg:schema", SCHEMA.to_owned())]);
        let mut properties: HashMap<&str, Value<'_>> = HashMap::new();
        properties.insert("org.freedesktop.Secret.Item.Label", Value::from(format!("Encryption passphrase for {label}")));
        properties.insert("org.freedesktop.Secret.Item.Attributes", Value::from(attributes));
        let value = (&session, Vec::<u8>::new(), passphrase.as_bytes().to_vec(), "text/plain");
        let _: (OwnedObjectPath, OwnedObjectPath) = bus
            .call_method(
                Some(SECRETS),
                "/org/freedesktop/secrets/aliases/default",
                Some("org.freedesktop.Secret.Collection"),
                "CreateItem",
                &(properties, value, true),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(())
    }
}

/// Unmounts a volume and ejects its drive, or detaches its loop image.
pub async fn eject(volume: &Volume) -> Result<(), Error> {
    eject_with(volume, false).await
}

/// As `eject`; `force` unmounts even while apps hold files on it.
pub async fn eject_with(volume: &Volume, force: bool) -> Result<(), Error> {
    if volume.mount_point.is_some() {
        unmount_with(&volume.path, force).await?;
    }
    let conn = isle_bus::system().await?;
    if volume.loop_device {
        conn.call_method(
            Some(UDISKS),
            volume.path.as_str(),
            Some("org.freedesktop.UDisks2.Loop"),
            "Delete",
            &(options(),),
        )
        .await?;
    } else if let Some(drive) = &volume.drive {
        conn.call_method(
            Some(UDISKS),
            drive.as_str(),
            Some("org.freedesktop.UDisks2.Drive"),
            "Eject",
            &(options(),),
        )
        .await?;
    }
    Ok(())
}
