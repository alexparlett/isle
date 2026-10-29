//! Every process from `/proc`: what any user can read for all of them, and disk counts for
//! the user's own (`p19a-monitor.md` choices 4 and 5).

use std::{collections::HashMap, fs, path::PathBuf, time::Instant};

use serde::Serialize;

/// A process by pid and start time, so a pid the kernel gives out again is a new process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct Key {
    pub pid: i32,
    /// Clock ticks after boot.
    pub start: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Process {
    pub key: Option<Key>,
    pub ppid: i32,
    /// The executable's base name, or `comm` in brackets for a kernel thread.
    pub name: String,
    pub cmdline: Vec<String>,
    pub uid: u32,
    /// `stat`'s state letter: R, S, D, T, Z and the rest.
    pub state: char,
    pub threads: u32,
    pub kernel_thread: bool,
    /// Cores' worth of time since the last sample: 1.5 is 150%.
    pub cpu: f32,
    /// User and system time over its life, in seconds.
    pub cpu_time: f64,
    /// Its footprint: anonymous, shared and swapped memory, in bytes.
    pub memory: u64,
    /// Bytes read from and written to storage over its life; none for another user's.
    pub disk: Option<(u64, u64)>,
    /// Bytes a second since the last sample.
    pub disk_rate: Option<(u64, u64)>,
    /// Context switches a second since the last sample.
    pub switches: f32,
    /// Its cgroup v2 path.
    pub cgroup: String,
}

impl Process {
    pub fn pid(&self) -> i32 {
        self.key.map_or(0, |k| k.pid)
    }
}

struct Last {
    ticks: u64,
    disk: Option<(u64, u64)>,
    switches: u64,
}

/// Reads every process, keeping each one's counters for the next sample's rates.
pub struct Procs {
    root: PathBuf,
    ticks: f64,
    last: HashMap<Key, Last>,
    at: Option<Instant>,
}

impl Procs {
    pub fn new(root: &std::path::Path) -> Self {
        Self {
            root: root.to_owned(),
            ticks: clock_ticks(),
            last: HashMap::new(),
            at: None,
        }
    }

    /// Every process now; rates are zero for a process the last sample did not see.
    pub fn sample(&mut self) -> Vec<Process> {
        let now = Instant::now();
        let secs = self.at.map(|at| now.duration_since(at).as_secs_f64()).filter(|s| *s > 0.0);
        let mut seen = HashMap::new();
        let mut out = Vec::new();
        let Ok(dir) = fs::read_dir(self.root.join("proc")) else {
            return out;
        };
        for entry in dir.flatten() {
            let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse::<i32>().ok()) else {
                continue;
            };
            let Some((mut process, ticks, switches)) = read(&entry.path(), pid, self.ticks) else {
                continue;
            };
            let key = process.key.unwrap();
            if let (Some(secs), Some(last)) = (secs, self.last.get(&key)) {
                process.cpu = (ticks.saturating_sub(last.ticks) as f64 / self.ticks / secs) as f32;
                process.switches = (switches.saturating_sub(last.switches) as f64 / secs) as f32;
                process.disk_rate = process.disk.zip(last.disk).map(|((r, w), (r0, w0))| {
                    (
                        (r.saturating_sub(r0) as f64 / secs) as u64,
                        (w.saturating_sub(w0) as f64 / secs) as u64,
                    )
                });
            }
            seen.insert(
                key,
                Last {
                    ticks,
                    disk: process.disk,
                    switches,
                },
            );
            out.push(process);
        }
        self.last = seen;
        self.at = Some(now);
        out.sort_by_key(|p| p.pid());
        out
    }
}

/// One process, with its CPU ticks and context switches for the rates; none once it is gone.
fn read(dir: &std::path::Path, pid: i32, hz: f64) -> Option<(Process, u64, u64)> {
    let stat = fs::read_to_string(dir.join("stat")).ok()?;
    // `comm` may hold spaces and brackets, so the fields start after its last bracket.
    let (head, rest) = stat.rsplit_once(')')?;
    let comm = head.split_once('(')?.1.to_owned();
    let f: Vec<&str> = rest.split_whitespace().collect();
    let num = |i: usize| f.get(i).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let state = f.first()?.chars().next().unwrap_or('?');
    let kernel_thread = num(6) & 0x0020_0000 != 0;
    let ticks = num(11) + num(12);
    let status = fs::read_to_string(dir.join("status")).unwrap_or_default();
    let field = |name: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(name))
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse::<u64>().ok())
    };
    let kb = |name: &str| field(name).unwrap_or(0) * 1024;
    let cmdline: Vec<String> = fs::read(dir.join("cmdline"))
        .unwrap_or_default()
        .split(|b| *b == 0)
        .filter(|a| !a.is_empty())
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .collect();
    let disk = fs::read_to_string(dir.join("io")).ok().map(|io| {
        let get = |name: &str| {
            io.lines()
                .find_map(|l| l.strip_prefix(name))
                .and_then(|v| v.trim().parse::<u64>().ok())
                .unwrap_or(0)
        };
        (get("read_bytes:"), get("write_bytes:"))
    });
    let cgroup = fs::read_to_string(dir.join("cgroup"))
        .unwrap_or_default()
        .lines()
        .find_map(|l| l.strip_prefix("0::"))
        .unwrap_or_default()
        .to_owned();
    let process = Process {
        key: Some(Key { pid, start: num(19) }),
        ppid: num(1) as i32,
        name: name(&comm, &cmdline, kernel_thread),
        uid: field("Uid:").unwrap_or(0) as u32,
        state,
        threads: num(17) as u32,
        kernel_thread,
        cpu: 0.0,
        cpu_time: ticks as f64 / hz,
        memory: kb("RssAnon:") + kb("RssShmem:") + kb("VmSwap:"),
        disk,
        disk_rate: None,
        switches: 0.0,
        cgroup,
        cmdline,
    };
    let switches = field("voluntary_ctxt_switches:").unwrap_or(0) + field("nonvoluntary_ctxt_switches:").unwrap_or(0);
    Some((process, ticks, switches))
}

