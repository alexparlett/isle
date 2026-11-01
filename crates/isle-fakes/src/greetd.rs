//! A fake greetd on a Unix socket: users with a password and a script of PAM's messages,
//! answering a greeter as greetd does, and recording the sessions it was asked to start.
//! It parses the wire format itself, so a greeter's own types are checked against it.

use std::{
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde_json::{Value, json};

/// What PAM says for a user, in order, before and after the password.
#[derive(Clone, Debug)]
pub enum Say {
    Info(String),
    Error(String),
    /// A question shown in clear; any answer passes.
    Visible(String),
    /// The password, checked against the user's.
    Password,
    /// Another secret question; any answer passes.
    Secret(String),
}

#[derive(Clone, Debug)]
pub struct User {
    pub name: String,
    pub password: String,
    pub script: Vec<Say>,
    /// Messages PAM sends with a wrong password, before greetd's `auth_error`.
    pub on_wrong: Vec<Say>,
}

impl User {
    pub fn new(name: &str, password: &str) -> Self {
        Self {
            name: name.into(),
            password: password.into(),
            script: vec![Say::Password],
            on_wrong: Vec::new(),
        }
    }
}

/// A session a greeter asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Started {
    pub user: String,
    pub cmd: Vec<String>,
    pub env: Vec<String>,
}

#[derive(Default)]
struct State {
    started: Vec<Started>,
    /// Every request's type, in order.
    requests: Vec<String>,
    /// Every answer given to a password question, so a test can see none was logged twice.
    answers: u32,
}

pub struct Greetd {
    pub socket: PathBuf,
    state: Arc<Mutex<State>>,
}

impl Greetd {
    /// Listens at `socket` for greeters, one at a time.
    pub fn start(socket: &Path, users: Vec<User>) -> Self {
        let _ = std::fs::remove_file(socket);
        let listener = UnixListener::bind(socket).unwrap();
        let state = Arc::new(Mutex::new(State::default()));
        let shared = state.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                serve(stream, &users, &shared);
            }
        });
        Self {
            socket: socket.to_owned(),
            state,
        }
    }

    pub fn started(&self) -> Vec<Started> {
        self.state.lock().unwrap().started.clone()
    }

    pub fn requests(&self) -> Vec<String> {
        self.state.lock().unwrap().requests.clone()
    }

    pub fn answers(&self) -> u32 {
        self.state.lock().unwrap().answers
    }
}

/// A session in the making: the user and how far through their script it is.
struct Making {
    user: User,
    at: usize,
    wrong: bool,
    /// The wrong password's messages still to send, before the refusal.
    refusing: Option<usize>,
    done: bool,
}

fn serve(mut stream: UnixStream, users: &[User], state: &Mutex<State>) {
    let mut making: Option<Making> = None;
    while let Some(request) = read(&mut stream) {
        let kind = request["type"].as_str().unwrap_or("").to_owned();
        state.lock().unwrap().requests.push(kind.clone());
        let reply = match kind.as_str() {
            "create_session" => {
                let name = request["username"].as_str().unwrap_or("");
                match (making.is_some(), users.iter().find(|u| u.name == name)) {
                    (true, _) => error("error", "a session is already being configured"),
                    (false, None) => {
                        // PAM asks for the password of an unknown user too, and refuses it.
                        let mut unknown = User::new(name, "\u{0}");
                        unknown.script = vec![Say::Password];
                        making = Some(Making::new(unknown));
                        next(making.as_mut().unwrap())
                    }
                    (false, Some(user)) => {
                        making = Some(Making::new(user.clone()));
                        next(making.as_mut().unwrap())
                    }
                }
            }
            "post_auth_message_response" => match making.as_mut() {
                None => error("error", "no session"),
                Some(m) => {
                    if let Some(Say::Password) = m.user.script.get(m.at.wrapping_sub(1))
                        && m.refusing.is_none()
                    {
                        state.lock().unwrap().answers += 1;
                        if request["response"].as_str() != Some(m.user.password.as_str()) {
                            m.wrong = true;
                        }
                    }
                    let reply = next(m);
                    if reply["type"] == "error" {
                        making = None;
                    }
                    reply
                }
            },
            "start_session" => match making.take() {
                Some(m) if m.done => {
                    let strings = |v: &Value| {
                        v.as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|s| s.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default()
                    };
                    state.lock().unwrap().started.push(Started {
                        user: m.user.name.clone(),
                        cmd: strings(&request["cmd"]),
                        env: strings(&request["env"]),
                    });
                    json!({"type": "success"})
                }
                other => {
                    making = other;
                    error("error", "session not authenticated")
                }
            },
            "cancel_session" => {
                making = None;
                json!({"type": "success"})
            }
            _ => error("error", "unknown request"),
        };
        if !write(&mut stream, &reply) {
            return;
        }
    }
}

impl Making {
    fn new(user: User) -> Self {
        Self {
            user,
            at: 0,
            wrong: false,
            refusing: None,
            done: false,
        }
    }
}

/// The next thing greetd says for `m`.
fn next(m: &mut Making) -> Value {
    if m.wrong {
        let at = m.refusing.get_or_insert(0);
        if let Some(say) = m.user.on_wrong.get(*at) {
            *at += 1;
            return message(say);
        }
        return error("auth_error", "Authentication failed");
    }
    match m.user.script.get(m.at) {
        Some(say) => {
            m.at += 1;
            message(say)
        }
        None => {
            m.done = true;
            json!({"type": "success"})
        }
    }
}

fn message(say: &Say) -> Value {
    let (kind, text) = match say {
        Say::Info(t) => ("info", t.as_str()),
        Say::Error(t) => ("error", t.as_str()),
        Say::Visible(t) => ("visible", t.as_str()),
        Say::Password => ("secret", "Password: "),
        Say::Secret(t) => ("secret", t.as_str()),
    };
    json!({"type": "auth_message", "auth_message_type": kind, "auth_message": text})
}

fn error(kind: &str, description: &str) -> Value {
    json!({"type": "error", "error_type": kind, "description": description})
}

fn read(stream: &mut UnixStream) -> Option<Value> {
    let mut length = [0u8; 4];
    stream.read_exact(&mut length).ok()?;
    let mut body = vec![0u8; u32::from_ne_bytes(length) as usize];
    stream.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn write(stream: &mut UnixStream, value: &Value) -> bool {
    let body = serde_json::to_vec(value).unwrap();
    let mut frame = (body.len() as u32).to_ne_bytes().to_vec();
    frame.extend_from_slice(&body);
    stream.write_all(&frame).is_ok()
}
