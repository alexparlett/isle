//! The SSH agent: which keys it holds, read past gcr's proxy, and keys added and removed
//! (choice 14).

use std::{
    os::unix::{fs::FileTypeExt, net::UnixStream},
    path::{Path, PathBuf},
    time::Duration,
};

use ssh_agent_lib::{
    blocking::Client,
    proto::{AddIdentity, PrivateCredential, PublicCredential, RemoveIdentity},
};
use ssh_key::{HashAlg, PrivateKey, PublicKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Agent {
    socket: PathBuf,
}

fn is_socket(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.file_type().is_socket())
}

/// The socket that tells which keys are loaded: gcr's inner agent beside its proxy, or the
/// agent `auth_sock` names.
pub fn locate(auth_sock: &Path) -> PathBuf {
    if let Some(dir) = auth_sock.parent()
        && dir.file_name().is_some_and(|n| n == "gcr")
    {
        let inner = dir.join(".ssh");
        if is_socket(&inner) {
            return inner;
        }
    }
    auth_sock.to_owned()
}

#[derive(Debug)]
pub enum Error {
    /// No agent is running.
    Missing,
    Refused,
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => write!(f, "no SSH agent is running"),
            Self::Refused => write!(f, "the SSH agent refused"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl Agent {
    /// The session's agent, from `SSH_AUTH_SOCK`.
    pub fn find() -> Option<Self> {
        let sock = std::env::var_os("SSH_AUTH_SOCK")?;
        Some(Self::at(&locate(Path::new(&sock))))
    }

    pub fn at(socket: &Path) -> Self {
        Self {
            socket: socket.to_owned(),
        }
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    fn client(&self) -> Result<Client<UnixStream>, Error> {
        let stream = UnixStream::connect(&self.socket).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused => Error::Missing,
            _ => Error::Io(e),
        })?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
        Ok(Client::new(stream))
    }

    /// The SHA256 fingerprints of the keys the agent holds.
    pub fn fingerprints(&self) -> Result<Vec<String>, Error> {
        let identities = self
            .client()?
            .request_identities()
            .map_err(|_| Error::Refused)?;
        Ok(identities
            .iter()
            .map(|i| {
                i.credential
                    .key_data()
                    .fingerprint(HashAlg::Sha256)
                    .to_string()
            })
            .collect())
    }

    /// Adds a decrypted key.
    pub fn add(&self, key: &PrivateKey) -> Result<(), Error> {
        let credential = PrivateCredential::Key {
            privkey: key.key_data().clone(),
            comment: key.comment().to_owned(),
        };
        self.client()?
            .add_identity(AddIdentity { credential })
            .map_err(|_| Error::Refused)
    }

    pub fn remove(&self, key: &PublicKey) -> Result<(), Error> {
        self.client()?
            .remove_identity(RemoveIdentity {
                credential: PublicCredential::Key(key.key_data().clone()),
            })
            .map_err(|_| Error::Refused)
    }
}
