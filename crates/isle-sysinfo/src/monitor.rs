//! One Monitor sample: every reader together, and what it cost.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::Serialize;

use crate::{disks, fds, gpus, memory, net, power, procs, sensors};

/// The CPU's time since the last sample, each 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct CpuSplit {
    pub user: f32,
    pub system: f32,
    pub idle: f32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Full {
    pub cpu: CpuSplit,
    pub processes: Vec<procs::Process>,
    pub memory: memory::Memory,
    pub disks: Vec<disks::Disk>,
    pub gpus: Vec<gpus::Gpu>,
    pub gpu_uses: Vec<gpus::GpuUse>,
    pub net_uses: Vec<net::NetUse>,
    /// Bytes and packets received, then sent, since boot.
    pub interfaces: [u64; 4],
    pub sensors: Vec<sensors::Sensor>,
    pub cpu_power: Option<(f32, power::Source)>,
    pub uid: u32,
    /// The time the sample took, and each reader's part of it.
    pub cost: Duration,
    pub parts: Vec<(&'static str, Duration)>,
}

pub struct MonitorSampler {
    root: PathBuf,
    uid: u32,
    stat: Option<[u64; 3]>,
    procs: procs::Procs,
    disks: disks::Disks,
    gpus: gpus::Gpus,
    net: net::Net,
    sensors: sensors::Sensors,
    power: power::CpuPower,
    fds: fds::Scanner,
}

impl MonitorSampler {
    pub fn new() -> Self {
        Self::at(Path::new("/"), nvml_wrapper::Nvml::init().ok())
    }

    pub fn at(root: &Path, nvml: Option<nvml_wrapper::Nvml>) -> Self {
        Self {
            root: root.to_owned(),
            // SAFETY: getuid cannot fail.
            uid: unsafe { libc::getuid() },
            stat: None,
            procs: procs::Procs::new(root),
            disks: disks::Disks::new(root),
            gpus: gpus::Gpus::new(root, nvml),
            net: net::Net::new(root),
            sensors: sensors::Sensors::new(root),
            power: power::CpuPower::new(root),
            fds: fds::Scanner::default(),
        }
    }

    pub fn sensors(&self) -> &sensors::Sensors {
        &self.sensors
    }

    /// Every figure; the sockets come from the kernel, not the root.
    pub fn sample(&mut self) -> Full {
        let start = Instant::now();
        let mut parts = Vec::new();
        let mut lap = Instant::now();
        let mut mark = |name: &'static str| {
            parts.push((name, lap.elapsed()));
            lap = Instant::now();
        };
        let processes = self.procs.sample();
        mark("processes");
        let own: Vec<i32> = processes.iter().filter(|p| p.uid == self.uid && !p.kernel_thread).map(|p| p.pid()).collect();
        let fds = self.fds.scan(&self.root, &own);
        mark("descriptors");
        let (gpus, gpu_uses) = self.gpus.sample(&fds.drm);
        mark("gpus");
        let cgroups: Vec<(i32, String)> = processes.iter().map(|p| (p.pid(), p.cgroup.clone())).collect();
        let sockets = net::tcp_sockets().unwrap_or_default();
        let net_uses = self.net.sample(&sockets, &fds.sockets, &cgroups);
        mark("network");
        let cpu_power = self.power.sample(&gpus);
        let sensors = self.sensors.read();
        mark("sensors");
        let cpu = self.cpu();
        let memory = memory::read(&self.root);
        let disks = self.disks.sample(&sensors);
        mark("cpu, memory, disks");
        Full {
            cpu,
            memory,
            disks,
            gpus,
            gpu_uses,
            net_uses,
            interfaces: net::interfaces(&self.root),
            sensors,
            cpu_power,
            uid: self.uid,
            processes,
            cost: start.elapsed(),
            parts,
        }
    }

    fn cpu(&mut self) -> CpuSplit {
        let stat = std::fs::read_to_string(self.root.join("proc/stat")).unwrap_or_default();
        let n: Vec<u64> = stat
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .skip(1)
            .filter_map(|v| v.parse().ok())
            .collect();
        let get = |i: usize| n.get(i).copied().unwrap_or(0);
        // user, nice; system, irq, softirq; idle, iowait.
        let now = [get(0) + get(1), get(2) + get(5) + get(6), get(3) + get(4)];
        let last = self.stat.replace(now);
        let Some(last) = last else { return CpuSplit::default() };
        let d: Vec<f32> = now.iter().zip(last).map(|(a, b)| a.saturating_sub(b) as f32).collect();
        let total: f32 = d.iter().sum();
        if total == 0.0 {
            return CpuSplit::default();
        }
        CpuSplit {
            user: d[0] / total,
            system: d[1] / total,
            idle: d[2] / total,
        }
    }
}

impl Default for MonitorSampler {
    fn default() -> Self {
        Self::new()
    }
}
