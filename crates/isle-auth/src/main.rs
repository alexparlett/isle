//! `isle-auth`: checks the lock's password through PAM for the compositor that started it,
//! one transaction per `attempt` line on standard input (`isle_auth`'s protocol).

mod pam;

use std::{
    ffi::{CStr, CString},
    io::{BufRead, Write},
    path::Path,
};

use isle_auth::{escape, unescape};
use zeroize::Zeroizing;

/// Isle's own service, installed by its package; `login` stands in while it is missing.
const SERVICE: &str = "isle";
const FALLBACK: &str = "login";

fn main() {
    unsafe {
        // Never a core with a password in it, and gone with the compositor.
        libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0);
        if libc::getppid() == 1 {
            return;
        }
    }
    let Some(user) = user_name() else {
        say("broken", "this user has no name in the password database");
        return;
    };
    let confdir = confdir();
    let service = service(confdir.as_deref());
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line.map(Zeroizing::new) else {
            return;
        };
        let (verb, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        match verb {
            "attempt" => {
                let password = Zeroizing::new(unescape(rest));
                attempt(&service, &user, confdir.as_deref(), password);
            }
            "quit" => return,
            _ => {}
        }
    }
}

fn say(verb: &str, text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{verb} {}", escape(text));
    let _ = out.flush();
}

fn user_name() -> Option<CString> {
    unsafe {
        let entry = libc::getpwuid(libc::getuid());
        if entry.is_null() || (*entry).pw_name.is_null() {
            return None;
        }
        Some(CStr::from_ptr((*entry).pw_name).to_owned())
    }
}

/// The tests' own PAM configuration, in builds that take one.
fn confdir() -> Option<CString> {
    if !cfg!(feature = "test-hooks") {
        return None;
    }
    std::env::var("ISLE_PAM_CONFDIR")
        .ok()
        .and_then(|dir| CString::new(dir).ok())
}

/// Isle's service where it is installed; a missing one would fall to `other`, which denies
/// everything.
fn service(confdir: Option<&CStr>) -> CString {
    let installed = match confdir {
        Some(dir) => Path::new(dir.to_str().unwrap_or_default())
            .join(SERVICE)
            .exists(),
        None => ["/etc/pam.d", "/usr/lib/pam.d"]
            .iter()
            .any(|dir| Path::new(dir).join(SERVICE).exists()),
    };
    if installed {
        CString::new(SERVICE).unwrap()
    } else {
        eprintln!("isle-auth: no `{SERVICE}` PAM service is installed; using `{FALLBACK}`");
        CString::new(FALLBACK).unwrap()
    }
}

fn attempt(service: &CStr, user: &CStr, confdir: Option<&CStr>, password: Zeroizing<String>) {
    let mut conversation =
        pam::Conversation::new(user.to_owned(), password, |style, text| match style {
            pam::Message::Info => say("info", text),
            pam::Message::Error => say("error", text),
        });
    match pam::check(service, user, confdir, &mut conversation) {
        Ok(()) => say("ok", ""),
        Err(pam::Failure::Refused(text)) => say("fail", &text),
        Err(pam::Failure::Broken(text)) => say("broken", &text),
    }
}
