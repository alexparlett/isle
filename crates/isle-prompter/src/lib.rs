//! Gcr's system prompter, which gnome-keyring, oo7, gcr's SSH agent and `pinentry-gnome3` ask
//! for passwords and confirmations (`lock-and-prompts-2026.md` §3). One caller prompts at a
//! time; the others wait their turn, as gcr's own prompter makes them.

pub mod exchange;

use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use futures_util::StreamExt;
use tokio::sync::watch;
use tracing::{debug, info, warn};
use zbus::{
    Connection, fdo, interface,
    message::Header,
    zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value},
};
use zeroize::Zeroizing;

pub const NAME: &str = "org.gnome.keyring.SystemPrompter";
pub const PATH: &str = "/org/gnome/keyring/Prompter";
const CALLBACK: &str = "org.gnome.keyring.internal.Prompter.Callback";

/// What a caller asks, from the prompt's properties.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Request {
    /// A yes or no, with no secret.
    pub confirm: bool,
    pub title: String,
    pub message: String,
    pub description: String,
    /// Why it asks again: the last password was wrong.
    pub warning: String,
    pub choice_label: Option<String>,
    pub choice_chosen: bool,
    pub continue_label: String,
    pub cancel_label: String,
    /// A new password, with its confirmation.
    pub password_new: bool,
    /// The caller's unique bus name.
    pub caller: String,
}

/// The user's answer.
pub enum Reply {
    Secret {
        secret: Zeroizing<String>,
        choice: bool,
    },
    Yes {
        choice: bool,
    },
    No,
}

type Handler = Arc<
    dyn Fn(Request, watch::Receiver<bool>) -> Pin<Box<dyn Future<Output = Reply> + Send>>
        + Send
        + Sync,
>;

struct Session {
    sender: String,
    callback: OwnedObjectPath,
    exchange: exchange::Exchange,
    /// Dismisses the prompt on screen when the caller stops.
    dismiss: Option<watch::Sender<bool>>,
}

#[derive(Default)]
struct Turns {
    active: Option<Session>,
    waiting: VecDeque<(String, OwnedObjectPath)>,
}

#[derive(Clone)]
struct Prompter {
    conn: Connection,
    turns: Arc<Mutex<Turns>>,
    handler: Handler,
}

/// A property's value, unwrapped of gcr's doubled variants.
fn unwrap(value: &OwnedValue) -> Value<'static> {
    let mut v: Value<'static> = value
        .try_clone()
        .map(Value::from)
        .unwrap_or(Value::from(""));
    while let Value::Value(inner) = v {
        v = *inner;
    }
    v
}

fn text(props: &HashMap<String, OwnedValue>, key: &str) -> String {
    match props.get(key).map(unwrap) {
        Some(Value::Str(s)) => s.to_string(),
        _ => String::new(),
    }
}

fn flag(props: &HashMap<String, OwnedValue>, key: &str) -> bool {
    matches!(props.get(key).map(unwrap), Some(Value::Bool(true)))
}

impl Prompter {
    /// The next caller in line takes its turn: told the prompter is ready, with its key.
    fn next_turn(&self) {
        let mut turns = self.turns.lock().unwrap();
        if turns.active.is_some() {
            return;
        }
        let Some((sender, callback)) = turns.waiting.pop_front() else {
            return;
        };
        let exchange = match exchange::Exchange::new() {
            Ok(e) => e,
            Err(err) => {
                warn!("the keyring prompter has no randomness: {err}");
                return;
            }
        };
        let opening = exchange.begin();
        turns.active = Some(Session {
            sender: sender.clone(),
            callback: callback.clone(),
            exchange,
            dismiss: None,
        });
        drop(turns);
        let conn = self.conn.clone();
        tokio::spawn(async move {
            let props: HashMap<&str, Value<'_>> = HashMap::new();
            if let Err(err) = conn
                .call_method(
                    Some(sender.as_str()),
                    callback.as_str(),
                    Some(CALLBACK),
                    "PromptReady",
                    &("", props, opening),
                )
                .await
            {
                debug!("the keyring's caller went: {err}");
            }
        });
    }

    /// The active caller stops, or leaves: its prompt goes and the next takes its turn.
    fn end_turn(&self, sender: &str, done: bool) {
        let mut turns = self.turns.lock().unwrap();
        turns.waiting.retain(|(s, _)| s != sender);
        let ended = match &turns.active {
            Some(s) if s.sender == sender => turns.active.take(),
            _ => None,
        };
        drop(turns);
        if let Some(session) = ended {
            if let Some(dismiss) = &session.dismiss {
                let _ = dismiss.send(true);
            }
            if done {
                let conn = self.conn.clone();
                tokio::spawn(async move {
                    let _ = conn
                        .call_method(
                            Some(session.sender.as_str()),
                            session.callback.as_str(),
                            Some(CALLBACK),
                            "PromptDone",
                            &(),
                        )
                        .await;
                });
            }
        }
        self.next_turn();
    }
}

