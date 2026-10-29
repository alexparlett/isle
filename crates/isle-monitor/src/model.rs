//! What the window shows from a sample: each process with its figures, the rows of the
//! resource and filter in force, the hierarchy with its totals, and the figures written.

use std::collections::{BTreeSet, HashMap};

use isle_sysinfo::{monitor::Full, power};
use isle_ui::table::{Cell, Column, Row, Sort};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resource {
    #[default]
    Cpu,
    Memory,
    Energy,
    Disk,
    Network,
    Gpu,
}

impl Resource {
    pub const ALL: [Resource; 6] = [Self::Cpu, Self::Memory, Self::Energy, Self::Disk, Self::Network, Self::Gpu];

    pub fn title(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Memory => "Memory",
            Self::Energy => "Energy",
            Self::Disk => "Disk",
            Self::Network => "Network",
            Self::Gpu => "GPU",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|r| *r == self).unwrap_or(0)
    }

    /// Its columns after the name, the first its default sort (`structure/monitor.md`).
    pub fn columns(self, gpus: usize) -> Vec<&'static str> {
        match self {
            Self::Cpu => vec!["cpu", "cpu_time", "threads", "pid", "user"],
            Self::Memory => vec!["memory", "threads", "pid", "user"],
            Self::Energy => vec!["energy", "cpu", "gpu", "sleep", "pid", "user"],
            Self::Disk => vec!["written", "read", "pid", "user"],
            Self::Network => vec!["received", "sent", "pid", "user"],
            Self::Gpu if gpus > 1 => vec!["gpu", "gpu_memory", "gpu_name", "pid", "user"],
            Self::Gpu => vec!["gpu", "gpu_memory", "pid", "user"],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Filter {
    #[default]
    All,
    Hierarchy,
    Mine,
    System,
    Windowed,
}

impl Filter {
    pub const ALL: [Filter; 5] = [Self::All, Self::Hierarchy, Self::Mine, Self::System, Self::Windowed];

    pub fn title(self) -> &'static str {
        match self {
            Self::All => "All Processes",
            Self::Hierarchy => "All Processes, Hierarchically",
            Self::Mine => "My Processes",
            Self::System => "System Processes",
            Self::Windowed => "Windowed Processes",
        }
    }
}

/// A column's title, width and kind (`surfaces/monitor.md` § The table).
pub fn column(id: &'static str) -> Column {
    match id {
        "name" => Column::text(id, "Process name", 240.),
        "cpu" => Column::figures(id, "% CPU", 64.),
        "cpu_time" => Column::figures(id, "CPU time", 80.),
        "threads" => Column::figures(id, "Threads", 64.),
        "pid" => Column::figures(id, "PID", 64.),
        "memory" => Column::figures(id, "Memory", 88.),
        "energy" => Column::figures(id, "Energy impact", 96.),
        "gpu" => Column::figures(id, "% GPU", 64.),
        "written" => Column::figures(id, "Bytes written", 104.),
        "read" => Column::figures(id, "Bytes read", 104.),
        "received" => Column::figures(id, "Bytes received", 112.),
        "sent" => Column::figures(id, "Bytes sent", 104.),
        "gpu_memory" => Column::figures(id, "GPU memory", 104.),
        "user" => Column::text(id, "User", 96.),
        "sleep" => Column::text(id, "Preventing sleep", 120.),
        "gpu_name" => Column::text(id, "GPU", 120.),
        other => Column::text(other, other, 96.),
    }
}

/// The figure columns, whose cells add up a subtree in the hierarchy.
pub fn summed(id: &str) -> bool {
    matches!(id, "cpu" | "cpu_time" | "threads" | "memory" | "energy" | "gpu" | "written" | "read" | "received" | "sent" | "gpu_memory")
}

/// One process with every figure Monitor shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Proc {
    pub key: u64,
    pub start: u64,
    pub pid: i32,
    pub ppid: i32,
    pub name: String,
    pub icon: Option<std::path::PathBuf>,
    pub uid: u32,
    pub user: String,
    pub stopped: bool,
    pub kernel: bool,
    pub windowed: bool,
    /// Cores.
    pub cpu: f32,
    pub cpu_time: f64,
    pub threads: u32,
    pub memory: u64,
    pub energy: f32,
    pub gpu: Option<f32>,
    pub gpu_memory: Option<u64>,
    pub gpu_name: Option<String>,
    /// "Sleep", "Idle", "Shut down".
    pub sleep: Vec<&'static str>,
    pub written: Option<u64>,
    pub read: Option<u64>,
    pub received: Option<u64>,
    pub sent: Option<u64>,
    pub cgroup: String,
}

