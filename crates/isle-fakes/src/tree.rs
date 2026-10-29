//! A `/proc` and `/sys` tree in a temporary folder, for readers that take a root: processes
//! with their files, and anything else written at a path. Removed when dropped.

use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Tree {
    pub root: PathBuf,
}

/// One process's files.
#[derive(Clone, Debug)]
pub struct Proc {
    pub pid: i32,
    pub ppid: i32,
    pub comm: String,
    pub cmdline: Vec<String>,
    pub uid: u32,
    pub state: char,
    /// Clock ticks.
    pub utime: u64,
    pub stime: u64,
    /// Clock ticks after boot.
    pub start: u64,
    pub threads: u32,
    pub kernel_thread: bool,
    /// Kilobytes.
    pub rss_anon: u64,
    pub rss_shmem: u64,
    pub swap: u64,
    pub voluntary: u64,
    pub nonvoluntary: u64,
    /// Read and written bytes; none leaves `io` out, as for another user's process.
    pub io: Option<(u64, u64)>,
    pub cgroup: String,
}

impl Default for Proc {
    fn default() -> Self {
        Self {
            pid: 100,
            ppid: 1,
            comm: "proc".into(),
            cmdline: vec!["/usr/bin/proc".into()],
            uid: 1000,
            state: 'S',
            utime: 0,
            stime: 0,
            start: 1000,
            threads: 1,
            kernel_thread: false,
            rss_anon: 0,
            rss_shmem: 0,
            swap: 0,
            voluntary: 0,
            nonvoluntary: 0,
            io: Some((0, 0)),
            cgroup: "/user.slice/user-1000.slice/user@1000.service/app.slice".into(),
        }
    }
}

impl Tree {
    pub fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("isle-tree-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Writes `contents` at `path` under the root, making its folders.
    pub fn write(&self, path: &str, contents: &str) -> &Self {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
        self
    }

    pub fn remove(&self, path: &str) -> &Self {
        let path = self.root.join(path);
        let _ = fs::remove_dir_all(&path);
        let _ = fs::remove_file(&path);
        self
    }

    /// Writes `proc/<pid>/{stat,status,cmdline,cgroup}` and `io` when it has one.
    pub fn process(&self, p: &Proc) -> &Self {
        let dir = format!("proc/{}", p.pid);
        self.remove(&dir);
        let flags: u64 = if p.kernel_thread { 0x0020_0000 } else { 0x0040_0000 };
        // Fields 3 to 52 of proc_pid_stat(5); those Isle reads carry the process's values.
        let mut fields: Vec<String> = vec!["0".into(); 50];
        fields[0] = p.state.to_string();
        fields[1] = p.ppid.to_string();
        fields[6] = flags.to_string();
        fields[11] = p.utime.to_string();
        fields[12] = p.stime.to_string();
        fields[17] = p.threads.to_string();
        fields[19] = p.start.to_string();
        self.write(&format!("{dir}/stat"), &format!("{} ({}) {}\n", p.pid, p.comm, fields.join(" ")));
        let mut status = format!(
            "Name:\t{}\nState:\t{}\nPPid:\t{}\nUid:\t{u}\t{u}\t{u}\t{u}\nThreads:\t{}\n",
            p.comm,
            p.state,
            p.ppid,
            p.threads,
            u = p.uid
        );
        if !p.kernel_thread {
            status += &format!(
                "VmRSS:\t{} kB\nRssAnon:\t{} kB\nRssFile:\t0 kB\nRssShmem:\t{} kB\nVmSwap:\t{} kB\n",
                p.rss_anon + p.rss_shmem,
                p.rss_anon,
                p.rss_shmem,
                p.swap
            );
        }
        status += &format!(
            "voluntary_ctxt_switches:\t{}\nnonvoluntary_ctxt_switches:\t{}\n",
            p.voluntary, p.nonvoluntary
        );
        self.write(&format!("{dir}/status"), &status);
        let cmdline: String = p.cmdline.iter().map(|a| format!("{a}\0")).collect();
        self.write(&format!("{dir}/cmdline"), &cmdline);
        self.write(&format!("{dir}/cgroup"), &format!("0::{}\n", p.cgroup));
        if let Some((read, written)) = p.io {
            self.write(
                &format!("{dir}/io"),
                &format!("rchar: 0\nwchar: 0\nsyscr: 0\nsyscw: 0\nread_bytes: {read}\nwrite_bytes: {written}\ncancelled_write_bytes: 0\n"),
            );
        }
        self
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
