//! Screenshot and PickColor.

use std::{collections::HashMap, sync::Arc};

use zbus::{Connection, interface, message::Header};
use zvariant::{ObjectPath, OwnedValue, Value};

use crate::{BoxFuture, Closed, Reply, from_frontend, request::with_request, response};

pub const TARGET_SCREEN: u32 = 1;
pub const TARGET_WINDOW: u32 = 2;
pub const TARGET_AREA: u32 = 4;
pub const TARGET_ACTIVE_WINDOW: u32 = 8;

#[derive(Clone, Debug, Default)]
pub struct ShotRequest {
    pub app_id: String,
    pub parent_window: String,
    pub interactive: bool,
    /// One target, by the constants above; nought when the app names none.
    pub target: u32,
}

pub trait Screenshot: Send + Sync {
    /// The shot's file as a URI; `None` is a cancel.
    fn screenshot(&self, request: ShotRequest, closed: Closed) -> BoxFuture<Option<String>>;
    /// A colour off the screen, sRGB in [0, 1]; `None` is a cancel.
    fn pick_color(
        &self,
        app_id: String,
        parent_window: String,
        closed: Closed,
    ) -> BoxFuture<Option<(f64, f64, f64)>>;
}

pub struct ScreenshotIface {
    handler: Arc<dyn Screenshot>,
}

impl ScreenshotIface {
    pub fn new(handler: Arc<dyn Screenshot>) -> Self {
        Self { handler }
    }
}

#[interface(name = "org.freedesktop.impl.portal.Screenshot")]
impl ScreenshotIface {
    async fn screenshot(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        handle: ObjectPath<'_>,
        app_id: String,
        parent_window: String,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<Reply> {
        from_frontend(conn, &header).await?;
        let flag = |k: &str| {
            options
                .get(k)
                .and_then(|v| bool::try_from(v.try_clone().ok()?).ok())
                .unwrap_or(false)
        };
        let request = ShotRequest {
            app_id,
            parent_window,
            interactive: flag("interactive"),
            target: options
                .get("target")
                .and_then(|v| u32::try_from(v.try_clone().ok()?).ok())
                .unwrap_or(0),
        };
        let handler = self.handler.clone();
        let uri = with_request(conn, &handle, |closed| handler.screenshot(request, closed)).await;
        Ok(match uri {
            Some(uri) => {
                let mut results = HashMap::new();
                results.insert(
                    "uri".to_owned(),
                    OwnedValue::try_from(Value::from(uri)).unwrap(),
                );
                (response::SUCCESS, results)
            }
            None => (response::CANCELLED, HashMap::new()),
        })
    }

    async fn pick_color(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        handle: ObjectPath<'_>,
        app_id: String,
        parent_window: String,
        _options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<Reply> {
        from_frontend(conn, &header).await?;
        let handler = self.handler.clone();
        let colour = with_request(conn, &handle, |closed| {
            handler.pick_color(app_id, parent_window, closed)
        })
        .await;
        Ok(match colour {
            Some(rgb) => {
                let mut results = HashMap::new();
                results.insert(
                    "color".to_owned(),
                    OwnedValue::try_from(Value::from(rgb)).unwrap(),
                );
                (response::SUCCESS, results)
            }
            None => (response::CANCELLED, HashMap::new()),
        })
    }

    #[zbus(property)]
    fn available_targets(&self) -> u32 {
        TARGET_SCREEN | TARGET_WINDOW | TARGET_AREA | TARGET_ACTIVE_WINDOW
    }

    #[zbus(property, name = "version")]
    fn version(&self) -> u32 {
        3
    }
}
