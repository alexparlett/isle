//! The machine's load: CPU, memory, GPU, temperatures, network and uptime, sampled while
//! something shows them (the System widget), and the readers Monitor adds beside it.

pub mod disks;
pub mod gpus;
pub mod memory;
pub mod net;
pub mod procs;

use std::{collections::VecDeque, fs, path::Path, time::Instant};

use serde::Serialize;

/// One reading.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Sample {
    /// Every core together, 0 to 1.
    pub cpu: f32,
    pub cores: Vec<f32>,
    pub memory_used: u64,
    pub memory_total: u64,
    /// The discrete GPU's load, 0 to 1, where one can be read.
    pub gpu: Option<f32>,
    pub gpu_memory: Option<(u64, u64)>,
    /// Degrees Celsius.
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    /// Bytes a second, every interface but loopback.
    pub down: u64,
    pub up: u64,
    /// Seconds since boot.
    pub uptime: u64,
}

impl Sample {
    pub fn memory(&self) -> f32 {
        if self.memory_total == 0 {
            0.0
        } else {
            self.memory_used as f32 / self.memory_total as f32
        }
    }
}

/// Reads samples, keeping what the next one needs: the CPU's counters, the network's bytes,
/// and NVML when the driver has it.
pub struct Sampler {
    root: std::path::PathBuf,
    cpu: Vec<(u64, u64)>,
    net: Option<(u64, u64, Instant)>,
    nvml: Option<nvml_wrapper::Nvml>,
    history: VecDeque<Sample>,
    keep: usize,
}

impl Sampler {
    /// `keep` samples are kept for graphs.
    pub fn new(keep: usize) -> Self {
        Self::at(Path::new("/"), keep, nvml_wrapper::Nvml::init().ok())
    }

    /// Reads `/proc` and `/sys` under `root`, for tests.
    pub fn at(root: &Path, keep: usize, nvml: Option<nvml_wrapper::Nvml>) -> Self {
        Self {
            root: root.to_owned(),
            cpu: Vec::new(),
            net: None,
            nvml,
            history: VecDeque::new(),
            keep,
        }
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.root.join(path)).unwrap_or_default()
    }

    pub fn history(&self) -> &VecDeque<Sample> {
        &self.history
    }

    pub fn sample(&mut self) -> Sample {
        let (cpu, cores) = self.cpu();
        let (memory_used, memory_total) = memory(&self.read("proc/meminfo"));
        let (down, up) = self.net();
        let (gpu, gpu_memory, gpu_temp) = self.gpu();
        let sample = Sample {
            cpu,
            cores,
            memory_used,
            memory_total,
            gpu,
            gpu_memory,
            cpu_temp: self.cpu_temp(),
            gpu_temp,
            down,
            up,
            uptime: self
                .read("proc/uptime")
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0) as u64,
        };
        self.history.push_back(sample.clone());
        while self.history.len() > self.keep {
            self.history.pop_front();
        }
        sample
    }

    /// Busy over the time since the last sample, from `/proc/stat`; zero the first time.
    fn cpu(&mut self) -> (f32, Vec<f32>) {
        let stat = self.read("proc/stat");
        let now: Vec<(u64, u64)> = stat
            .lines()
            .take_while(|l| l.starts_with("cpu"))
            .map(|l| {
                let n: Vec<u64> = l.split_whitespace().skip(1).filter_map(|v| v.parse().ok()).collect();
                let idle = n.get(3).copied().unwrap_or(0) + n.get(4).copied().unwrap_or(0);
                (n.iter().sum::<u64>(), idle)
            })
            .collect();
        let busy: Vec<f32> = now
            .iter()
            .zip(self.cpu.iter())
            .map(|((total, idle), (t0, i0))| {
                let dt = total.saturating_sub(*t0);
                if dt == 0 {
                    0.0
                } else {
                    1.0 - idle.saturating_sub(*i0) as f32 / dt as f32
                }
            })
            .collect();
        self.cpu = now;
        match busy.split_first() {
            Some((all, cores)) => (*all, cores.to_vec()),
            None => (0.0, Vec::new()),
        }
    }

    fn net(&mut self) -> (u64, u64) {
        let (mut rx, mut tx) = (0u64, 0u64);
        for line in self.read("proc/net/dev").lines().skip(2) {
            let Some((name, rest)) = line.split_once(':') else {
                continue;
            };
            if name.trim() == "lo" {
                continue;
            }
            let n: Vec<u64> = rest.split_whitespace().filter_map(|v| v.parse().ok()).collect();
            rx += n.first().copied().unwrap_or(0);
            tx += n.get(8).copied().unwrap_or(0);
        }
        let now = Instant::now();
        let rate = match self.net {
            Some((r0, t0, at)) => {
                let secs = now.duration_since(at).as_secs_f64().max(0.001);
                (
                    (rx.saturating_sub(r0) as f64 / secs) as u64,
                    (tx.saturating_sub(t0) as f64 / secs) as u64,
                )
            }
            None => (0, 0),
        };
        self.net = Some((rx, tx, now));
        rate
    }

    /// The CPU package's temperature from hwmon: AMD's Tctl, Intel's package.
    fn cpu_temp(&self) -> Option<f32> {
        let dir = fs::read_dir(self.root.join("sys/class/hwmon")).ok()?;
        for entry in dir.flatten() {
            let name = fs::read_to_string(entry.path().join("name")).unwrap_or_default();
            if matches!(name.trim(), "k10temp" | "coretemp" | "zenpower") {
                let milli: f32 = fs::read_to_string(entry.path().join("temp1_input"))
                    .ok()?
                    .trim()
                    .parse()
                    .ok()?;
                return Some(milli / 1000.0);
            }
        }
        None
    }

    /// The discrete GPU through NVML, else an amdgpu's busy percentage and hwmon.
    fn gpu(&self) -> (Option<f32>, Option<(u64, u64)>, Option<f32>) {
        if let Some(nvml) = &self.nvml
            && let Ok(device) = nvml.device_by_index(0)
        {
            let load = device.utilization_rates().ok().map(|u| u.gpu as f32 / 100.0);
            let memory = device.memory_info().ok().map(|m| (m.used, m.total));
            let temp = device
                .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                .ok()
                .map(|t| t as f32);
            return (load, memory, temp);
        }
        let Ok(cards) = fs::read_dir(self.root.join("sys/class/drm")) else {
            return (None, None, None);
        };
        for card in cards.flatten() {
            let device = card.path().join("device");
            let Ok(busy) = fs::read_to_string(device.join("gpu_busy_percent")) else {
                continue;
            };
            let load = busy.trim().parse::<f32>().ok().map(|p| p / 100.0);
            let read = |f: &str| fs::read_to_string(device.join(f)).ok().and_then(|s| s.trim().parse::<u64>().ok());
            let memory = read("mem_info_vram_used").zip(read("mem_info_vram_total"));
            let temp = fs::read_dir(device.join("hwmon"))
                .ok()
                .and_then(|mut d| d.next())
                .and_then(|e| e.ok())
                .and_then(|e| fs::read_to_string(e.path().join("temp1_input")).ok())
                .and_then(|s| s.trim().parse::<f32>().ok())
                .map(|m| m / 1000.0);
            return (load, memory, temp);
        }
        (None, None, None)
    }
}