#[interface(name = "org.gnome.keyring.internal.Prompter")]
impl Prompter {
    async fn begin_prompting(&self, callback: ObjectPath<'_>, #[zbus(header)] header: Header<'_>) {
        let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
        self.turns
            .lock()
            .unwrap()
            .waiting
            .push_back((sender, callback.into()));
        self.next_turn();
    }

    async fn perform_prompt(
        &self,
        callback: ObjectPath<'_>,
        r#type: String,
        properties: HashMap<String, OwnedValue>,
        exchange: String,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<()> {
        let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
        let (dismissed, key, own) = {
            let mut turns = self.turns.lock().unwrap();
            let Some(session) = turns
                .active
                .as_mut()
                .filter(|s| s.sender == sender && s.callback.as_str() == callback.as_str())
            else {
                return Err(fdo::Error::Failed("not this caller's turn".into()));
            };
            let (dismiss, dismissed) = watch::channel(false);
            session.dismiss = Some(dismiss);
            let key = session.exchange.key(&exchange);
            (dismissed, key, session.exchange.begin())
        };
        let request = Request {
            confirm: r#type == "confirm",
            title: text(&properties, "title"),
            message: text(&properties, "message"),
            description: text(&properties, "description"),
            warning: text(&properties, "warning"),
            choice_label: Some(text(&properties, "choice-label")).filter(|s| !s.is_empty()),
            choice_chosen: flag(&properties, "choice-chosen"),
            continue_label: text(&properties, "continue-label"),
            cancel_label: text(&properties, "cancel-label"),
            password_new: flag(&properties, "password-new"),
            caller: sender.clone(),
        };
        info!(title = request.title, "the keyring asks");
        let prompter = self.clone();
        let callback: OwnedObjectPath = callback.into();
        tokio::spawn(async move {
            let reply = (prompter.handler)(request, dismissed).await;
            let (answer, choice, closing) = match reply {
                Reply::Secret { secret, choice } => {
                    let closing = key.as_ref().and_then(|k| {
                        prompter
                            .turns
                            .lock()
                            .unwrap()
                            .active
                            .as_ref()
                            .and_then(|s| s.exchange.send(k, secret.as_bytes()))
                    });
                    match closing {
                        Some(closing) => ("yes", choice, closing),
                        None => ("no", false, own),
                    }
                }
                Reply::Yes { choice } => ("yes", choice, own),
                Reply::No => ("no", false, own),
            };
            // gcr wraps each value twice, and its callers unwrap twice.
            let mut props: HashMap<&str, Value<'_>> = HashMap::new();
            props.insert("choice-chosen", Value::Value(Box::new(Value::from(choice))));
            if let Err(err) = prompter
                .conn
                .call_method(
                    Some(sender.as_str()),
                    callback.as_str(),
                    Some(CALLBACK),
                    "PromptReady",
                    &(answer, props, closing),
                )
                .await
            {
                debug!("the keyring's caller went: {err}");
            }
        });
        Ok(())
    }

    async fn stop_prompting(&self, callback: ObjectPath<'_>, #[zbus(header)] header: Header<'_>) {
        let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
        let _ = callback;
        self.end_turn(&sender, true);
    }
}

/// Serves the prompter on `conn` and owns its name, refusing to hand it to a later owner;
/// `Ok(false)` when another prompter holds it. Callers that leave the bus lose their turn.
pub async fn serve<F, Fut>(conn: &Connection, handler: F) -> zbus::Result<bool>
where
    F: Fn(Request, watch::Receiver<bool>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Reply> + Send + 'static,
{
    let handler: Handler =
        Arc::new(move |request, dismissed| Box::pin(handler(request, dismissed)));
    let prompter = Prompter {
        conn: conn.clone(),
        turns: Default::default(),
        handler,
    };
    conn.object_server().at(PATH, prompter.clone()).await?;
    use zbus::fdo::{RequestNameFlags, RequestNameReply};
    match conn
        .request_name_with_flags(
            NAME,
            RequestNameFlags::DoNotQueue | RequestNameFlags::ReplaceExisting,
        )
        .await
    {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => {}
        Ok(_) | Err(zbus::Error::NameTaken) => {
            conn.object_server().remove::<Prompter, _>(PATH).await?;
            return Ok(false);
        }
        Err(err) => return Err(err),
    }
    let dbus = fdo::DBusProxy::new(conn).await?;
    let mut owners = dbus.receive_name_owner_changed().await?;
    tokio::spawn(async move {
        while let Some(signal) = owners.next().await {
            if let Ok(args) = signal.args()
                && args.new_owner().is_none()
                && args.name().starts_with(':')
            {
                prompter.end_turn(args.name().as_str(), false);
            }
        }
    });
    Ok(true)
}
