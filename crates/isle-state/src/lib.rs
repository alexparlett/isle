//! Machine-written state: one versioned JSON file per domain under `~/.local/state/isle/`.

pub mod domains;

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// One kind of state with a file of its own, so a damaged file costs only its domain.
pub trait Domain: Serialize + DeserializeOwned + Default {
    const NAME: &'static str;
    const VERSION: u32;
}

#[derive(Serialize, Deserialize)]
struct File<T> {
    version: u32,
    data: T,
}

/// How a load went; anything but `Read` and `Missing` is worth a log line.
#[derive(Debug, PartialEq, Eq)]
pub enum Loaded {
    Read,
    Missing,
    /// Unreadable; moved aside to the path given.
    Damaged(PathBuf),
    /// Written by a newer Isle; moved aside to the path given.
    Newer {
        version: u32,
        moved_to: PathBuf,
    },
}

#[derive(Clone, Debug)]
pub struct StateDir {
    dir: PathBuf,
}

impl StateDir {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// `$XDG_STATE_HOME/isle`, else `~/.local/state/isle`.
    pub fn default_dir() -> PathBuf {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("isle")
    }

    pub fn path<D: Domain>(&self) -> PathBuf {
        self.dir.join(format!("{}.json", D::NAME))
    }

    /// The domain's state; empty when missing, damaged or from a newer Isle.
    pub fn load<D: Domain>(&self) -> (D, Loaded) {
        let path = self.path::<D>();
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => return (D::default(), Loaded::Missing),
        };
        #[derive(Deserialize)]
        struct Header {
            version: u32,
        }
        match serde_json::from_slice::<Header>(&bytes) {
            Ok(header) if header.version > D::VERSION => {
                let moved_to = move_aside(&path);
                return (
                    D::default(),
                    Loaded::Newer {
                        version: header.version,
                        moved_to,
                    },
                );
            }
            Ok(_) => {}
            Err(_) => return (D::default(), Loaded::Damaged(move_aside(&path))),
        }
        match serde_json::from_slice::<File<D>>(&bytes) {
            Ok(file) => (file.data, Loaded::Read),
            Err(_) => (D::default(), Loaded::Damaged(move_aside(&path))),
        }
    }

    /// Writes by temporary file, fsync and rename, so a crash leaves the old or the new.
    pub fn save<D: Domain>(&self, data: &D) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.path::<D>();
        let temp = self
            .dir
            .join(format!(".{}.json.{}.tmp", D::NAME, std::process::id()));
        let json = serde_json::to_vec_pretty(&File {
            version: D::VERSION,
            data,
        })
        .map_err(io::Error::other)?;
        {
            let mut file = fs::File::create(&temp)?;
            file.write_all(&json)?;
            file.sync_all()?;
        }
        fs::rename(&temp, &path)?;
        fs::File::open(&self.dir)?.sync_all()
    }
}

/// Renames `path` to `path.bad`, replacing an older one.
fn move_aside(path: &Path) -> PathBuf {
    let mut bad = path.as_os_str().to_owned();
    bad.push(".bad");
    let bad = PathBuf::from(bad);
    let _ = fs::rename(path, &bad);
    bad
}