/// `comm`, which the kernel cuts at 15 bytes: a cut one is completed from the first
/// argument's base name when that starts the same way. In brackets for a kernel thread.
fn name(comm: &str, cmdline: &[String], kernel_thread: bool) -> String {
    if kernel_thread {
        return format!("[{comm}]");
    }
    let base = cmdline
        .first()
        .and_then(|a| a.split(' ').next())
        .map(|a| a.rsplit('/').next().unwrap_or(a));
    match base {
        Some(base) if comm.len() == 15 && base.starts_with(comm) => base.to_owned(),
        _ => comm.to_owned(),
    }
}

fn clock_ticks() -> f64 {
    // SAFETY: sysconf has no preconditions.
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if hz > 0 { hz as f64 } else { 100.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use isle_fakes::tree::{Proc, Tree};

    #[test]
    fn rates_come_from_two_samples_and_a_reused_pid_starts_afresh() {
        let tree = Tree::new("procs-rates");
        let mut p = Proc {
            pid: 42,
            comm: "cargo".into(),
            cmdline: vec!["/home/a/.cargo/bin/cargo".into(), "build".into()],
            utime: 100,
            stime: 50,
            rss_anon: 2048,
            rss_shmem: 1024,
            swap: 512,
            io: Some((1000, 2000)),
            voluntary: 10,
            ..Default::default()
        };
        tree.process(&p);
        let mut procs = Procs::new(tree.path());
        let first = procs.sample();
        assert_eq!(first.len(), 1);
        let one = &first[0];
        assert_eq!(one.name, "cargo");
        assert_eq!(one.memory, (2048 + 1024 + 512) * 1024);
        assert_eq!(one.cpu, 0.0, "the first sample has nothing to compare with");
        assert_eq!(one.disk, Some((1000, 2000)));

        std::thread::sleep(std::time::Duration::from_millis(100));
        p.utime += 10;
        p.io = Some((1000 + 100_000, 2000));
        p.voluntary += 50;
        tree.process(&p);
        let second = &procs.sample()[0];
        assert!(second.cpu > 0.0, "{}", second.cpu);
        assert!(second.disk_rate.unwrap().0 > 0);
        assert!(second.switches > 0.0);

        p.start += 1;
        p.utime += 1000;
        tree.process(&p);
        let reused = &procs.sample()[0];
        assert_eq!(reused.cpu, 0.0, "a new start time is a new process");
    }

    #[test]
    fn kernel_threads_another_users_process_and_odd_names() {
        let tree = Tree::new("procs-kinds");
        tree.process(&Proc {
            pid: 2,
            ppid: 0,
            comm: "kthreadd".into(),
            cmdline: vec![],
            kernel_thread: true,
            uid: 0,
            io: None,
            ..Default::default()
        });
        tree.process(&Proc {
            pid: 300,
            comm: "sshd".into(),
            cmdline: vec!["sshd: alex [priv]".into()],
            uid: 0,
            io: None,
            ..Default::default()
        });
        tree.process(&Proc {
            pid: 301,
            comm: "Web Content".into(),
            cmdline: vec!["/usr/lib/firefox/firefox".into(), "-contentproc".into()],
            ..Default::default()
        });
        tree.process(&Proc {
            pid: 302,
            comm: "gnome-keyring-d".into(),
            cmdline: vec!["/usr/bin/gnome-keyring-daemon".into(), "--start".into()],
            ..Default::default()
        });
        tree.write("proc/self/stat", "not a pid");
        let all = Procs::new(tree.path()).sample();
        let names: Vec<&str> = all.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["[kthreadd]", "sshd", "Web Content", "gnome-keyring-daemon"]);
        assert!(all[0].kernel_thread);
        assert_eq!(all[1].disk, None, "another user's process has no io to read");
        assert_eq!(all[1].uid, 0);
    }
}
