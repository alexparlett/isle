//! The Access dialog the frontend raises for permissions it checks itself: a non-interactive
//! screenshot among them.

use std::{collections::HashMap, sync::Arc};

use zbus::{Connection, interface, message::Header};
use zvariant::{ObjectPath, OwnedValue};

use crate::{BoxFuture, Closed, Reply, from_frontend, request::with_request, response};

/// What the frontend asks the user.
#[derive(Clone, Debug, Default)]
pub struct AccessRequest {
    pub app_id: String,
    pub parent_window: String,
    pub title: String,
    pub subtitle: String,
    pub body: String,
    pub deny_label: Option<String>,
    pub grant_label: Option<String>,
    pub icon: Option<String>,
}

pub trait Access: Send + Sync {
    /// Whether the user granted it.
    fn dialog(&self, request: AccessRequest, closed: Closed) -> BoxFuture<Option<bool>>;
}

pub struct AccessIface {
    handler: Arc<dyn Access>,
}

impl AccessIface {
    pub fn new(handler: Arc<dyn Access>) -> Self {
        Self { handler }
    }
}

fn text(options: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    options
        .get(key)
        .and_then(|v| String::try_from(v.try_clone().ok()?).ok())
}

#[interface(name = "org.freedesktop.impl.portal.Access")]
impl AccessIface {
    #[allow(clippy::too_many_arguments)]
    async fn access_dialog(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        handle: ObjectPath<'_>,
        app_id: String,
        parent_window: String,
        title: String,
        subtitle: String,
        body: String,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<Reply> {
        from_frontend(conn, &header).await?;
        let request = AccessRequest {
            app_id,
            parent_window,
            title,
            subtitle,
            body,
            deny_label: text(&options, "deny_label"),
            grant_label: text(&options, "grant_label"),
            icon: text(&options, "icon"),
        };
        let handler = self.handler.clone();
        let granted = with_request(conn, &handle, |closed| handler.dialog(request, closed)).await;
        Ok(match granted {
            Some(true) => (response::SUCCESS, HashMap::new()),
            Some(false) => (response::CANCELLED, HashMap::new()),
            None => (response::OTHER, HashMap::new()),
        })
    }
}
