//! `org.freedesktop.PolicyKit1`: an authority an agent registers with, which asks it to
//! authenticate as polkitd does, and which answers `CheckAuthorization`; and polkit's helper on
//! a socket of the test's own, which takes one password.

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

/// How `CheckAuthorization` answers: at once, or by asking the registered agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Allow,
    Refuse,
    AskAgent,
}

struct Authority {
    agent: Arc<Mutex<Option<Registered>>>,
    answer: Arc<Mutex<Answer>>,
    checks: Arc<Mutex<Vec<Check>>>,
}

/// A `CheckAuthorization` call as the authority received it.
#[derive(Clone, Debug)]
pub struct Check {
    pub action: String,
    pub subject_kind: String,
    pub interactive: bool,
}

async fn begin_with(
    conn: &Connection,
    agent: &Registered,
    cookie: &str,
    action: &str,
    message: &str,
    details: HashMap<&str, &str>,
    uids: &[u32],
) -> Result<(), String> {
    let identities: Vec<(&str, HashMap<&str, zbus::zvariant::Value<'_>>)> = uids
        .iter()
        .map(|uid| {
            let mut d = HashMap::new();
            d.insert("uid", zbus::zvariant::Value::from(*uid));
            ("unix-user", d)
        })
        .collect();
    let proxy = zbus::Proxy::new(
        conn,
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

    async fn check_authorization(
        &self,
        subject: (String, HashMap<String, OwnedValue>),
        action_id: String,
        _details: HashMap<String, String>,
        flags: u32,
        _cancellation_id: String,
        #[zbus(connection)] conn: &Connection,
    ) -> (bool, bool, HashMap<String, String>) {
        let interactive = flags & 1 != 0;
        let n = {
            let mut checks = self.checks.lock().unwrap();
            checks.push(Check {
                action: action_id.clone(),
                subject_kind: subject.0,
                interactive,
            });
            checks.len()
        };
        let answer = *self.answer.lock().unwrap();
        let allowed = match answer {
            Answer::Allow => true,
            Answer::Refuse => false,
            Answer::AskAgent => {
                let agent = self.agent.lock().unwrap().clone();
                match agent {
                    Some(agent) if interactive => begin_with(
                        conn,
                        &agent,
                        &format!("check-{n}"),
                        &action_id,
                        "",
                        HashMap::new(),
                        &[unsafe { libc::getuid() }],
                    )
                    .await
                    .is_ok(),
                    _ => false,
                }
            }
        };
        (allowed, !allowed && !interactive, HashMap::new())
    }
}

/// A polkitd on the bus at an address.
pub struct Polkit {
    conn: Connection,
    agent: Arc<Mutex<Option<Registered>>>,
    answer: Arc<Mutex<Answer>>,
    checks: Arc<Mutex<Vec<Check>>>,
}

impl Polkit {
    pub async fn start(address: &str) -> zbus::Result<Self> {
        let agent = Arc::new(Mutex::new(None));
        let answer = Arc::new(Mutex::new(Answer::AskAgent));
        let checks = Arc::new(Mutex::new(Vec::new()));
        let conn = zbus::connection::Builder::address(address)?
            .serve_at(
                PATH,
                Authority {
                    agent: agent.clone(),
                    answer: answer.clone(),
                    checks: checks.clone(),
                },
            )?
            .name("org.freedesktop.PolicyKit1")?
            .build()
            .await?;
        Ok(Self {
            conn,
            agent,
            answer,
            checks,
        })
    }

    /// How `CheckAuthorization` answers from now on; it asks the agent until set.
    pub fn answer(&self, answer: Answer) {
        *self.answer.lock().unwrap() = answer;
    }

    /// Every `CheckAuthorization` call so far.
    pub fn checks(&self) -> Vec<Check> {
        self.checks.lock().unwrap().clone()
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
        begin_with(&self.conn, &agent, cookie, action, message, details, uids).await
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