/// Used and total bytes from `/proc/meminfo`: total less what is available.
fn memory(meminfo: &str) -> (u64, u64) {
    let field = |name: &str| {
        meminfo
            .lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
            * 1024
    };
    let total = field("MemTotal:");
    (total.saturating_sub(field("MemAvailable:")), total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("isle-sysinfo-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for d in ["proc/net", "sys/class/hwmon/hwmon0", "sys/class/drm/card1/device/hwmon/hwmon9"] {
            fs::create_dir_all(dir.join(d)).unwrap();
        }
        fs::write(dir.join("proc/meminfo"), "MemTotal: 1000 kB\nMemFree: 100 kB\nMemAvailable: 250 kB\n").unwrap();
        fs::write(dir.join("proc/uptime"), "3725.5 100.0\n").unwrap();
        fs::write(dir.join("sys/class/hwmon/hwmon0/name"), "k10temp\n").unwrap();
        fs::write(dir.join("sys/class/hwmon/hwmon0/temp1_input"), "52500\n").unwrap();
        fs::write(dir.join("sys/class/drm/card1/device/gpu_busy_percent"), "40\n").unwrap();
        fs::write(dir.join("sys/class/drm/card1/device/hwmon/hwmon9/temp1_input"), "61000\n").unwrap();
        dir
    }

    fn stat(dir: &Path, total: u64, idle: u64) {
        fs::write(
            dir.join("proc/stat"),
            format!("cpu  {} 0 0 {idle} 0 0 0 0 0 0\ncpu0 {} 0 0 {idle} 0 0 0 0 0 0\nintr 1\n", total - idle, total - idle),
        )
        .unwrap();
    }

    fn net(dir: &Path, rx: u64, tx: u64) {
        fs::write(
            dir.join("proc/net/dev"),
            format!("h\nh\n    lo: 999 0 0 0 0 0 0 0 999 0 0 0 0 0 0 0\n  eth0: {rx} 0 0 0 0 0 0 0 {tx} 0 0 0 0 0 0 0\n"),
        )
        .unwrap();
    }

    #[test]
    fn a_sample_reads_load_memory_temperatures_and_uptime() {
        let dir = root("sample");
        stat(&dir, 1000, 800);
        net(&dir, 1000, 500);
        let mut s = Sampler::at(&dir, 3, None);
        let first = s.sample();
        assert_eq!(first.cpu, 0.0, "the first sample has nothing to compare with");
        stat(&dir, 2000, 1300);
        net(&dir, 3000, 900);
        let second = s.sample();
        assert!((second.cpu - 0.5).abs() < 1e-6);
        assert_eq!(second.cores.len(), 1);
        assert_eq!((second.memory_used, second.memory_total), (750 * 1024, 1000 * 1024));
        assert!((second.memory() - 0.75).abs() < 1e-6);
        assert_eq!(second.cpu_temp, Some(52.5));
        assert_eq!(second.gpu, Some(0.4));
        assert_eq!(second.gpu_temp, Some(61.0));
        assert_eq!(second.uptime, 3725);
        assert!(second.down > 0 && second.up > 0, "loopback left out, the rest counted");
        for _ in 0..5 {
            s.sample();
        }
        assert_eq!(s.history().len(), 3);
        let _ = fs::remove_dir_all(&dir);
    }
}
