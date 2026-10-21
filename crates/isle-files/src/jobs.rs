//! Files' running jobs: each on a thread of its own, its progress, questions and end handed
//! to the app's task, which every window draws from (`p1/structure/files.md` § File jobs).

use std::{
    cell::Cell,
    path::PathBuf,
    sync::{Mutex, mpsc},
    time::{Duration, Instant},
};

use freya::prelude::*;
use futures_channel::mpsc::{UnboundedSender, unbounded};
use futures_util::StreamExt;
use isle_app::{AppHandle, WindowKey};
use isle_fs::jobs::{self, Answer, Conflict, Control, Progress, Report, Undo, Work};

#[derive(Clone, Debug, PartialEq)]
pub enum JobState {
    Running,
    Waiting,
    Done,
    Failed(String),
}

/// A job's stop switch, shared with its thread; equal only to itself.
#[derive(Clone, Debug, Default)]
pub struct StopFlag(pub std::sync::Arc<std::sync::atomic::AtomicBool>);

impl PartialEq for StopFlag {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl StopFlag {
    pub fn store(&self, on: bool, order: std::sync::atomic::Ordering) {
        self.0.store(on, order)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct JobView {
    pub id: u64,
    /// "Copying 3 items to “Backup”".
    pub title: String,
    pub progress: Progress,
    pub state: JobState,
    pub window: WindowKey,
    pub started: Instant,
    pub stop: StopFlag,
}

/// A name already taken, waiting for the user.
#[derive(Clone, Debug)]
pub struct Question {
    pub job: u64,
    pub window: WindowKey,
    pub conflict: Conflict,
    pub reply: mpsc::Sender<Answer>,
}

impl PartialEq for Question {
    fn eq(&self, other: &Self) -> bool {
        self.job == other.job && self.conflict == other.conflict
    }
}

/// A job's end, for the window that started it.
#[derive(Clone, Debug, PartialEq)]
pub struct Finished {
    pub window: WindowKey,
    /// The verb Undo names: "Move to Trash", "Copy of 3 Items".
    pub undo_label: String,
    pub report: Report,
}

enum Message {
    Progress(u64, Progress),
    Ask(u64, Conflict, mpsc::Sender<Answer>),
    End(u64, Report),
}

#[derive(Clone, Copy, PartialEq)]
pub struct Jobs {
    pub list: State<Vec<JobView>>,
    pub questions: State<Vec<Question>>,
    pub finished: State<Vec<Finished>>,
    /// Counts ends, for views that read the Trash or a gathering again after each job.
    pub ended: State<u64>,
}

thread_local! {
    static NEXT: Cell<u64> = const { Cell::new(0) };
}

struct ThreadControl {
    id: u64,
    tx: Mutex<UnboundedSender<Message>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    last: Mutex<Instant>,
}

impl Control for ThreadControl {
    fn progress(&self, progress: Progress) {
        // Ten reports a second at most reach the windows.
        let mut last = self.last.lock().unwrap();
        if last.elapsed() < Duration::from_millis(100) && progress.items < progress.total_items {
            return;
        }
        *last = Instant::now();
        let _ = self.tx.lock().unwrap().unbounded_send(Message::Progress(self.id, progress));
    }

    fn ask(&self, conflict: Conflict) -> Answer {
        let (reply, answer) = mpsc::channel();
        let _ = self.tx.lock().unwrap().unbounded_send(Message::Ask(self.id, conflict, reply));
        // A question nobody answers, its window and app gone, stops the job.
        answer.recv().unwrap_or(Answer { choice: jobs::Choice::Stop, all: true })
    }

    fn stopped(&self) -> bool {
        self.stop.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Jobs {
    pub fn new() -> Self {
        Self {
            list: State::create_global(Vec::new()),
            questions: State::create_global(Vec::new()),
            finished: State::create_global(Vec::new()),
            ended: State::create_global(0),
        }
    }

    /// The switcher card's progress: the running jobs' total share done.
    fn total(list: &[JobView]) -> Option<f64> {
        let running: Vec<_> = list.iter().filter(|j| matches!(j.state, JobState::Running | JobState::Waiting)).collect();
        if running.is_empty() {
            return None;
        }
        let share = |p: &Progress| {
            if p.total_bytes > 0 {
                p.bytes as f64 / p.total_bytes as f64
            } else if p.total_items > 0 {
                p.items as f64 / p.total_items as f64
            } else {
                0.
            }
        };
        Some(running.iter().map(|j| share(&j.progress)).sum::<f64>() / running.len() as f64)
    }

    /// Starts a job for `window`; it runs whatever windows close, holding the app open.
    pub fn start(self, app: &AppHandle, window: WindowKey, work: Work, title: String, undo_label: String) {
        let id = NEXT.with(|n| {
            n.set(n.get() + 1);
            n.get()
        });
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut list = self.list;
        list.write().push(JobView {
            id,
            title,
            progress: Progress::default(),
            state: JobState::Running,
            window,
            started: Instant::now(),
            stop: StopFlag(stop.clone()),
        });
        let (tx, mut rx) = unbounded();
        let control = ThreadControl {
            id,
            tx: Mutex::new(tx),
            stop,
            last: Mutex::new(Instant::now() - Duration::from_secs(1)),
        };
        std::thread::Builder::new()
            .name("isle-files-job".into())
            .spawn(move || {
                let report = jobs::run(work, &control);
                let _ = control.tx.lock().unwrap().unbounded_send(Message::End(control.id, report));
            })
            .ok();
        let hold = app.hold();
        let app = app.clone();
        let jobs = self;
        app.clone().spawn(async move {
            let _hold = hold;
            let (mut list, mut questions, mut finished, mut ended) = (jobs.list, jobs.questions, jobs.finished, jobs.ended);
            while let Some(message) = rx.next().await {
                match message {
                    Message::Progress(id, progress) => {
                        if let Some(job) = list.write().iter_mut().find(|j| j.id == id) {
                            job.progress = progress;
                        }
                    }
                    Message::Ask(id, conflict, reply) => {
                        if let Some(job) = list.write().iter_mut().find(|j| j.id == id) {
                            job.state = JobState::Waiting;
                        }
                        questions.write().push(Question { job: id, window, conflict, reply });
                    }
                    Message::End(id, report) => {
                        let failed = !report.failed.is_empty();
                        if let Some(job) = list.write().iter_mut().find(|j| j.id == id) {
                            job.state = if failed {
                                JobState::Failed(report.failed[0].1.clone())
                            } else {
                                JobState::Done
                            };
                        }
                        let undoable = matches!(&report.undo, Some(Undo::Steps(s)) if !s.is_empty());
                        if failed || undoable || report.undo == Some(Undo::CantUndo) {
                            finished.write().push(Finished { window, undo_label: undo_label.clone(), report });
                        }
                        *ended.write() += 1;
                        app.progress(Self::total(&list.peek()));
                        // A finished row stays a moment, then goes.
                        async_sleep(Duration::from_millis(2000)).await;
                        list.write().retain(|j| j.id != id || matches!(j.state, JobState::Failed(_)));
                        app.progress(Self::total(&list.peek()));
                        return;
                    }
                }
                app.progress(Self::total(&list.peek()));
            }
        });
    }

    /// Answers a waiting question.
    pub fn answer(self, job: u64, answer: Answer) {
        let mut questions = self.questions;
        let found = questions.write().iter().position(|q| q.job == job);
        if let Some(i) = found {
            let question = questions.write().remove(i);
            let _ = question.reply.send(answer);
        }
        let mut list = self.list;
        if let Some(j) = list.write().iter_mut().find(|j| j.id == job) {
            j.state = JobState::Running;
        }
    }
}

/// A sleep on the event loop, from a timer thread.
pub async fn async_sleep(duration: Duration) {
    let (tx, rx) = futures_channel::oneshot::channel::<()>();
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        let _ = tx.send(());
    });
    let _ = rx.await;
}

/// The words a job's row and its notification use.
pub fn title(work: &Work) -> (String, String) {
    let n = |items: &[PathBuf]| crate::format::items(items.len());
    let folder = |p: &PathBuf| crate::places::folder_name(p);
    match work {
        Work::Copy { items, into } => (format!("Copying {} to “{}”", n(items), folder(into)), format!("Copy of {}", n(items))),
        Work::Move { items, into } => (format!("Moving {} to “{}”", n(items), folder(into)), "Move".into()),
        Work::Duplicate { items } => (format!("Duplicating {}", n(items)), "Duplicate".into()),
        Work::Trash { items } => (format!("Moving {} to the Trash", n(items)), "Move to Trash".into()),
        Work::PutBack { items } => (format!("Putting back {}", crate::format::items(items.len())), "Put Back".into()),
        Work::Delete { items } => (format!("Deleting {}", n(items)), "Delete".into()),
        Work::EmptyTrash { .. } => ("Emptying the Trash".into(), "Empty Trash".into()),
        Work::NewFolder { .. } => ("Making a folder".into(), "New Folder".into()),
        Work::NewFile { .. } => ("Making a file".into(), "New File".into()),
        Work::Rename { .. } => ("Renaming".into(), "Rename".into()),
        Work::RenameMany { items, .. } => (format!("Renaming {}", n(items)), "Rename".into()),
        Work::Compress { items } => (format!("Compressing {}", n(items)), "Compress".into()),
        Work::Expand { .. } => ("Expanding".into(), "Expand".into()),
        Work::Undo { .. } => ("Undoing".into(), String::new()),
    }
}
