//! SSH keys in `~/.ssh`, read without a passphrase, generated, and decrypted to add to the
//! agent (choices 13 and 15).

use std::{
    fs,
    io::{self, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

use ssh_key::{
    EcdsaCurve, HashAlg, LineEnding, PrivateKey, PublicKey,
    private::{KeypairData, RsaKeypair},
    public::KeyData,
    rand_core::OsRng,
};

/// A key pair, or a public key whose private half is in a format other than OpenSSH's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshKey {
    /// The private key's file name, or the public key's without `.pub`.
    pub name: String,
    pub private: Option<PathBuf>,
    pub public: Option<PathBuf>,
    /// "Ed25519", "RSA 3072", "ECDSA P-256".
    pub kind: String,
    /// `SHA256:…`.
    pub fingerprint: String,
    pub comment: String,
    /// Whether the private key has a passphrase; `None` when it could not be read.
    pub encrypted: Option<bool>,
    /// The public key as an `authorized_keys` line.
    pub public_key: String,
}

fn kind_of(key: &KeyData) -> String {
    match key {
        KeyData::Ed25519(_) => "Ed25519".into(),
        KeyData::Rsa(rsa) => match rsa.n.as_positive_bytes() {
            Some(n) => format!("RSA {}", n.len() * 8),
            None => "RSA".into(),
        },
        KeyData::Ecdsa(e) => match e.curve() {
            EcdsaCurve::NistP256 => "ECDSA P-256".into(),
            EcdsaCurve::NistP384 => "ECDSA P-384".into(),
            EcdsaCurve::NistP521 => "ECDSA P-521".into(),
        },
        KeyData::SkEd25519(_) => "Ed25519-SK".into(),
        KeyData::SkEcdsaSha2NistP256(_) => "ECDSA-SK".into(),
        other => other.algorithm().as_str().to_owned(),
    }
}

fn describe(
    name: String,
    private: Option<PathBuf>,
    public: Option<PathBuf>,
    key: &PublicKey,
    comment: &str,
    encrypted: Option<bool>,
) -> SshKey {
    SshKey {
        name,
        private,
        public,
        kind: kind_of(key.key_data()),
        fingerprint: key.fingerprint(HashAlg::Sha256).to_string(),
        comment: comment.to_owned(),
        encrypted,
        public_key: key.to_openssh().unwrap_or_default(),
    }
}

/// The keys in `dir`, by name.
pub fn keys(dir: &Path) -> Vec<SshKey> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .collect();
    files.sort();
    let mut out = Vec::new();
    let mut paired = Vec::new();
    for path in &files {
        if path.extension().is_some_and(|e| e == "pub") {
            continue;
        }
        // A private key is small; anything larger is not one.
        if fs::metadata(path).map_or(true, |m| m.len() > 64 * 1024) {
            continue;
        }
        let Ok(text) = fs::read(path) else { continue };
        let Ok(key) = PrivateKey::from_openssh(&text) else {
            continue;
        };
        let public_path = path.with_file_name(format!(
            "{}.pub",
            path.file_name().unwrap().to_string_lossy()
        ));
        let public = fs::read_to_string(&public_path)
            .ok()
            .and_then(|t| PublicKey::from_openssh(t.trim()).ok());
        // An encrypted key keeps its comment inside the encryption; the .pub has it in the clear.
        let comment = if key.is_encrypted() {
            public
                .as_ref()
                .map(|p| p.comment().to_owned())
                .unwrap_or_default()
        } else {
            key.comment().to_owned()
        };
        let has_public = public.is_some();
        if has_public {
            paired.push(public_path.clone());
        }
        out.push(describe(
            path.file_name().unwrap().to_string_lossy().into_owned(),
            Some(path.clone()),
            has_public.then_some(public_path),
            key.public_key(),
            &comment,
            Some(key.is_encrypted()),
        ));
    }
    for path in files
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "pub"))
    {
        if paired.contains(path) {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let Ok(public) = PublicKey::from_openssh(text.trim()) else {
            continue;
        };
        let private = path.with_extension("");
        let private = private.is_file().then_some(private);
        out.push(describe(
            path.file_stem().unwrap().to_string_lossy().into_owned(),
            private,
            Some(path.clone()),
            &public,
            public.comment(),
            None,
        ));
    }
    out.sort_by_key(|k| k.name.to_lowercase());
    out
}