/// A row key for a pid and start time.
pub fn key(pid: i32, start: u64) -> u64 {
    ((start & 0xffff_ffff) << 32) | pid as u32 as u64
}

/// What the window knows beside a sample: windowed processes' app names and icons by pid,
/// holds by pid, user names by uid.
#[derive(Clone, Debug, Default)]
pub struct Extras {
    pub windows: HashMap<i32, (String, Option<std::path::PathBuf>)>,
    pub holds: HashMap<i32, BTreeSet<&'static str>>,
    pub users: HashMap<u32, String>,
}

/// Every process in `full`, named and with its figures.
pub fn processes(full: &Full, extras: &Extras) -> Vec<Proc> {
    let gpu = |pid: i32| full.gpu_uses.iter().find(|u| u.pid == pid);
    let net = |pid: i32| full.net_uses.iter().find(|u| u.pid == pid);
    full.processes
        .iter()
        .map(|p| {
            let pid = p.pid();
            let start = p.key.map_or(0, |k| k.start);
            let window = extras.windows.get(&pid);
            let used = gpu(pid);
            let traffic = net(pid);
            let gpu_busy = used.and_then(|u| u.busy);
            Proc {
                key: key(pid, start),
                start,
                pid,
                ppid: p.ppid,
                name: window.map_or_else(|| p.name.clone(), |w| w.0.clone()),
                icon: window.and_then(|w| w.1.clone()),
                uid: p.uid,
                user: extras.users.get(&p.uid).cloned().unwrap_or_else(|| p.uid.to_string()),
                stopped: p.state == 'T' || p.state == 't',
                kernel: p.kernel_thread,
                windowed: window.is_some(),
                cpu: p.cpu,
                cpu_time: p.cpu_time,
                threads: p.threads,
                memory: p.memory,
                energy: power::energy_impact(p.cpu, gpu_busy.unwrap_or(0.0), p.switches),
                gpu: gpu_busy,
                gpu_memory: used.map(|u| u.memory),
                gpu_name: used.and_then(|u| full.gpus.get(u.gpu)).map(|g| g.name.clone()),
                sleep: extras.holds.get(&pid).map(|h| h.iter().copied().collect()).unwrap_or_default(),
                written: p.disk.map(|d| d.1),
                read: p.disk.map(|d| d.0),
                received: traffic.map(|t| t.received),
                sent: traffic.map(|t| t.sent),
                cgroup: p.cgroup.clone(),
            }
        })
        .collect()
}

/// A column's figure for sorting and adding up; none where it was not read.
pub fn figure(p: &Proc, id: &str) -> Option<f64> {
    Some(match id {
        "cpu" => p.cpu as f64,
        "cpu_time" => p.cpu_time,
        "threads" => p.threads as f64,
        "pid" => p.pid as f64,
        "memory" => p.memory as f64,
        "energy" => p.energy as f64,
        "gpu" => p.gpu? as f64,
        "gpu_memory" => p.gpu_memory? as f64,
        "written" => p.written? as f64,
        "read" => p.read? as f64,
        "received" => p.received? as f64,
        "sent" => p.sent? as f64,
        _ => return None,
    })
}

/// A figure written for its column.
pub fn write(id: &str, v: f64) -> String {
    match id {
        "cpu" | "gpu" => format!("{:.1}", v * 100.0),
        "energy" => format!("{v:.1}"),
        "cpu_time" => duration(v),
        "threads" | "pid" => format!("{v:.0}"),
        _ => bytes(v as u64),
    }
}

/// Bytes in decimal units, as the Mac writes them: "1.2 GB", "84 KB", "312 bytes".
pub fn bytes(b: u64) -> String {
    let v = b as f64;
    match [(1e12, "TB"), (1e9, "GB"), (1e6, "MB"), (1e3, "KB")].into_iter().find(|(d, _)| v >= *d) {
        Some((d, unit)) if v / d < 10.0 => format!("{:.1} {unit}", v / d),
        Some((d, unit)) => format!("{:.0} {unit}", v / d),
        None => format!("{b} bytes"),
    }
}

