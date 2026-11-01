//! SSH keys and the agent against the real tools: generated keys load in `ssh-keygen` and
//! `ssh-add`, a key is added and removed with its passphrase, and "In agent" is read past a
//! gcr-shaped proxy that lists every `.pub`.

use std::{
    io::{Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixListener},
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant},
};

use isle_secrets::{
    agent::{self, Agent},
    ssh::{self, DecryptError, GenerateError, NewKind},
};
use ssh_agent_lib::{
    proto::{Identity, PublicCredential, Response},
    ssh_encoding::Encode,
};
use ssh_key::PublicKey;

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("isle-ssh-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct SshAgent(Child);

impl SshAgent {
    fn start(socket: &Path) -> Self {
        let child = Command::new("ssh-agent")
            .arg("-D")
            .arg("-a")
            .arg(socket)
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("ssh-agent");
        let start = Instant::now();
        while !socket.exists() {
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "ssh-agent did not start"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        Self(child)
    }
}

impl Drop for SshAgent {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Answers every listing with each `.pub` in `keys`, as gcr-ssh-agent's proxy does.
fn gcr_proxy(socket: &Path, keys: PathBuf) {
    let listener = UnixListener::bind(socket).unwrap();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let keys = keys.clone();
            std::thread::spawn(move || {
                loop {
                    let mut len = [0u8; 4];
                    if stream.read_exact(&mut len).is_err() {
                        return;
                    }
                    let mut body = vec![0u8; u32::from_be_bytes(len) as usize];
                    if stream.read_exact(&mut body).is_err() {
                        return;
                    }
                    let response = if body.first() == Some(&11) {
                        Response::IdentitiesAnswer(
                            ssh::keys(&keys)
                                .iter()
                                .map(|k| {
                                    let public = PublicKey::from_openssh(&k.public_key).unwrap();
                                    Identity {
                                        credential: PublicCredential::Key(
                                            public.key_data().clone(),
                                        ),
                                        comment: k.comment.clone(),
                                    }
                                })
                                .collect(),
                        )
                    } else {
                        Response::Failure
                    };
                    let mut out = Vec::new();
                    (response.encoded_len().unwrap() as u32)
                        .encode(&mut out)
                        .unwrap();
                    response.encode(&mut out).unwrap();
                    if stream.write_all(&out).is_err() {
                        return;
                    }
                }
            });
        }
    });
}

