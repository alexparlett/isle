//! `org.freedesktop.UDisks2`: removable sticks that come and go, mount, unmount and eject.

use zbus::{Connection, object_server::SignalEmitter, zvariant::OwnedObjectPath};

const PATH: &str = "/org/freedesktop/UDisks2";

struct Drive;

#[zbus::interface(name = "org.freedesktop.UDisks2.Drive")]
impl Drive {
    #[zbus(property)]
    fn removable(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn media_removable(&self) -> bool {
        true
    }
    fn eject(&self, _options: std::collections::HashMap<String, zbus::zvariant::OwnedValue>) {}
}

struct Block {
    label: String,
    drive: OwnedObjectPath,
}

#[zbus::interface(name = "org.freedesktop.UDisks2.Block")]
impl Block {
    #[zbus(property)]
    fn id_label(&self) -> String {
        self.label.clone()
    }
    #[zbus(property)]
    fn drive(&self) -> OwnedObjectPath {
        self.drive.clone()
    }
    #[zbus(property)]
    fn hint_ignore(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn hint_system(&self) -> bool {
        false
    }
}

struct Filesystem {
    label: String,
    mounted: bool,
}

#[zbus::interface(name = "org.freedesktop.UDisks2.Filesystem")]
impl Filesystem {
    #[zbus(property)]
    fn mount_points(&self) -> Vec<Vec<u8>> {
        if self.mounted {
            let mut point = format!("/run/media/test/{}", self.label).into_bytes();
            point.push(0);
            vec![point]
        } else {
            Vec::new()
        }
    }

    async fn mount(
        &mut self,
        _options: std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> String {
        self.mounted = true;
        let _ = self.mount_points_changed(&emitter).await;
        format!("/run/media/test/{}", self.label)
    }

    async fn unmount(
        &mut self,
        _options: std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.mounted = false;
        let _ = self.mount_points_changed(&emitter).await;
    }
}

/// UDisks2 on the bus at an address, with no sticks until one is plugged.
pub struct UDisks {
    conn: Connection,
}

impl UDisks {
    pub async fn start(address: &str) -> zbus::Result<Self> {
        let conn = zbus::connection::Builder::address(address)?
            .serve_at(PATH, zbus::fdo::ObjectManager)?
            .name("org.freedesktop.UDisks2")?
            .build()
            .await?;
        Ok(Self { conn })
    }

    /// A stick with one filesystem labelled `label`.
    pub async fn plug(&self, label: &str) -> zbus::Result<()> {
        let drive = OwnedObjectPath::try_from(format!("{PATH}/drives/{label}")).unwrap();
        let block = format!("{PATH}/block_devices/{label}1");
        let server = self.conn.object_server();
        server.at(drive.clone(), Drive).await?;
        server
            .at(
                block.as_str(),
                Block {
                    label: label.into(),
                    drive,
                },
            )
            .await?;
        server
            .at(
                block.as_str(),
                Filesystem {
                    label: label.into(),
                    mounted: false,
                },
            )
            .await?;
        Ok(())
    }

    pub async fn mounted(&self, label: &str) -> bool {
        self.conn
            .object_server()
            .interface::<_, Filesystem>(format!("{PATH}/block_devices/{label}1"))
            .await
            .map(|i| async move { i.get().await.mounted })
            .unwrap()
            .await
    }
}