/// CPU time as hours, minutes and seconds: "1:02:11", "14:03", "0:12".
pub fn duration(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

fn text(p: &Proc, id: &str) -> String {
    match id {
        "user" => p.user.clone(),
        "sleep" => p.sleep.join(", "),
        "gpu_name" => p.gpu_name.clone().unwrap_or_default(),
        _ => figure(p, id).map(|v| write(id, v)).unwrap_or_default(),
    }
}

/// Whether `p` passes the filter and the search; `uid` is the user's.
pub fn shown(p: &Proc, filter: Filter, uid: u32, query: &str) -> bool {
    let passes = match filter {
        Filter::All | Filter::Hierarchy => true,
        Filter::Mine => p.uid == uid && !p.kernel,
        Filter::System => p.uid != uid || p.kernel,
        Filter::Windowed => p.windowed,
    };
    if !passes {
        return false;
    }
    let query = query.trim().to_lowercase();
    query.is_empty()
        || p.name.to_lowercase().contains(&query)
        || p.pid.to_string() == query
        || p.user.to_lowercase().contains(&query)
}

/// Orders by the sort's column, then by name, then by pid; a column without a figure sorts
/// by its text.
fn order(a: &Proc, b: &Proc, sort: Sort, totals: &HashMap<u64, HashMap<&'static str, f64>>) -> std::cmp::Ordering {
    let value = |p: &Proc| totals.get(&p.key).and_then(|t| t.get(sort.column).copied()).or_else(|| figure(p, sort.column));
    let by = match (value(a), value(b)) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) if sort.column == "name" => a.name.to_lowercase().cmp(&b.name.to_lowercase()).reverse(),
        (None, None) => text(a, sort.column).cmp(&text(b, sort.column)),
    };
    let by = if sort.descending { by.reverse() } else { by };
    by.then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())).then(a.pid.cmp(&b.pid))
}

/// The table's rows for the resource, filter, search and sort; in the hierarchy, each
/// process under its parent, `open` holding the expanded ones, sorted among its siblings by
/// its subtree's totals, which its cells carry beside its own figures.
pub fn rows(procs: &[Proc], columns: &[&'static str], filter: Filter, uid: u32, query: &str, sort: Sort, open: &BTreeSet<u64>) -> Vec<Row> {
    let kept: Vec<&Proc> = procs.iter().filter(|p| shown(p, filter, uid, query)).collect();
    let cells = |p: &Proc, totals: Option<&HashMap<&'static str, f64>>| -> Vec<Cell> {
        columns
            .iter()
            .map(|id| Cell {
                text: text(p, id),
                total: totals.and_then(|t| t.get(id)).map(|v| write(id, *v)),
            })
            .collect()
    };
    let row = |p: &Proc, depth: u16, disclosure: Option<bool>, totals: Option<&HashMap<&'static str, f64>>| Row {
        key: p.key,
        depth,
        disclosure,
        icon: p.icon.clone(),
        name: p.name.clone(),
        note: p.stopped.then(|| "Stopped".to_owned()),
        cells: cells(p, totals),
    };
    if filter != Filter::Hierarchy || !query.trim().is_empty() {
        let mut flat = kept;
        let none = HashMap::new();
        flat.sort_by(|a, b| order(a, b, sort, &none));
        return flat.into_iter().map(|p| row(p, 0, None, None)).collect();
    }
    let by_pid: HashMap<i32, &Proc> = kept.iter().map(|p| (p.pid, *p)).collect();
    let mut children: HashMap<i32, Vec<&Proc>> = HashMap::new();
    let mut roots = Vec::new();
    for p in &kept {
        match by_pid.get(&p.ppid).filter(|_| p.ppid != p.pid) {
            Some(_) => children.entry(p.ppid).or_default().push(*p),
            None => roots.push(*p),
        }
    }
    // Each process's subtree totals, its own figures included, for those with children.
    let mut totals: HashMap<u64, HashMap<&'static str, f64>> = HashMap::new();
    fn add(p: &Proc, columns: &[&'static str], children: &HashMap<i32, Vec<&Proc>>, totals: &mut HashMap<u64, HashMap<&'static str, f64>>) -> HashMap<&'static str, f64> {
        let mut sum: HashMap<&'static str, f64> = columns.iter().filter(|c| summed(c)).filter_map(|c| Some((*c, figure(p, c)?))).collect();
        for child in children.get(&p.pid).into_iter().flatten() {
            for (c, v) in add(child, columns, children, totals) {
                *sum.entry(c).or_default() += v;
            }
        }
        if children.contains_key(&p.pid) {
            totals.insert(p.key, sum.clone());
        }
        sum
    }
    for root in &roots {
        add(root, columns, &children, &mut totals);
    }
    let mut out = Vec::new();
    fn walk<'a>(
        level: Vec<&'a Proc>,
        depth: u16,
        sort: Sort,
        children: &HashMap<i32, Vec<&'a Proc>>,
        totals: &HashMap<u64, HashMap<&'static str, f64>>,
        open: &BTreeSet<u64>,
        out: &mut Vec<Row>,
        row: &dyn Fn(&Proc, u16, Option<bool>, Option<&HashMap<&'static str, f64>>) -> Row,
    ) {
        let mut level = level;
        level.sort_by(|a, b| order(a, b, sort, totals));
        for p in level {
            let kids = children.get(&p.pid);
            let is_open = open.contains(&p.key);
            out.push(row(p, depth, kids.map(|_| is_open), totals.get(&p.key)));
            if let (Some(kids), true) = (kids, is_open) {
                walk(kids.clone(), depth + 1, sort, children, totals, open, out, row);
            }
        }
    }
    walk(roots, 0, sort, &children, &totals, open, &mut out, &row);
    out
}

