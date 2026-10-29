//! One process read whole for Inspect (`p19a-monitor.md` choice 22): what any user may read
//! of any process, and for the user's own its open files and its sockets, joined to the
//! kernel's socket dumps by inode.

use std::{collections::HashMap, fs, path::Path};

use serde::Serialize;

use crate::{net, procs::Key};

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Detail {
    pub ppid: i32,
    pub parent: String,
    pub uid: u32,
    /// Seconds since the epoch.
    pub started: Option<i64>,
    pub state: String,
    pub nice: i32,
    pub threads: u32,
    pub cmdline: Vec<String>,
    /// Readable for the user's own processes.
    pub exe: Option<String>,
    pub cwd: Option<String>,
    /// The systemd unit its cgroup names.
    pub unit: String,
    /// Proportional set size, bytes; the user's own only.
    pub pss: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OpenFile {
    pub fd: u32,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Port {
    pub fd: u32,
    /// "TCP", "UDP" or "Unix".
    pub protocol: &'static str,
    pub local: String,
    /// The far end, or for a Unix socket the process at its other end where that is known.
    pub remote: String,
}

fn state_word(c: char) -> &'static str {
    match c {
        'R' => "Running",
        'S' => "Sleeping",
        'D' => "Waiting for disk",
        'T' | 't' => "Stopped",
        'Z' => "Zombie",
        'I' => "Idle",
        _ => "Unknown",
    }
}

fn ticks() -> f64 {
    // SAFETY: sysconf has no preconditions.
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if hz > 0 { hz as f64 } else { 100.0 }
}

/// The process `key` names, if it still runs under that start time.
pub fn detail(key: Key) -> Option<Detail> {
    let dir = Path::new("/proc").join(key.pid.to_string());
    let stat = fs::read_to_string(dir.join("stat")).ok()?;
    let (_, rest) = stat.rsplit_once(')')?;
    let f: Vec<&str> = rest.split_whitespace().collect();
    let num = |i: usize| f.get(i).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    if num(19) as u64 != key.start {
        return None;
    }
    let ppid = num(1) as i32;
    let status = fs::read_to_string(dir.join("status")).unwrap_or_default();
    let uid = status.lines().find_map(|l| l.strip_prefix("Uid:")).and_then(|v| v.split_whitespace().next()).and_then(|v| v.parse().ok()).unwrap_or(0);
    let btime = fs::read_to_string("/proc/stat").ok().and_then(|s| s.lines().find_map(|l| l.strip_prefix("btime ")).and_then(|v| v.trim().parse::<i64>().ok()));
    let parent = fs::read_to_string(format!("/proc/{ppid}/comm")).map(|c| c.trim().to_owned()).unwrap_or_default();
    let link = |name: &str| fs::read_link(dir.join(name)).ok().map(|p| p.to_string_lossy().into_owned());
    let pss = fs::read_to_string(dir.join("smaps_rollup"))
        .ok()
        .and_then(|s| s.lines().find_map(|l| l.strip_prefix("Pss:")).and_then(|v| v.split_whitespace().next()).and_then(|v| v.parse::<u64>().ok()))
        .map(|kb| kb * 1024);
    let cgroup = fs::read_to_string(dir.join("cgroup")).unwrap_or_default();
    let unit = cgroup.lines().find_map(|l| l.strip_prefix("0::")).and_then(|p| p.rsplit('/').next()).unwrap_or_default().to_owned();
    Some(Detail {
        ppid,
        parent,
        uid,
        started: btime.map(|b| b + (num(19) as f64 / ticks()) as i64),
        state: state_word(f.first().and_then(|s| s.chars().next()).unwrap_or('?')).to_owned(),
        nice: num(16) as i32,
        threads: num(17) as u32,
        cmdline: fs::read(dir.join("cmdline"))
            .unwrap_or_default()
            .split(|b| *b == 0)
            .filter(|a| !a.is_empty())
            .map(|a| String::from_utf8_lossy(a).into_owned())
            .collect(),
        exe: link("exe"),
        cwd: link("cwd"),
        unit,
        pss,
    })
}

/// The process's open files and sockets; none for another user's, whose descriptors need
/// ptrace access. `owners` names a Unix peer's process by its socket inode.
pub fn open_files(pid: i32, owners: &HashMap<u32, (i32, String)>) -> Option<(Vec<OpenFile>, Vec<Port>)> {
    let dir = fs::read_dir(format!("/proc/{pid}/fd")).ok()?;
    let mut files = Vec::new();
    let mut sockets: Vec<(u32, u32)> = Vec::new();
    for entry in dir.flatten() {
        let Some(fd) = entry.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else { continue };
        let Ok(target) = fs::read_link(entry.path()) else { continue };
        let target = target.to_string_lossy().into_owned();
        match target.strip_prefix("socket:[").and_then(|t| t.strip_suffix(']')).and_then(|t| t.parse().ok()) {
            Some(inode) => sockets.push((fd, inode)),
            None => files.push(OpenFile { fd, path: target }),
        }
    }
    files.sort_by_key(|f| f.fd);
    let inet: Vec<net::Socket> = net::tcp_sockets().unwrap_or_default().into_iter().chain(net::udp_sockets().unwrap_or_default()).collect();
    let unix = net::unix_sockets().unwrap_or_default();
    let mut ports = Vec::new();
    for (fd, inode) in sockets {
        if let Some(s) = inet.iter().find(|s| s.inode == inode) {
            let protocol = if s.protocol == libc::IPPROTO_UDP as u8 { "UDP" } else { "TCP" };
            let remote = if s.remote.port() == 0 { String::new() } else { s.remote.to_string() };
            ports.push(Port { fd, protocol, local: s.local.to_string(), remote });
        } else if let Some(u) = unix.iter().find(|u| u.inode == inode) {
            let remote = u
                .peer
                .and_then(|p| owners.get(&p))
                .map(|(pid, name)| format!("{name} ({pid})"))
                .unwrap_or_default();
            ports.push(Port { fd, protocol: "Unix", local: u.name.clone().unwrap_or_default(), remote });
        }
    }
    ports.sort_by_key(|p| p.fd);
    Some((files, ports))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn this_process_reads_whole_with_its_file_its_tcp_and_its_unix_peer() {
        let pid = std::process::id() as i32;
        let stat = fs::read_to_string("/proc/self/stat").unwrap();
        let start = stat.rsplit_once(')').unwrap().1.split_whitespace().nth(19).unwrap().parse().unwrap();
        let d = detail(Key { pid, start }).unwrap();
        assert!(d.exe.is_some() && d.cwd.is_some() && d.pss.is_some(), "{d:?}");
        assert!(d.started.is_some());
        assert!(detail(Key { pid, start: start + 1 }).is_none(), "another start time is another process");

        let path = std::env::temp_dir().join(format!("isle-inspect-{pid}"));
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"x").unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let (a, b) = std::os::unix::net::UnixStream::pair().unwrap();
        let mut owners = HashMap::new();
        for (fd, s) in [(&a, "a"), (&b, "b")] {
            use std::os::fd::AsRawFd;
            let link = fs::read_link(format!("/proc/self/fd/{}", fd.as_raw_fd())).unwrap();
            let inode: u32 = link.to_string_lossy().trim_start_matches("socket:[").trim_end_matches(']').parse().unwrap();
            owners.insert(inode, (pid, s.to_owned()));
        }
        let (files, ports) = open_files(pid, &owners).unwrap();
        assert!(files.iter().any(|f| f.path == path.to_string_lossy()));
        let port = listener.local_addr().unwrap().to_string();
        assert!(ports.iter().any(|p| p.protocol == "TCP" && p.local == port), "{ports:?}");
        assert!(ports.iter().any(|p| p.protocol == "Unix" && p.remote.ends_with(&format!("({pid})"))), "{ports:?}");
        let _ = fs::remove_file(path);
    }
}
