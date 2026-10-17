//! One instance per app: the first owns the app's bus name and serves
//! `org.freedesktop.Application`; a later launch hands its request to it and exits
//! (`apps.md` Conventions 3; `settings-2026.md` §3).

use std::collections::HashMap;

use futures_channel::mpsc::UnboundedSender;
use tracing::info;
use zbus::{
    fdo::{RequestNameFlags, RequestNameReply},
    interface, proxy,
};
use zvariant::{OwnedValue, Value};

/// What a launch or an activation asked the app for.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    Activate { token: Option<String> },
    Open { uris: Vec<String>, token: Option<String> },
    Action { name: String, params: Vec<String>, token: Option<String> },
}

impl Request {
    /// The request a command line makes: URIs to open, or the app activated.
    pub fn from_args(args: Vec<String>, token: Option<String>) -> Self {
        if args.is_empty() {
            Request::Activate { token }
        } else {
            Request::Open { uris: args, token }
        }
    }

    pub fn token(&self) -> Option<&str> {
        match self {
            Request::Activate { token } | Request::Open { token, .. } | Request::Action { token, .. } => {
                token.as_deref()
            }
        }
    }
}

/// The object path of a desktop file id's `org.freedesktop.Application`.
pub fn object_path(id: &str) -> String {
    format!("/{}", id.replace('.', "/").replace('-', "_"))
}

pub enum Claim {
    /// This process owns the name; requests arrive through the channel given.
    Ours(zbus::Connection),
    /// Another instance took the request.
    Handed,
}

struct Application {
    requests: UnboundedSender<Request>,
}

fn token(platform_data: &HashMap<String, OwnedValue>) -> Option<String> {
    ["activation-token", "desktop-startup-id"]
        .iter()
        .find_map(|key| platform_data.get(*key))
        .and_then(|v| String::try_from(v.clone()).ok())
        .filter(|t| !t.is_empty())
}

#[interface(name = "org.freedesktop.Application")]
impl Application {
    fn activate(&self, platform_data: HashMap<String, OwnedValue>) {
        let _ = self.requests.unbounded_send(Request::Activate {
            token: token(&platform_data),
        });
    }

    fn open(&self, uris: Vec<String>, platform_data: HashMap<String, OwnedValue>) {
        let _ = self.requests.unbounded_send(Request::Open {
            uris,
            token: token(&platform_data),
        });
    }

    fn activate_action(
        &self,
        name: String,
        parameter: Vec<OwnedValue>,
        platform_data: HashMap<String, OwnedValue>,
    ) {
        let params = parameter
            .into_iter()
            .filter_map(|v| String::try_from(v).ok())
            .collect();
        let _ = self.requests.unbounded_send(Request::Action {
            name,
            params,
            token: token(&platform_data),
        });
    }
}

#[proxy(interface = "org.freedesktop.Application")]
trait RunningApp {
    fn activate(&self, platform_data: HashMap<&str, Value<'_>>) -> zbus::Result<()>;
    fn open(&self, uris: &[&str], platform_data: HashMap<&str, Value<'_>>) -> zbus::Result<()>;
    fn activate_action(
        &self,
        name: &str,
        parameter: &[Value<'_>],
        platform_data: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<()>;
}

/// Serves the app's object, then asks for its name; if another instance has it, hands
/// `first` to that instance.
pub async fn claim(
    id: &str,
    first: Request,
    requests: UnboundedSender<Request>,
) -> zbus::Result<Claim> {
    let path = object_path(id);
    let bus = zbus::connection::Builder::session()?
        .serve_at(path.as_str(), Application { requests: requests.clone() })?
        .build()
        .await?;
    let taken = match bus
        .request_name_with_flags(id, RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(reply) => reply == RequestNameReply::Exists,
        Err(zbus::Error::NameTaken) => true,
        Err(err) => return Err(err),
    };
    if !taken {
        let _ = requests.unbounded_send(first);
        return Ok(Claim::Ours(bus));
    }

    info!(id, "already running; handing the request over");
    let running = RunningAppProxy::builder(&bus)
        .destination(id)?
        .path(path.as_str())?
        .build()
        .await?;
    let mut data = HashMap::new();
    if let Some(token) = first.token() {
        data.insert("activation-token", Value::from(token.to_owned()));
    }
    match &first {
        Request::Activate { .. } => running.activate(data).await?,
        Request::Open { uris, .. } => {
            let uris: Vec<&str> = uris.iter().map(String::as_str).collect();
            running.open(&uris, data).await?
        }
        Request::Action { name, params, .. } => {
            let params: Vec<Value> = params.iter().map(|p| Value::from(p.as_str())).collect();
            running.activate_action(name, &params, data).await?
        }
    }
    Ok(Claim::Handed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_paths_follow_the_desktop_entry_spec() {
        assert_eq!(
            object_path("io.github.alexparlett.Isle.Settings"),
            "/io/github/alexparlett/Isle/Settings"
        );
        assert_eq!(object_path("org.example.foo-viewer"), "/org/example/foo_viewer");
    }

    #[test]
    fn a_command_line_opens_its_uris_or_activates() {
        assert_eq!(
            Request::from_args(vec![], Some("t".into())),
            Request::Activate { token: Some("t".into()) }
        );
        assert_eq!(
            Request::from_args(vec!["isle-settings://displays".into()], None),
            Request::Open { uris: vec!["isle-settings://displays".into()], token: None }
        );
    }
}
