//! Thumbnails for the views: asks queued newest first, an ask whose cell has scrolled away
//! dropped, the cache read off the window's thread, the rest made by `isle-thumbnail`
//! (`p15-files.md` choices 14 and 15).

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Write},
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Arc, Condvar, Mutex, OnceLock},
};

use bytes::Bytes;
use futures_channel::oneshot;
use isle_thumbnail::{Answer, Ask, Flavour, cache};
use tracing::warn;

/// At most this many thumbnails being made at once, as the helper has threads.
const OUT: usize = 4;

/// Types worth asking for: the helper or a system thumbnailer can draw them.
pub fn worth_asking(mime: &str) -> bool {
    mime.starts_with("image/")
        || mime.starts_with("video/")
        || mime.starts_with("audio/")
        || mime == "application/pdf"
        || mime.starts_with("application/vnd.oasis.opendocument")
        || mime.starts_with("application/vnd.openxmlformats")
        || mime == "application/msword"
}

struct Job {
    path: PathBuf,
    mime: String,
    flavour: Flavour,
    mtime: i64,
    size: u64,
    reply: oneshot::Sender<Option<Bytes>>,
}

#[derive(Default)]
struct Queue {
    /// Waiting asks, the newest last and taken first.
    waiting: Vec<Job>,
    /// Asks with the helper, by id.
    out: HashMap<u64, Job>,
    next: u64,
    helper: Option<(Child, ChildStdin)>,
}

struct Thumbs {
    queue: Mutex<Queue>,
    wake: Condvar,
    program: PathBuf,
    /// Held so a package installed while Files runs cannot hand it another build's helper.
    _pinned: Option<File>,
}

/// The helper's program: beside Files, or as the environment names it for tests.
pub fn helper() -> PathBuf {
    std::env::var_os("ISLE_THUMBNAIL_HELPER")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok().map(|e| e.with_file_name("isle-thumbnail")))
        .unwrap_or_else(|| PathBuf::from("isle-thumbnail"))
}

fn thumbs() -> &'static Arc<Thumbs> {
    static THUMBS: OnceLock<Arc<Thumbs>> = OnceLock::new();
    THUMBS.get_or_init(|| {
        let found = helper();
        let pinned = File::open(&found).ok();
        let program = match &pinned {
            Some(file) => PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd())),
            None => found,
        };
        let thumbs = Arc::new(Thumbs {
            queue: Mutex::default(),
            wake: Condvar::new(),
            program,
            _pinned: pinned,
        });
        let worker = thumbs.clone();
        std::thread::Builder::new()
            .name("isle-files-thumbs".into())
            .spawn(move || worker.run())
            .ok();
        thumbs
    })
}

fn read(path: &Path) -> Option<Bytes> {
    std::fs::read(path).ok().map(Bytes::from)
}

impl Thumbs {
    fn run(self: Arc<Self>) {
        loop {
            let job = {
                let mut q = self.queue.lock().unwrap();
                loop {
                    // An ask whose cell has gone has a closed reply.
                    q.waiting.retain(|j| !j.reply.is_canceled());
                    if q.out.len() < OUT
                        && let Some(job) = q.waiting.pop()
                    {
                        break job;
                    }
                    q = self.wake.wait(q).unwrap();
                }
            };
            let root = cache::root();
            if let Some(found) = cache::find(&root, &job.path, job.flavour, job.mtime, job.size) {
                let _ = job.reply.send(read(&found));
                continue;
            }
            if cache::failed(&root, &job.path, job.mtime) {
                let _ = job.reply.send(None);
                continue;
            }
            self.ask(job);
        }
    }

    /// Hands a job to the helper, starting it when it is not running.
    fn ask(self: &Arc<Self>, job: Job) {
        let mut q = self.queue.lock().unwrap();
        if q.helper.is_none() {
            match self.start() {
                Some(helper) => q.helper = Some(helper),
                None => {
                    let _ = job.reply.send(None);
                    return;
                }
            }
        }
        q.next += 1;
        let id = q.next;
        let ask = Ask {
            id,
            path: job.path.clone(),
            mime: job.mime.clone(),
            flavour: job.flavour,
            mtime: job.mtime,
            size: job.size,
        };
        let line = serde_json::to_string(&ask).unwrap_or_default() + "\n";
        let sent = q.helper.as_mut().is_some_and(|(_, stdin)| stdin.write_all(line.as_bytes()).is_ok());
        if sent {
            q.out.insert(id, job);
        } else {
            q.helper = None;
            let _ = job.reply.send(None);
        }
    }

    fn start(self: &Arc<Self>) -> Option<(Child, ChildStdin)> {
        let mut child = Command::new(&self.program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|err| warn!("isle-thumbnail did not start: {err}"))
            .ok()?;
        let stdin = child.stdin.take()?;
        let stdout = child.stdout.take()?;
        let reader = self.clone();
        std::thread::Builder::new()
            .name("isle-files-thumbs-read".into())
            .spawn(move || reader.answers(stdout))
            .ok()?;
        Some((child, stdin))
    }

    /// Takes the helper's answers until it ends; then its outstanding asks get none.
    fn answers(self: Arc<Self>, stdout: std::process::ChildStdout) {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let Ok(answer) = serde_json::from_str::<Answer>(&line) else { continue };
            let job = self.queue.lock().unwrap().out.remove(&answer.id);
            if let Some(job) = job {
                let _ = job.reply.send(answer.thumbnail.as_deref().and_then(read));
            }
            self.wake.notify_one();
        }
        warn!("isle-thumbnail ended; it starts again at the next ask");
        let mut q = self.queue.lock().unwrap();
        if let Some((mut child, _)) = q.helper.take() {
            let _ = child.wait();
        }
        for (_, job) in q.out.drain() {
            let _ = job.reply.send(None);
        }
        self.wake.notify_one();
    }
}

/// Asks for a thumbnail of an item; the answer, or `None` when there is none to be had.
pub async fn get(path: PathBuf, mime: String, flavour: Flavour, mtime: i64, size: u64) -> Option<Bytes> {
    let (reply, answer) = oneshot::channel();
    let thumbs = thumbs();
    thumbs.queue.lock().unwrap().waiting.push(Job {
        path,
        mime,
        flavour,
        mtime,
        size,
        reply,
    });
    thumbs.wake.notify_one();
    answer.await.ok().flatten()
}
