//! The lock's password check, run in the `isle-auth` helper process so PAM's modules never
//! load into the compositor (`p9-lock-and-prompts.md` choice 4), and the compositor's side of
//! its line protocol.
//!
//! To the helper: `attempt PASSWORD`, then `quit`. From it, for each attempt: any number of
//! `info TEXT` and `error TEXT` as PAM says them, then one of `ok`, `fail TEXT` or
//! `broken TEXT`. Text is escaped so each message is one line.

use std::{path::Path, process::Stdio};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
};
use zeroize::Zeroizing;

/// `\` and line breaks escaped, so a message or a password is one line.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

pub fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// How an attempt ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Result {
    /// The password was right and the account may log in.
    Ok,
    /// PAM refused it: a wrong password, or an account locked out.
    Fail(String),
    /// PAM could not check it at all.
    Broken(String),
}

/// One attempt's outcome, with the messages PAM sent on the way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub result: Result,
    pub messages: Vec<String>,
}

impl Outcome {
    /// The minutes left of a `pam_faillock` lockout, when the account is locked out.
    pub fn lockout(&self) -> Option<u32> {
        if !self
            .messages
            .iter()
            .any(|m| m.starts_with("The account is locked due to"))
        {
            return None;
        }
        let minutes = self.messages.iter().find_map(|m| {
            let rest = m.trim().strip_prefix('(')?;
            let (number, unit) = rest.split_once(' ')?;
            unit.starts_with("minute").then(|| number.parse().ok())?
        });
        Some(minutes.unwrap_or(1))
    }
}

/// The helper, started for a lock and ended with it.
pub struct Helper {
    _child: Child,
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
}

impl Helper {
    /// Starts `program` with `env` added; it is killed when the helper is dropped.
    pub fn spawn(program: &Path, env: &[(&str, &str)]) -> std::io::Result<Self> {
        let mut command = Command::new(program);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            // pam_faillock's messages are read in their untranslated form.
            .env("LC_ALL", "C");
        for (key, value) in env {
            command.env(key, value);
        }
        let mut child = command.spawn()?;
        let stdin = child.stdin.take().expect("piped");
        let stdout = child.stdout.take().expect("piped");
        Ok(Self {
            _child: child,
            stdin,
            lines: BufReader::new(stdout).lines(),
        })
    }

    /// Checks `password`; an error is the helper gone.
    pub async fn attempt(&mut self, password: &str) -> std::io::Result<Outcome> {
        let line = Zeroizing::new(format!("attempt {}\n", escape(password)));
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;
        let mut messages = Vec::new();
        loop {
            let Some(line) = self.lines.next_line().await? else {
                return Err(std::io::ErrorKind::UnexpectedEof.into());
            };
            let (verb, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
            let text = unescape(rest);
            let result = match verb {
                "info" | "error" => {
                    messages.push(text);
                    continue;
                }
                "ok" => Result::Ok,
                "fail" => Result::Fail(text),
                "broken" => Result::Broken(text),
                _ => continue,
            };
            return Ok(Outcome { result, messages });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_keeps_a_message_on_one_line_and_comes_back_whole() {
        let text = "two\nlines \\ and\r a return";
        let escaped = escape(text);
        assert!(!escaped.contains('\n') && !escaped.contains('\r'));
        assert_eq!(unescape(&escaped), text);
    }

    #[test]
    fn faillocks_messages_read_as_a_lockout_and_its_minutes() {
        let outcome = Outcome {
            result: Result::Fail("Authentication failure".into()),
            messages: vec![
                "The account is locked due to 3 failed logins.".into(),
                "(10 minutes left to unlock)".into(),
            ],
        };
        assert_eq!(outcome.lockout(), Some(10));
        let wrong = Outcome {
            result: Result::Fail("Authentication failure".into()),
            messages: vec![],
        };
        assert_eq!(wrong.lockout(), None);
    }
}
