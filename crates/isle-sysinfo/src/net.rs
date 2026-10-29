//! Network use per process from TCP's own counters, which `NETLINK_SOCK_DIAG` gives any user
//! for every socket; UDP has none (`p19a-monitor.md` choices 8 and 9). The footer's totals are
//! `/proc/net/dev`'s, every protocol in them.

use std::{
    collections::HashMap,
    fs, io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    os::unix::fs::MetadataExt,
    path::PathBuf,
    time::Instant,
};

use serde::Serialize;

const SOCK_DIAG_BY_FAMILY: u16 = 20;
const INET_DIAG_INFO: u16 = 2;
const INET_DIAG_CGROUP_ID: u16 = 21;
const NLM_F_REQUEST: u16 = 0x1;
const NLM_F_DUMP: u16 = 0x300;
const NLMSG_DONE: u16 = 3;
const NLMSG_ERROR: u16 = 2;
// Offsets into `struct tcp_info` and `struct inet_diag_msg` (linux/tcp.h, linux/inet_diag.h).
const TCPI_BYTES_ACKED: usize = 120;
const TCPI_BYTES_RECEIVED: usize = 128;
const TCPI_SEGS_OUT: usize = 136;
const TCPI_SEGS_IN: usize = 140;
const TCPI_BYTES_SENT: usize = 200;
const DIAG_MSG_LEN: usize = 72;

/// One TCP socket as the kernel reports it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Socket {
    pub inode: u32,
    pub cookie: u64,
    pub uid: u32,
    /// The cgroup v2 id: its folder's inode in the cgroup filesystem.
    pub cgroup: u64,
    pub state: u8,
    pub local: SocketAddr,
    pub remote: SocketAddr,
    pub received: u64,
    pub sent: u64,
    pub segments_in: u64,
    pub segments_out: u64,
}

/// Every TCP socket, IPv4 and IPv6.
pub fn tcp_sockets() -> io::Result<Vec<Socket>> {
    let mut out = dump(libc::AF_INET as u8)?;
    out.extend(dump(libc::AF_INET6 as u8)?);
    Ok(out)
}