fn keygen(args: &[&str]) -> String {
    let out = Command::new("ssh-keygen").args(args).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn generated_keys_load_in_openssh_and_match_its_fingerprints() {
    let dir = Dir::new("gen");
    let keys = dir.0.join(".ssh");
    let key = ssh::generate(
        &keys,
        "id_test",
        NewKind::Ed25519,
        "alex@isle",
        "correct horse",
    )
    .unwrap();
    let private = keys.join("id_test");
    assert_eq!(
        std::fs::metadata(&private).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::metadata(&keys).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(key.encrypted, Some(true));

    let derived = keygen(&["-y", "-P", "correct horse", "-f", private.to_str().unwrap()]);
    let line = std::fs::read_to_string(keys.join("id_test.pub")).unwrap();
    assert_eq!(
        derived.split_whitespace().take(2).collect::<Vec<_>>(),
        line.split_whitespace().take(2).collect::<Vec<_>>()
    );
    let listed = keygen(&[
        "-l",
        "-E",
        "sha256",
        "-f",
        keys.join("id_test.pub").to_str().unwrap(),
    ]);
    assert!(
        listed.contains(&key.fingerprint),
        "{listed} lacks {}",
        key.fingerprint
    );

    assert!(matches!(
        ssh::generate(&keys, "id_test", NewKind::Ed25519, "", ""),
        Err(GenerateError::Exists)
    ));
    assert!(matches!(
        ssh::generate(&keys, "../escape", NewKind::Ed25519, "", ""),
        Err(GenerateError::BadName)
    ));

    let socket = dir.0.join("agent.sock");
    let _agent = SshAgent::start(&socket);
    let askpass = dir.0.join("askpass");
    std::fs::write(&askpass, "#!/bin/sh\necho 'correct horse'\n").unwrap();
    std::fs::set_permissions(&askpass, std::fs::Permissions::from_mode(0o700)).unwrap();
    let added = Command::new("ssh-add")
        .arg(&private)
        .env("SSH_AUTH_SOCK", &socket)
        .env("SSH_ASKPASS", &askpass)
        .env("SSH_ASKPASS_REQUIRE", "force")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(
        added.status.success(),
        "{}",
        String::from_utf8_lossy(&added.stderr)
    );
    assert_eq!(
        Agent::at(&socket).fingerprints().unwrap(),
        [key.fingerprint]
    );
}

#[test]
fn an_rsa_key_is_3072_bits() {
    let dir = Dir::new("rsa");
    let key = ssh::generate(&dir.0, "id_rsa", NewKind::Rsa3072, "rsa@isle", "").unwrap();
    assert_eq!(key.kind, "RSA 3072");
    assert_eq!(key.encrypted, Some(false));
    let listed = keygen(&["-l", "-f", dir.0.join("id_rsa").to_str().unwrap()]);
    assert!(
        listed.starts_with("3072 ") && listed.contains(&key.fingerprint),
        "{listed}"
    );
}

#[test]
fn keys_are_read_without_their_passphrase() {
    let dir = Dir::new("read");
    ssh::generate(
        &dir.0,
        "id_ed25519",
        NewKind::Ed25519,
        "locked@isle",
        "pass",
    )
    .unwrap();
    ssh::generate(&dir.0, "deploy", NewKind::Ed25519, "open@isle", "").unwrap();
    keygen(&[
        "-q",
        "-t",
        "rsa",
        "-b",
        "2048",
        "-m",
        "PEM",
        "-N",
        "",
        "-C",
        "pem@isle",
        "-f",
        dir.0.join("legacy").to_str().unwrap(),
    ]);
    std::fs::write(dir.0.join("config"), "Host *\n").unwrap();
    std::fs::write(dir.0.join("known_hosts"), "").unwrap();

    let keys = ssh::keys(&dir.0);
    let names: Vec<&str> = keys.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(names, ["deploy", "id_ed25519", "legacy"]);
    let locked = &keys[1];
    assert_eq!(locked.encrypted, Some(true));
    assert_eq!(
        locked.comment, "locked@isle",
        "an encrypted key's comment comes from its .pub"
    );
    assert_eq!(locked.kind, "Ed25519");
    assert_eq!(keys[0].encrypted, Some(false));
    let legacy = &keys[2];
    assert_eq!(legacy.encrypted, None, "a PEM key is shown by its .pub");
    assert_eq!(legacy.kind, "RSA 2048");
    assert_eq!(legacy.comment, "pem@isle");
}

#[test]
fn added_with_its_passphrase_listed_and_removed() {
    let dir = Dir::new("agent");
    let key = ssh::generate(&dir.0, "id_ed25519", NewKind::Ed25519, "me@isle", "pass").unwrap();
    let socket = dir.0.join("agent.sock");
    let _agent = SshAgent::start(&socket);
    let agent = Agent::at(&socket);
    assert!(agent.fingerprints().unwrap().is_empty());

    assert_eq!(
        ssh::decrypt(&key, "nope").unwrap_err(),
        DecryptError::WrongPassphrase
    );
    let private = ssh::decrypt(&key, "pass").unwrap();
    agent.add(&private).unwrap();
    assert_eq!(agent.fingerprints().unwrap(), [key.fingerprint.clone()]);
    let listed = Command::new("ssh-add")
        .arg("-l")
        .env("SSH_AUTH_SOCK", &socket)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&listed.stdout).contains("me@isle"));

    agent.remove(private.public_key()).unwrap();
    assert!(agent.fingerprints().unwrap().is_empty());
    assert!(matches!(
        Agent::at(&dir.0.join("none")).fingerprints(),
        Err(agent::Error::Missing)
    ));
}

#[test]
fn in_agent_is_read_past_gcrs_proxy() {
    let dir = Dir::new("gcr");
    let keys = dir.0.join("keys");
    let loaded = ssh::generate(&keys, "id_ed25519", NewKind::Ed25519, "loaded", "").unwrap();
    ssh::generate(&keys, "id_other", NewKind::Ed25519, "not loaded", "").unwrap();
    let gcr = dir.0.join("gcr");
    std::fs::create_dir(&gcr).unwrap();
    let _inner = SshAgent::start(&gcr.join(".ssh"));
    gcr_proxy(&gcr.join("ssh"), keys.clone());

    let through_proxy = Agent::at(&gcr.join("ssh")).fingerprints().unwrap();
    assert_eq!(through_proxy.len(), 2, "the proxy lists every .pub");

    let located = agent::locate(&gcr.join("ssh"));
    assert_eq!(located, gcr.join(".ssh"));
    let agent = Agent::at(&located);
    assert!(agent.fingerprints().unwrap().is_empty());
    agent.add(&ssh::decrypt(&loaded, "").unwrap()).unwrap();
    assert_eq!(agent.fingerprints().unwrap(), [loaded.fingerprint]);

    let plain = dir.0.join("plain.sock");
    assert_eq!(
        agent::locate(&plain),
        plain,
        "any other agent is read directly"
    );
}
