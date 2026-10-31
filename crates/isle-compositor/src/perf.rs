//! What the redraw pass did, for `isle perf`: counts since the start and the last frames'
//! phase times.

use std::{collections::HashMap, time::Duration};

/// How many frames the phase times cover.
const KEPT: usize = 1024;

/// The redraw pass's phases, in the order `FramePhases` notes them.
pub const PHASES: [&str; 6] = ["total", "steps", "shell_dispatch", "shell_draw", "glass", "render"];

#[derive(Default)]
pub struct Perf {
    /// Redraw passes run, and those that changed nothing on screen.
    pub redraws: u64,
    pub undamaged: u64,
    /// Element lists built, for frames, glass and captures; counted where only `&self` is had.
    pub element_builds: std::cell::Cell<u64>,
    /// Commits by window id; other surfaces' commits together.
    pub commits: HashMap<u64, u64>,
    pub other_commits: u64,
    /// Each phase's last frames in microseconds, a ring.
    phases: Vec<[u32; 6]>,
    next: usize,
}

impl Perf {
    pub fn frame(&mut self, total: Duration, steps: [Duration; 5]) {
        let us = |d: Duration| d.as_micros().min(u32::MAX as u128) as u32;
        let row = [us(total), us(steps[0]), us(steps[1]), us(steps[2]), us(steps[3]), us(steps[4])];
        if self.phases.len() < KEPT {
            self.phases.push(row);
        } else {
            self.phases[self.next] = row;
        }
        self.next = (self.next + 1) % KEPT;
    }

    /// Each phase's p50, p95 and longest, in microseconds, over the kept frames.
    pub fn percentiles(&self) -> Vec<(&'static str, [u32; 3])> {
        PHASES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let mut times: Vec<u32> = self.phases.iter().map(|r| r[i]).collect();
                times.sort_unstable();
                let at = |q: f64| times.get(((times.len() as f64 - 1.0) * q).round() as usize).copied().unwrap_or(0);
                (*name, [at(0.5), at(0.95), times.last().copied().unwrap_or(0)])
            })
            .collect()
    }
}

/// CPU time by thread name since each thread started, in milliseconds; threads of one name
/// summed.
pub fn threads() -> Vec<(String, u64)> {
    let tick_ms = 1000 / rustix::param::clock_ticks_per_second().max(1);
    let mut by_name: HashMap<String, u64> = HashMap::new();
    let Ok(tasks) = std::fs::read_dir("/proc/self/task") else {
        return Vec::new();
    };
    for task in tasks.flatten() {
        let Ok(stat) = std::fs::read_to_string(task.path().join("stat")) else { continue };
        let (Some(open), Some(close)) = (stat.find('('), stat.rfind(')')) else { continue };
        let name = stat[open + 1..close].to_owned();
        let fields: Vec<&str> = stat[close + 2..].split(' ').collect();
        // utime and stime, the 14th and 15th fields; `fields` starts at the 3rd.
        let ticks: u64 = fields.get(11).and_then(|f| f.parse().ok()).unwrap_or(0)
            + fields.get(12).and_then(|f| f.parse().ok()).unwrap_or(0);
        *by_name.entry(name).or_default() += ticks * tick_ms;
    }
    let mut threads: Vec<_> = by_name.into_iter().collect();
    threads.sort_by(|a, b| b.1.cmp(&a.1));
    threads
}

/// The process's resident and anonymous memory, in KiB.
pub fn memory() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |key: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(key))
            .and_then(|v| v.split_whitespace().next()?.parse().ok())
            .unwrap_or(0)
    };
    (field("VmRSS:"), field("RssAnon:"))
}

/// The compositor's descendants running without transparent huge pages, by name: its
/// helpers, never an app (B137).
pub fn without_huge_pages() -> Vec<String> {
    let mut procs: HashMap<i32, (i32, String, bool)> = HashMap::new();
    let Ok(dirs) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    for dir in dirs.flatten() {
        let Some(pid) = dir.file_name().to_str().and_then(|n| n.parse::<i32>().ok()) else { continue };
        let Ok(status) = std::fs::read_to_string(dir.path().join("status")) else { continue };
        let field = |key: &str| status.lines().find_map(|l| l.strip_prefix(key)).map(str::trim);
        let parent = field("PPid:").and_then(|p| p.parse().ok()).unwrap_or(0);
        let off = field("THP_enabled:") == Some("0");
        procs.insert(pid, (parent, field("Name:").unwrap_or("").to_owned(), off));
    }
    let me = std::process::id() as i32;
    let below = |mut pid: i32| {
        while pid > 1 {
            pid = procs.get(&pid).map_or(0, |p| p.0);
            if pid == me {
                return true;
            }
        }
        false
    };
    let mut names: Vec<String> = procs
        .iter()
        .filter(|(pid, (_, _, off))| *off && below(**pid))
        .map(|(_, (_, name, _))| name.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

impl crate::state::Isle {
    /// The loop's side of `isle perf`, as JSON: counts since the start and the phase times.
    pub fn perf_status(&self) -> serde_json::Value {
        let apps: HashMap<u64, String> = self.wm.records.iter().map(|r| (r.id, r.names().0)).collect();
        let mut commits: HashMap<String, u64> = HashMap::new();
        for (id, count) in &self.perf.commits {
            let app = apps.get(id).cloned().unwrap_or_else(|| "closed windows".into());
            *commits.entry(app).or_default() += count;
        }
        commits.insert("other surfaces".into(), self.perf.other_commits);
        let outputs: Vec<_> = self
            .space
            .outputs()
            .map(|o| {
                serde_json::json!({
                    "name": o.name(),
                    "frames": crate::redraw::OutputState::of(o).borrow().frames_presented,
                })
            })
            .collect();
        let (glass_runs, glass_skips) = self.shell.glass_counts();
        let phases: serde_json::Map<String, serde_json::Value> = self
            .perf
            .percentiles()
            .into_iter()
            .map(|(name, us)| (name.to_owned(), serde_json::json!(us)))
            .collect();
        serde_json::json!({
            "outputs": outputs,
            "redraws": self.perf.redraws,
            "undamaged": self.perf.undamaged,
            "element_builds": self.perf.element_builds.get(),
            "glass_runs": glass_runs,
            "glass_skips": glass_skips,
            "commits": commits,
            "phases_us": phases,
        })
    }
}
