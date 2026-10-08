//! `org.freedesktop.PolicyKit1`: an authority an agent registers with, which asks it to
//! authenticate as polkitd does; and polkit's helper on a socket of the test's own, which
//! takes one password.

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

use zbus::{Connection, message::Header, zvariant::OwnedValue};

const PATH: &str = "/org/freedesktop/PolicyKit1/Authority";

#[derive(Clone, Debug, Default)]
struct Registered {
    sender: String,
    path: String,
    session: String,
}

struct Authority {
    agent: Arc<Mutex<Option<Registered>>>,
}

#[zbus::interface(name = "org.freedesktop.PolicyKit1.Authority")]
impl Authority {
    fn register_authentication_agent(
        &self,
        subject: (String, HashMap<String, OwnedValue>),
        _locale: String,
        object_path: String,
        #[zbus(header)] header: Header<'_>,
    ) {
        let session = subject
            .1
            .get("session-id")
            .and_then(|v| String::try_from(v.clone()).ok())
            .unwrap_or_default();
        *self.agent.lock().unwrap() = Some(Registered {
            sender: header.sender().map(|s| s.to_string()).unwrap_or_default(),
            path: object_path,
            session,
        });
    }
}

/// A polkitd on the bus at an address.
pub struct Polkit {
    conn: Connection,
    agent: Arc<Mutex<Option<Registered>>>,
}

impl Polkit {
    pub async fn start(address: &str) -> zbus::Result<Self> {
        let agent = Arc::new(Mutex::new(None));
        let conn = zbus::connection::Builder::address(address)?
            .serve_at(
                PATH,
                Authority {
                    agent: agent.clone(),
                },
            )?
            .name("org.freedesktop.PolicyKit1")?
            .build()
            .await?;
        Ok(Self { conn, agent })
    }

    /// The session the agent registered for, once one has.
    pub fn registered(&self) -> Option<String> {
        self.agent
            .lock()
            .unwrap()
            .as_ref()
            .map(|a| a.session.clone())
    }

    /// Asks the agent to authenticate one of `uids` for `action`, as polkitd does for a
    /// caller; `Ok` once authenticated, the agent's error otherwise.
    pub async fn begin(
        &self,
        cookie: &str,
        action: &str,
        message: &str,
        details: HashMap<&str, &str>,
        uids: &[u32],
    ) -> Result<(), String> {
        let agent = self
            .agent
            .lock()
            .unwrap()
            .clone()
            .ok_or("no agent registered")?;
        let identities: Vec<(&str, HashMap<&str, zbus::zvariant::Value<'_>>)> = uids
            .iter()
            .map(|uid| {
                let mut d = HashMap::new();
                d.insert("uid", zbus::zvariant::Value::from(*uid));
                ("unix-user", d)
            })
            .collect();
        let proxy = zbus::Proxy::new(
            &self.conn,
            agent.sender.as_str(),
            agent.path.as_str(),
            "org.freedesktop.PolicyKit1.AuthenticationAgent",
        )
        .await
        .map_err(|e| e.to_string())?;
        proxy
            .call_method(
                "BeginAuthentication",
                &(action, message, "", details, cookie, identities),
            )
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub async fn cancel(&self, cookie: &str) -> Result<(), String> {
        let agent = self
            .agent
            .lock()
            .unwrap()
            .clone()
            .ok_or("no agent registered")?;
        let proxy = zbus::Proxy::new(
            &self.conn,
            agent.sender.as_str(),
            agent.path.as_str(),
            "org.freedesktop.PolicyKit1.AuthenticationAgent",
        )
        .await
        .map_err(|e| e.to_string())?;
        proxy
            .call_method("CancelAuthentication", &(cookie,))
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// polkit's helper on a socket: asks for a password and succeeds on `password`, else fails.
pub struct Helper {
    pub path: PathBuf,
    attempts: Arc<AtomicU32>,
}

impl Helper {
    pub fn start(path: &Path, password: &str) -> std::io::Result<Self> {
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path)?;
        let attempts = Arc::new(AtomicU32::new(0));
        let (count, password) = (attempts.clone(), password.to_owned());
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                count.fetch_add(1, Ordering::Relaxed);
                let password = password.clone();
                std::thread::spawn(move || {
                    let mut write = stream.try_clone().unwrap();
                    let mut lines = BufReader::new(stream).lines();
                    let (Some(Ok(_user)), Some(Ok(_cookie))) = (lines.next(), lines.next()) else {
                        return;
                    };
                    let _ = writeln!(write, "PAM_PROMPT_ECHO_OFF Password: ");
                    let answer = lines.next().and_then(Result::ok).unwrap_or_default();
                    if answer == password {
                        let _ = writeln!(write, "SUCCESS");
                    } else {
                        let _ = writeln!(write, "PAM_ERROR_MSG Sorry\\054 try again.");
                        let _ = writeln!(write, "FAILURE");
                    }
                });
            }
        });
        Ok(Self {
            path: path.to_owned(),
            attempts,
        })
    }

    /// Conversations the agent started.
    pub fn attempts(&self) -> u32 {
        self.attempts.load(Ordering::Relaxed)
    }
}
