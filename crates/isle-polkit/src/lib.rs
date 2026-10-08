//! A polkit authentication agent: the `AuthenticationAgent` interface polkitd calls, its
//! registration for the session, and the conversation with polkit's helper over its socket,
//! the only way to the helper on Arch's polkit 127 (`lock-and-prompts-2026.md` §2).

use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};
use tracing::info;
use zbus::{Connection, interface, zvariant::OwnedValue};
use zeroize::Zeroizing;

pub const AGENT_PATH: &str = "/io/github/alexparlett/Isle/PolkitAgent";
pub const HELPER_SOCKET: &str = "/run/polkit/agent-helper.socket";

/// What polkitd asks the agent to authenticate.
#[derive(Clone, Debug, Default)]
pub struct Request {
    pub action_id: String,
    pub message: String,
    pub icon_name: String,
    pub details: HashMap<String, String>,
    pub cookie: String,
    /// The users who may answer, by uid.
    pub identities: Vec<u32>,
}

impl Request {
    /// The process the authorisation is for: `pkexec`'s caller, Settings.
    pub fn subject_pid(&self) -> Option<i32> {
        self.details
            .get("polkit.subject-pid")
            .or_else(|| self.details.get("polkit.caller-pid"))
            .and_then(|p| p.parse().ok())
    }
}

#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.freedesktop.PolicyKit1.Error")]
pub enum Error {
    #[zbus(error)]
    ZBus(zbus::Error),
    Failed(String),
    Cancelled(String),
}

/// How a request ended.
pub enum Outcome {
    Authorised,
    Cancelled,
}

type Handler = Arc<
    dyn Fn(
            Request,
            tokio::sync::watch::Receiver<bool>,
        ) -> Pin<Box<dyn Future<Output = Outcome> + Send>>
        + Send
        + Sync,
>;

struct Agent {
    handler: Handler,
    /// Each request's cancel, by cookie.
    cancels: Arc<std::sync::Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>>,
}

#[interface(name = "org.freedesktop.PolicyKit1.AuthenticationAgent")]
impl Agent {
    /// polkitd waits on this until the user is authenticated or gives up.
    async fn begin_authentication(
        &self,
        action_id: String,
        message: String,
        icon_name: String,
        details: HashMap<String, String>,
        cookie: String,
        identities: Vec<(String, HashMap<String, OwnedValue>)>,
    ) -> Result<(), Error> {
        let identities = identities
            .into_iter()
            .filter(|(kind, _)| kind == "unix-user")
            .filter_map(|(_, d)| d.get("uid").and_then(|v| u32::try_from(v).ok()))
            .collect();
        info!(action_id, "polkit asks");
        let (cancel, cancelled) = tokio::sync::watch::channel(false);
        self.cancels.lock().unwrap().insert(cookie.clone(), cancel);
        let request = Request {
            action_id,
            message,
            icon_name,
            details,
            cookie: cookie.clone(),
            identities,
        };
        let outcome = (self.handler)(request, cancelled).await;
        self.cancels.lock().unwrap().remove(&cookie);
        match outcome {
            Outcome::Authorised => Ok(()),
            Outcome::Cancelled => Err(Error::Cancelled("The user dismissed the prompt".into())),
        }
    }

    async fn cancel_authentication(&self, cookie: String) {
        if let Some(cancel) = self.cancels.lock().unwrap().get(&cookie) {
            let _ = cancel.send(true);
        }
    }
}

