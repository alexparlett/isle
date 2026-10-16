//! The variables Isle's own children get on top of its environment: Isle's sockets, named
//! once they exist, so the process environment is never written while threads run.

use std::sync::RwLock;

static VARS: RwLock<Vec<(&'static str, String)>> = RwLock::new(Vec::new());

pub fn set(key: &'static str, value: String) {
    let mut vars = VARS.write().unwrap();
    vars.retain(|(k, _)| *k != key);
    vars.push((key, value));
}

/// For `Command::envs`.
pub fn vars() -> Vec<(&'static str, String)> {
    VARS.read().unwrap().clone()
}