fn dump(family: u8) -> io::Result<Vec<Socket>> {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    // SAFETY: socket(2) has no preconditions; a descriptor it returns is this function's.
    let fd = unsafe { libc::socket(libc::AF_NETLINK, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, libc::NETLINK_SOCK_DIAG) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    let mut req = Vec::with_capacity(72);
    req.extend_from_slice(&72u32.to_ne_bytes());
    req.extend_from_slice(&SOCK_DIAG_BY_FAMILY.to_ne_bytes());
    req.extend_from_slice(&(NLM_F_REQUEST | NLM_F_DUMP).to_ne_bytes());
    req.extend_from_slice(&1u32.to_ne_bytes());
    req.extend_from_slice(&0u32.to_ne_bytes());
    // inet_diag_req_v2: family, protocol, the extensions asked for, padding, every state,
    // and an empty socket id.
    req.extend_from_slice(&[family, libc::IPPROTO_TCP as u8, 1 << (INET_DIAG_INFO - 1), 0]);
    req.extend_from_slice(&u32::MAX.to_ne_bytes());
    req.extend_from_slice(&[0u8; 48]);
    let sent = unsafe { libc::send(fd.as_raw_fd(), req.as_ptr().cast(), req.len(), 0) };
    if sent < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut out = Vec::new();
    let mut buf = vec![0u8; 65536];
    loop {
        let n = unsafe { libc::recv(fd.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len(), 0) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut data = &buf[..n as usize];
        while data.len() >= 16 {
            let len = u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
            let kind = u16::from_ne_bytes(data[4..6].try_into().unwrap());
            if len < 16 || len > data.len() {
                return Ok(out);
            }
            match kind {
                NLMSG_DONE => return Ok(out),
                NLMSG_ERROR => {
                    let errno = i32::from_ne_bytes(data[16..20].try_into().unwrap_or_default());
                    return Err(io::Error::from_raw_os_error(-errno));
                }
                _ => {
                    if let Some(socket) = parse(&data[16..len]) {
                        out.push(socket);
                    }
                }
            }
            data = &data[(len + 3) & !3..];
        }
    }
}

/// One `inet_diag_msg` and its attributes.
fn parse(msg: &[u8]) -> Option<Socket> {
    if msg.len() < DIAG_MSG_LEN {
        return None;
    }
    let u32_at = |b: &[u8], o: usize| b.get(o..o + 4).map(|s| u32::from_ne_bytes(s.try_into().unwrap()));
    let u64_at = |b: &[u8], o: usize| b.get(o..o + 8).map(|s| u64::from_ne_bytes(s.try_into().unwrap()));
    let family = msg[0];
    let addr = |offset: usize, port: u16| -> SocketAddr {
        let ip = if family == libc::AF_INET as u8 {
            IpAddr::V4(Ipv4Addr::new(msg[offset], msg[offset + 1], msg[offset + 2], msg[offset + 3]))
        } else {
            IpAddr::V6(Ipv6Addr::from(<[u8; 16]>::try_from(&msg[offset..offset + 16]).unwrap()))
        };
        SocketAddr::new(ip, port)
    };
    let sport = u16::from_be_bytes([msg[4], msg[5]]);
    let dport = u16::from_be_bytes([msg[6], msg[7]]);
    let mut socket = Socket {
        inode: u32_at(msg, 68)?,
        cookie: u64_at(msg, 44)?,
        uid: u32_at(msg, 64)?,
        cgroup: 0,
        state: msg[1],
        local: addr(8, sport),
        remote: addr(24, dport),
        received: 0,
        sent: 0,
        segments_in: 0,
        segments_out: 0,
    };
    let mut attrs = &msg[DIAG_MSG_LEN..];
    while attrs.len() >= 4 {
        let len = u16::from_ne_bytes([attrs[0], attrs[1]]) as usize;
        let kind = u16::from_ne_bytes([attrs[2], attrs[3]]);
        if len < 4 || len > attrs.len() {
            break;
        }
        let value = &attrs[4..len];
        match kind {
            INET_DIAG_INFO => {
                let acked = u64_at(value, TCPI_BYTES_ACKED).unwrap_or(0);
                socket.sent = u64_at(value, TCPI_BYTES_SENT).filter(|b| *b > 0).unwrap_or(acked);
                socket.received = u64_at(value, TCPI_BYTES_RECEIVED).unwrap_or(0);
                socket.segments_out = u32_at(value, TCPI_SEGS_OUT).unwrap_or(0) as u64;
                socket.segments_in = u32_at(value, TCPI_SEGS_IN).unwrap_or(0) as u64;
            }
            INET_DIAG_CGROUP_ID => socket.cgroup = u64_at(value, 0).unwrap_or(0),
            _ => {}
        }
        attrs = &attrs[((len + 3) & !3).min(attrs.len())..];
    }
    Some(socket)
}

/// Bytes and packets received, then sent, over every interface but loopback since boot.
pub fn interfaces(root: &std::path::Path) -> [u64; 4] {
    let mut out = [0u64; 4];
    for line in fs::read_to_string(root.join("proc/net/dev")).unwrap_or_default().lines().skip(2) {
        let Some((name, rest)) = line.split_once(':') else { continue };
        if name.trim() == "lo" {
            continue;
        }
        let n: Vec<u64> = rest.split_whitespace().filter_map(|v| v.parse().ok()).collect();
        for (slot, field) in [0, 1, 8, 9].into_iter().enumerate() {
            out[slot] += n.get(field).copied().unwrap_or(0);
        }
    }
    out
}

/// A process's TCP use over the lives of its sockets since sampling began.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct NetUse {
    pub pid: i32,
    pub received: u64,
    pub sent: u64,
    pub packets_in: u64,
    pub packets_out: u64,
}

/// Attributes sockets to processes and keeps each process's running totals.
pub struct Net {
    root: PathBuf,
    /// Each socket's counters at the last sample, by cookie.
    last: HashMap<u64, [u64; 4]>,
    totals: HashMap<i32, NetUse>,
    pub at: Option<Instant>,
}

impl Net {
    pub fn new(root: &std::path::Path) -> Self {
        Self {
            root: root.to_owned(),
            last: HashMap::new(),
            totals: HashMap::new(),
            at: None,
        }
    }

    /// Each process's totals. `own` are the user's processes, whose fds name their sockets;
    /// `cgroups` is every process's cgroup path, for another user's sockets.
    pub fn sample(&mut self, sockets: &[Socket], own: &[i32], cgroups: &[(i32, String)]) -> Vec<NetUse> {
        let owners = self.owners(own);
        let by_cgroup = self.sole_by_cgroup(cgroups);
        let mut seen = HashMap::new();
        for s in sockets {
            let now = [s.received, s.sent, s.segments_in, s.segments_out];
            let last = self.last.get(&s.cookie).copied().unwrap_or_default();
            seen.insert(s.cookie, now);
            let Some(pid) = owners.get(&s.inode).or_else(|| by_cgroup.get(&s.cgroup)).copied() else {
                continue;
            };
            let total = self.totals.entry(pid).or_insert_with(|| NetUse { pid, ..Default::default() });
            total.received += now[0].saturating_sub(last[0]);
            total.sent += now[1].saturating_sub(last[1]);
            total.packets_in += now[2].saturating_sub(last[2]);
            total.packets_out += now[3].saturating_sub(last[3]);
        }
        self.last = seen;
        self.at = Some(Instant::now());
        let alive: Vec<i32> = cgroups.iter().map(|(p, _)| *p).collect();
        self.totals.retain(|pid, _| alive.contains(pid));
        let mut out: Vec<NetUse> = self.totals.values().cloned().collect();
        out.sort_by_key(|u| u.pid);
        out
    }

    /// Socket inodes to the user's processes holding them.
    fn owners(&self, own: &[i32]) -> HashMap<u32, i32> {
        let mut out = HashMap::new();
        for pid in own {
            let Ok(dir) = fs::read_dir(self.root.join(format!("proc/{pid}/fd"))) else { continue };
            for entry in dir.flatten() {
                let Ok(target) = fs::read_link(entry.path()) else { continue };
                let target = target.to_string_lossy();
                if let Some(inode) = target.strip_prefix("socket:[").and_then(|t| t.strip_suffix(']')).and_then(|t| t.parse().ok()) {
                    out.entry(inode).or_insert(*pid);
                }
            }
        }
        out
    }

    /// Cgroup ids holding exactly one process, to that process.
    fn sole_by_cgroup(&self, cgroups: &[(i32, String)]) -> HashMap<u64, i32> {
        let mut members: HashMap<&str, Vec<i32>> = HashMap::new();
        for (pid, path) in cgroups {
            members.entry(path.as_str()).or_default().push(*pid);
        }
        members
            .into_iter()
            .filter(|(_, pids)| pids.len() == 1)
            .filter_map(|(path, pids)| {
                let dir = self.root.join("sys/fs/cgroup").join(path.trim_start_matches('/'));
                Some((fs::metadata(dir).ok()?.ino(), pids[0]))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn socket(inode: u32, cookie: u64, uid: u32, cgroup: u64, received: u64) -> Socket {
        let any = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1);
        Socket { inode, cookie, uid, cgroup, state: 1, local: any, remote: any, received, sent: 0, segments_in: 1, segments_out: 1 }
    }

    #[test]
    fn another_users_socket_counts_for_the_only_process_in_its_cgroup() {
        let tree = isle_fakes::tree::Tree::new("net-cgroup");
        tree.write("sys/fs/cgroup/system.slice/sole.service/cgroup.procs", "700\n")
            .write("sys/fs/cgroup/system.slice/shared.service/cgroup.procs", "701\n702\n");
        let ino = |p: &str| fs::metadata(tree.root.join(p)).unwrap().ino();
        let sole = ino("sys/fs/cgroup/system.slice/sole.service");
        let shared = ino("sys/fs/cgroup/system.slice/shared.service");
        let cgroups = [
            (700, "/system.slice/sole.service".to_owned()),
            (701, "/system.slice/shared.service".to_owned()),
            (702, "/system.slice/shared.service".to_owned()),
        ];
        let mut net = Net::new(tree.path());
        let uses = net.sample(&[socket(1, 1, 0, sole, 900), socket(2, 2, 0, shared, 50)], &[], &cgroups);
        assert_eq!(uses.len(), 1, "{uses:?}");
        assert_eq!((uses[0].pid, uses[0].received), (700, 900));
    }

    #[test]
    fn the_footer_counts_every_interface_but_loopback() {
        let tree = isle_fakes::tree::Tree::new("net-dev");
        tree.write(
            "proc/net/dev",
            "h\nh\n    lo: 999 9 0 0 0 0 0 0 999 9 0 0 0 0 0 0\n wlan0: 1000 10 0 0 0 0 0 0 500 5 0 0 0 0 0 0\n",
        );
        assert_eq!(interfaces(tree.path()), [1000, 10, 500, 5]);
    }

    /// The test's own loopback connection is found with its bytes, and given to this process.
    #[test]
    fn the_dump_finds_this_processs_socket_and_its_bytes() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut client = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        client.write_all(&[7u8; 5000]).unwrap();
        let mut got = vec![0u8; 5000];
        server.read_exact(&mut got).unwrap();

        let sockets = tcp_sockets().unwrap();
        let mine = sockets
            .iter()
            .find(|s| s.local.port() != port && s.remote.port() == port)
            .expect("the client's socket");
        assert_eq!(mine.sent, 5000);
        assert!(mine.segments_out > 0);
        assert!(mine.cgroup != 0);

        let pid = std::process::id() as i32;
        let mut net = Net::new(std::path::Path::new("/"));
        let uses = net.sample(&sockets, &[pid], &[(pid, String::new())]);
        let me = uses.iter().find(|u| u.pid == pid).expect("this process's use");
        assert!(me.sent >= 5000 && me.received >= 5000, "{me:?}");

        client.write_all(&[1u8; 1000]).unwrap();
        server.read_exact(&mut got[..1000]).unwrap();
        let again = net.sample(&tcp_sockets().unwrap(), &[pid], &[(pid, String::new())]);
        let me2 = again.iter().find(|u| u.pid == pid).unwrap();
        assert_eq!(me2.sent - me.sent, 1000, "only what moved since the last sample is added");
    }
}
