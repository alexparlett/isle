//! UDisks2: filesystems on removable drives and loop images the user set up, with mounting,
//! unmounting and ejecting. Nothing mounts by itself.

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
    for (path, interfaces) in &objects {
        let (Some(block), Some(fs)) = (
            interfaces.get("org.freedesktop.UDisks2.Block"),
            interfaces.get("org.freedesktop.UDisks2.Filesystem"),
        ) else {
            continue;
        };
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
        if !(removable || loop_device) || (block.bool("HintSystem") == Some(true) && !loop_device) {
            continue;
        }
        let mounts: Vec<Vec<u8>> = fs
            .get("MountPoints")
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
    let conn = isle_bus::system().await?;
    conn.call_method(
        Some(UDISKS),
        volume,
        Some("org.freedesktop.UDisks2.Filesystem"),
        "Unmount",
        &(options(),),
    )
    .await?;
    Ok(())
}

/// Unmounts a volume and ejects its drive, or detaches its loop image.
pub async fn eject(volume: &Volume) -> Result<(), Error> {
    if volume.mount_point.is_some() {
        unmount(&volume.path).await?;
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