/// What New SSH key makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewKind {
    Ed25519,
    Rsa3072,
}

#[derive(Debug)]
pub enum GenerateError {
    /// A file of that name is there already.
    Exists,
    /// A name that is not a plain file name.
    BadName,
    Io(io::Error),
    Key(ssh_key::Error),
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exists => write!(f, "a key with that name already exists"),
            Self::BadName => write!(f, "that name can't be used"),
            Self::Io(e) => write!(f, "{e}"),
            Self::Key(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for GenerateError {}

fn create(path: &Path, mode: u32, bytes: &[u8]) -> io::Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

/// Generates a key into `dir` as `name` and `name.pub`, encrypted with `passphrase` unless it
/// is empty. Slow for RSA; run it off the window's thread.
pub fn generate(
    dir: &Path,
    name: &str,
    kind: NewKind,
    comment: &str,
    passphrase: &str,
) -> Result<SshKey, GenerateError> {
    if name.is_empty() || name.contains('/') || name.starts_with('.') || name.ends_with(".pub") {
        return Err(GenerateError::BadName);
    }
    let private_path = dir.join(name);
    let public_path = dir.join(format!("{name}.pub"));
    if private_path.exists() || public_path.exists() {
        return Err(GenerateError::Exists);
    }
    let data = match kind {
        NewKind::Ed25519 => KeypairData::from(ssh_key::private::Ed25519Keypair::random(&mut OsRng)),
        NewKind::Rsa3072 => {
            KeypairData::from(RsaKeypair::random(&mut OsRng, 3072).map_err(GenerateError::Key)?)
        }
    };
    let mut key = PrivateKey::new(data, comment).map_err(GenerateError::Key)?;
    if !passphrase.is_empty() {
        key = key
            .encrypt(&mut OsRng, passphrase)
            .map_err(GenerateError::Key)?;
    }
    let text = key.to_openssh(LineEnding::LF).map_err(GenerateError::Key)?;
    let mut public = key.public_key().clone();
    public.set_comment(comment);
    let public_line = public.to_openssh().map_err(GenerateError::Key)?;
    if !dir.is_dir() {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(GenerateError::Io)?;
    }
    create(&private_path, 0o600, text.as_bytes()).map_err(|e| match e.kind() {
        io::ErrorKind::AlreadyExists => GenerateError::Exists,
        _ => GenerateError::Io(e),
    })?;
    if let Err(e) = create(&public_path, 0o644, format!("{public_line}\n").as_bytes()) {
        let _ = fs::remove_file(&private_path);
        return Err(GenerateError::Io(e));
    }
    let _ = fs::set_permissions(&private_path, fs::Permissions::from_mode(0o600));
    Ok(describe(
        name.to_owned(),
        Some(private_path),
        Some(public_path),
        &public,
        comment,
        Some(!passphrase.is_empty()),
    ))
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecryptError {
    WrongPassphrase,
    /// The private half is not an OpenSSH key Keychain can read.
    Unreadable,
}

/// The key's private half, decrypted with `passphrase` when it has one.
pub fn decrypt(key: &SshKey, passphrase: &str) -> Result<PrivateKey, DecryptError> {
    let path = key.private.as_ref().ok_or(DecryptError::Unreadable)?;
    let text = zeroize::Zeroizing::new(fs::read(path).map_err(|_| DecryptError::Unreadable)?);
    let private = PrivateKey::from_openssh(&*text).map_err(|_| DecryptError::Unreadable)?;
    if !private.is_encrypted() {
        return Ok(private);
    }
    let mut private = private
        .decrypt(passphrase)
        .map_err(|_| DecryptError::WrongPassphrase)?;
    if private.comment().is_empty() {
        private.set_comment(key.comment.clone());
    }
    Ok(private)
}