#[zbus::proxy(
    interface = "org.freedesktop.PolicyKit1.Authority",
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority"
)]
trait Authority {
    fn register_authentication_agent(
        &self,
        subject: &(&str, HashMap<&str, zvariant::Value<'_>>),
        locale: &str,
        object_path: &str,
    ) -> zbus::Result<()>;
}

/// Serves the agent on `conn` and registers it for `session`; each request runs `handler`,
/// which is told through its receiver when polkitd cancels.
pub async fn serve<F, Fut>(conn: &Connection, session: &str, handler: F) -> zbus::Result<()>
where
    F: Fn(Request, tokio::sync::watch::Receiver<bool>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Outcome> + Send + 'static,
{
    let handler: Handler =
        Arc::new(move |request, cancelled| Box::pin(handler(request, cancelled)));
    conn.object_server()
        .at(
            AGENT_PATH,
            Agent {
                handler,
                cancels: Default::default(),
            },
        )
        .await?;
    let authority = AuthorityProxy::new(conn).await?;
    let mut details = HashMap::new();
    details.insert("session-id", zvariant::Value::from(session));
    let locale = std::env::var("LANG").unwrap_or_else(|_| "C".into());
    authority
        .register_authentication_agent(&("unix-session", details), &locale, AGENT_PATH)
        .await?;
    info!(session, "the polkit agent is registered");
    Ok(())
}

/// What polkit's helper says, one line at a time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// PAM asks for a secret.
    Secret(String),
    /// PAM asks for something visible: a user name.
    Visible(String),
    Error(String),
    Info(String),
    Success,
    Failure,
}

/// `g_strcompress`: the helper escapes its lines with `g_strescape`, which writes bytes past
/// ASCII as octal.
pub fn unescape(text: &str) -> String {
    let mut out: Vec<u8> = Vec::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        i += 1;
        if b != b'\\' {
            out.push(b);
            continue;
        }
        let Some(&next) = bytes.get(i) else {
            out.push(b'\\');
            break;
        };
        i += 1;
        match next {
            b'n' => out.push(b'\n'),
            b't' => out.push(b'\t'),
            b'r' => out.push(b'\r'),
            b'b' => out.push(8),
            b'f' => out.push(12),
            b'v' => out.push(11),
            b'0'..=b'7' => {
                let mut value = (next - b'0') as u32;
                for _ in 0..2 {
                    match bytes.get(i) {
                        Some(d @ b'0'..=b'7') => {
                            value = value * 8 + (d - b'0') as u32;
                            i += 1;
                        }
                        _ => break,
                    }
                }
                out.push(value as u8);
            }
            other => out.push(other),
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn parse(line: &str) -> Option<Line> {
    let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
    let text = || unescape(rest.trim_end());
    Some(match verb {
        "PAM_PROMPT_ECHO_OFF" => Line::Secret(text()),
        "PAM_PROMPT_ECHO_ON" => Line::Visible(text()),
        "PAM_ERROR_MSG" => Line::Error(text()),
        "PAM_TEXT_INFO" => Line::Info(text()),
        "SUCCESS" => Line::Success,
        "FAILURE" => Line::Failure,
        _ => return None,
    })
}

/// One attempt with polkit's helper: one PAM conversation for `user` and the request's
/// cookie.
pub struct Conversation {
    lines: Lines<BufReader<OwnedReadHalf>>,
    write: OwnedWriteHalf,
}

impl Conversation {
    pub async fn start(socket: &str, user: &str, cookie: &str) -> std::io::Result<Self> {
        let stream = UnixStream::connect(socket).await?;
        let (read, mut write) = stream.into_split();
        write
            .write_all(format!("{user}\n{cookie}\n").as_bytes())
            .await?;
        write.flush().await?;
        Ok(Self {
            lines: BufReader::new(read).lines(),
            write,
        })
    }

    /// The helper's next line; `None` when it hung up.
    pub async fn next(&mut self) -> std::io::Result<Option<Line>> {
        loop {
            let Some(line) = self.lines.next_line().await? else {
                return Ok(None);
            };
            if let Some(parsed) = parse(&line) {
                return Ok(Some(parsed));
            }
        }
    }

    /// Answers PAM's prompt; a newline cannot be part of the answer.
    pub async fn respond(&mut self, answer: &str) -> std::io::Result<()> {
        let line = Zeroizing::new(format!("{}\n", answer.replace('\n', "")));
        self.write.write_all(line.as_bytes()).await?;
        self.write.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_helpers_lines_are_read_and_unescaped() {
        assert_eq!(
            parse("PAM_PROMPT_ECHO_OFF Password: "),
            Some(Line::Secret("Password:".into()))
        );
        assert_eq!(
            parse("PAM_TEXT_INFO Place your finger\\non the reader"),
            Some(Line::Info("Place your finger\non the reader".into()))
        );
        assert_eq!(
            parse("PAM_ERROR_MSG caf\\303\\251"),
            Some(Line::Error("café".into()))
        );
        assert_eq!(parse("SUCCESS"), Some(Line::Success));
        assert_eq!(parse("FAILURE"), Some(Line::Failure));
        assert_eq!(parse("noise"), None);
    }
}
