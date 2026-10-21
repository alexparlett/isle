//! Asking xdg-desktop-portal for a file, as any app does; under Isle its FileChooser is
//! Files' panel, a sheet on the asking window (`p15-files.md` choice 33).

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};

use futures_util::StreamExt;
use zbus::proxy;
use zvariant::{OwnedValue, Value};

#[proxy(
    interface = "org.freedesktop.portal.FileChooser",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait FileChooser {
    fn open_file(
        &self,
        parent_window: &str,
        title: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<zvariant::OwnedObjectPath>;
}

#[proxy(interface = "org.freedesktop.portal.Request", default_service = "org.freedesktop.portal.Desktop")]
trait PortalRequest {
    #[zbus(signal)]
    fn response(&self, response: u32, results: HashMap<String, OwnedValue>) -> zbus::Result<()>;
}

/// What to ask for.
#[derive(Clone, Debug, Default)]
pub struct Wanted {
    pub title: String,
    pub directory: bool,
    /// Named filters of MIME types.
    pub types: Vec<(String, Vec<String>)>,
    pub folder: Option<PathBuf>,
}

static NEXT: AtomicU32 = AtomicU32::new(1);

/// The file chosen, or `None` when the user cancelled; `parent` is the asking window's
/// `wayland:` handle, or empty.
pub async fn choose_file(bus: &zbus::Connection, parent: &str, wanted: Wanted) -> zbus::Result<Option<PathBuf>> {
    let token = format!("isle{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let sender = bus.unique_name().map(|n| n.trim_start_matches(':').replace('.', "_")).unwrap_or_default();
    let path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
    // Listening before asking, so the answer cannot come first.
    let request = PortalRequestProxy::builder(bus).path(path)?.build().await?;
    let mut responses = request.receive_response().await?;
    let mut options: HashMap<&str, Value<'_>> = HashMap::new();
    options.insert("handle_token", Value::from(token.clone()));
    options.insert("directory", Value::from(wanted.directory));
    options.insert("modal", Value::from(true));
    if !wanted.types.is_empty() {
        let filters: Vec<(String, Vec<(u32, String)>)> =
            wanted.types.iter().map(|(name, types)| (name.clone(), types.iter().map(|t| (1u32, t.clone())).collect())).collect();
        options.insert("filters", Value::from(filters));
    }
    if let Some(folder) = &wanted.folder {
        use std::os::unix::ffi::OsStrExt;
        let mut bytes = folder.as_os_str().as_bytes().to_vec();
        bytes.push(0);
        options.insert("current_folder", Value::from(bytes));
    }
    FileChooserProxy::new(bus).await?.open_file(parent, &wanted.title, options).await?;
    let Some(answer) = responses.next().await else { return Ok(None) };
    let args = answer.args()?;
    if args.response != 0 {
        return Ok(None);
    }
    let uris: Vec<String> = args
        .results
        .get("uris")
        .and_then(|v| v.try_clone().ok())
        .and_then(|v| Vec::<String>::try_from(v).ok())
        .unwrap_or_default();
    Ok(uris.first().and_then(|u| u.strip_prefix("file://")).map(|p| PathBuf::from(decode(p))))
}

fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