/// User names by uid from `/etc/passwd`.
pub fn users(root: &std::path::Path) -> HashMap<u32, String> {
    std::fs::read_to_string(root.join("etc/passwd"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split(':').collect();
            Some((f.get(2)?.parse().ok()?, f.first()?.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: i32, ppid: i32, name: &str, cpu: f32) -> Proc {
        Proc { key: pid as u64, pid, ppid, name: name.into(), cpu, uid: 1000, user: "alex".into(), ..Default::default() }
    }

    fn desc(column: &'static str) -> Sort {
        Sort { column, descending: true }
    }

    #[test]
    fn flat_rows_sort_by_the_column_then_name_and_filters_and_search_apply() {
        let mut root = p(1, 0, "systemd", 0.01);
        root.uid = 0;
        root.user = "root".into();
        let procs = vec![root, p(10, 1, "firefox", 0.5), p(11, 10, "cargo", 1.5), p(12, 1, "steam", 0.5)];
        let cols = Resource::Cpu.columns(1);
        let names = |rows: Vec<Row>| rows.into_iter().map(|r| r.name).collect::<Vec<_>>();
        let none = BTreeSet::new();
        assert_eq!(names(rows(&procs, &cols, Filter::All, 1000, "", desc("cpu"), &none)), ["cargo", "firefox", "steam", "systemd"]);
        assert_eq!(names(rows(&procs, &cols, Filter::Mine, 1000, "", desc("cpu"), &none)), ["cargo", "firefox", "steam"]);
        assert_eq!(names(rows(&procs, &cols, Filter::System, 1000, "", desc("cpu"), &none)), ["systemd"]);
        assert_eq!(names(rows(&procs, &cols, Filter::All, 1000, "fire", desc("cpu"), &none)), ["firefox"]);
        assert_eq!(names(rows(&procs, &cols, Filter::All, 1000, "12", desc("cpu"), &none)), ["steam"], "a pid matches whole");
        let first = &rows(&procs, &cols, Filter::All, 1000, "", desc("cpu"), &none)[0];
        assert_eq!(first.cells[0].text, "150.0");
    }

    #[test]
    fn the_hierarchy_nests_opens_and_sorts_siblings_by_their_totals() {
        let procs = vec![p(1, 0, "systemd", 0.0), p(10, 1, "firefox", 0.1), p(11, 10, "content", 0.9), p(12, 1, "steam", 0.5)];
        let cols = Resource::Cpu.columns(1);
        let closed = rows(&procs, &cols, Filter::Hierarchy, 1000, "", desc("cpu"), &BTreeSet::new());
        assert_eq!(closed.len(), 1, "collapsed below the top");
        assert_eq!(closed[0].disclosure, Some(false));
        assert_eq!(closed[0].cells[0].total.as_deref(), Some("150.0"));
        let open: BTreeSet<u64> = [1, 10].into();
        let tree = rows(&procs, &cols, Filter::Hierarchy, 1000, "", desc("cpu"), &open);
        let shape: Vec<(String, u16)> = tree.iter().map(|r| (r.name.clone(), r.depth)).collect();
        // firefox's subtree (1.0) outweighs steam (0.5), though firefox alone is less.
        assert_eq!(shape, [("systemd".into(), 0), ("firefox".into(), 1), ("content".into(), 2), ("steam".into(), 1)]);
        assert_eq!(tree[3].disclosure, None, "a leaf has no caret");
        assert_eq!(tree[3].cells[0].total, None);
    }

    #[test]
    fn figures_are_written_as_the_sheet_has_them() {
        assert_eq!(bytes(1_234_567_890), "1.2 GB");
        assert_eq!(bytes(84_000), "84 KB");
        assert_eq!(bytes(312), "312 bytes");
        assert_eq!(duration(3731.0), "1:02:11");
        assert_eq!(duration(843.0), "14:03");
        assert_eq!(write("cpu", 0.124), "12.4");
        assert_eq!(key(42, 7), (7 << 32) | 42);
    }
}
