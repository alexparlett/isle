//! The greeter's side of greetd's IPC (`greetd-ipc(7)`): each message a 32-bit native-endian
//! length and a JSON object tagged by `type`. greetd runs PAM; the greeter answers its
//! questions (`p14b-greeter.md` choices 5 and 6).

use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use zeroize::Zeroizing;

/// A request too large to be greetd's is refused rather than read.
const LIMIT: u32 = 1 << 20;

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request<'a> {
    CreateSession {
        username: &'a str,
    },
    PostAuthMessageResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<&'a str>,
    },
    StartSession {
        cmd: &'a [String],
        env: &'a [String],
    },
    CancelSession,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ErrorType {
    AuthError,
    Error,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MessageType {
    Visible,
    Secret,
    Info,
    Error,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    Success,
    Error {
        error_type: ErrorType,
        description: String,
    },
    AuthMessage {
        auth_message_type: MessageType,
        auth_message: String,
    },
}

/// Where a conversation stands after greetd's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// PAM asks a question: the field, its text the placeholder, shown in clear unless
    /// `secret`. `messages` are PAM's info and error lines that came before it.
    Ask {
        prompt: String,
        secret: bool,
        messages: Vec<String>,
    },
    /// Authenticated: the session can be started.
    Ready { messages: Vec<String> },
    /// PAM refused: a wrong answer, or an account locked out. The session is cancelled.
    Refused {
        description: String,
        messages: Vec<String>,
    },
    /// greetd or PAM could not go on. The session is cancelled.
    Broken(String),
}

impl Step {
    /// The minutes left of a `pam_faillock` lockout, when the account is locked out.
    pub fn lockout(&self) -> Option<u32> {
        let Step::Refused { messages, .. } = self else {
            return None;
        };
        crate::Outcome {
            result: crate::Result::Fail(String::new()),
            messages: messages.clone(),
        }
        .lockout()
    }
}

/// One connection to greetd, which holds at most one session in the making.
pub struct Greetd {
    stream: UnixStream,
}

impl Greetd {
    pub async fn connect(socket: &Path) -> std::io::Result<Self> {
        Ok(Self {
            stream: UnixStream::connect(socket).await?,
        })
    }

    /// `GREETD_SOCK`, which greetd sets only for its greeter.
    pub async fn from_env() -> std::io::Result<Self> {
        let socket = std::env::var_os("GREETD_SOCK")
            .ok_or_else(|| std::io::Error::other("GREETD_SOCK is not set"))?;
        Self::connect(Path::new(&socket)).await
    }

    /// Begins signing `user` in.
    pub async fn create(&mut self, user: &str) -> std::io::Result<Step> {
        let response = self
            .send(&Request::CreateSession { username: user })
            .await?;
        self.step(response).await
    }

    /// Answers the question asked; `None` for a question that takes no answer.
    pub async fn answer(&mut self, answer: Option<&str>) -> std::io::Result<Step> {
        let response = self
            .send(&Request::PostAuthMessageResponse { response: answer })
            .await?;
        self.step(response).await
    }

    /// Starts the authenticated session; greetd runs it once the greeter exits. An error
    /// is greetd's description.
    pub async fn start(
        &mut self,
        cmd: &[String],
        env: &[String],
    ) -> std::io::Result<Result<(), String>> {
        Ok(
            match self.send(&Request::StartSession { cmd, env }).await? {
                Response::Success => Ok(()),
                Response::Error { description, .. } => Err(description),
                Response::AuthMessage { .. } => Err("greetd asked a question at the start".into()),
            },
        )
    }

    /// Gives up the session in the making, if there is one.
    pub async fn cancel(&mut self) -> std::io::Result<()> {
        self.send(&Request::CancelSession).await.map(drop)
    }

    /// Reads on from `response` to the next question or the end, acknowledging PAM's info
    /// and error lines on the way.
    async fn step(&mut self, mut response: Response) -> std::io::Result<Step> {
        let mut messages = Vec::new();
        loop {
            let step = match response {
                Response::Success => Step::Ready { messages },
                Response::AuthMessage {
                    auth_message_type: kind @ (MessageType::Visible | MessageType::Secret),
                    auth_message,
                } => Step::Ask {
                    prompt: auth_message,
                    secret: kind == MessageType::Secret,
                    messages,
                },
                Response::AuthMessage { auth_message, .. } => {
                    let text = auth_message.trim_end().to_owned();
                    if !text.is_empty() {
                        messages.push(text);
                    }
                    response = self
                        .send(&Request::PostAuthMessageResponse { response: None })
                        .await?;
                    continue;
                }
                Response::Error {
                    error_type,
                    description,
                } => {
                    // A refused session is greetd's to forget only once it is cancelled.
                    let _ = self.cancel().await;
                    match error_type {
                        ErrorType::AuthError => Step::Refused {
                            description,
                            messages,
                        },
                        ErrorType::Error => Step::Broken(description),
                    }
                }
            };
            return Ok(step);
        }
    }

    async fn send(&mut self, request: &Request<'_>) -> std::io::Result<Response> {
        let body = Zeroizing::new(serde_json::to_vec(request).map_err(std::io::Error::other)?);
        let length = u32::try_from(body.len()).map_err(std::io::Error::other)?;
        let mut frame = Zeroizing::new(Vec::with_capacity(4 + body.len()));
        frame.extend_from_slice(&length.to_ne_bytes());
        frame.extend_from_slice(&body);
        self.stream.write_all(&frame).await?;
        let mut length = [0u8; 4];
        self.stream.read_exact(&mut length).await?;
        let length = u32::from_ne_bytes(length);
        if length > LIMIT {
            return Err(std::io::Error::other("greetd's answer is too long"));
        }
        let mut body = vec![0u8; length as usize];
        self.stream.read_exact(&mut body).await?;
        serde_json::from_slice(&body).map_err(std::io::Error::other)
    }
}
