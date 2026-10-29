//! Every GPU and what each process uses of it (`p19a-monitor.md` choice 7): NVIDIA through
//! NVML, since its open module has no DRM fdinfo; amdgpu, i915 and xe through DRM fdinfo, for
//! the user's own processes, and sysfs.

use std::{collections::HashMap, fs, path::PathBuf, time::Instant};

use nvml_wrapper::{
    Nvml,
    enum_wrappers::device::{Clock, TemperatureSensor},
    enums::device::UsedGpuMemory,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Backend {
    Nvml,
    Fdinfo,
    /// Neither: its figures cannot be read.
    #[default]
    None,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Gpu {
    /// Short: "RTX 5070 Ti", "Radeon 610M".
    pub name: String,
    /// The PCI slot, `0000:79:00.0`.
    pub slot: String,
    pub driver: String,
    pub backend: Backend,
    /// It shares the CPU's package, so its hwmon power is the package's (choice 11).
    pub integrated: bool,
    /// 0 to 1.
    pub busy: Option<f32>,
    /// Used and total bytes.
    pub memory: Option<(u64, u64)>,
    /// Degrees Celsius.
    pub temperature: Option<f32>,
    /// Revolutions a minute.
    pub fan: Option<u32>,
    /// Watts.
    pub power: Option<f32>,
    /// Megahertz.
    pub clock: Option<u32>,
}

/// One process's use of the GPU it uses most.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct GpuUse {
    pub pid: i32,
    /// Into the sample's GPUs.
    pub gpu: usize,
    /// 0 to 1; none where the driver gave no sample since the last.
    pub busy: Option<f32>,
    pub memory: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Client {
    /// Busy time in nanoseconds, or busy and total cycles, by engine.
    engines: HashMap<String, (u64, u64)>,
    memory: u64,
}

pub struct Gpus {
    root: PathBuf,
    nvml: Option<Nvml>,
    /// Each fdinfo client's engines at the last sample, by device and client id.
    last: HashMap<(String, u64), HashMap<String, (u64, u64)>>,
    at: Option<Instant>,
    /// NVML's last process sample, in microseconds.
    nvml_seen: u64,
}

impl Gpus {
    pub fn new(root: &std::path::Path, nvml: Option<Nvml>) -> Self {
        Self {
            root: root.to_owned(),
            nvml,
            last: HashMap::new(),
            at: None,
            nvml_seen: 0,
        }
    }

    /// The GPUs and each process's use; `own` are the pids whose fdinfo can be read.
    pub fn sample(&mut self, own: &[i32]) -> (Vec<Gpu>, Vec<GpuUse>) {
        let now = Instant::now();
        let secs = self.at.map(|at| now.duration_since(at).as_secs_f64()).filter(|s| *s > 0.0);
        let mut gpus = self.devices();
        let mut uses: Vec<GpuUse> = Vec::new();
        for (index, gpu) in gpus.iter_mut().enumerate() {
            if gpu.driver == "nvidia" {
                uses.extend(self.nvml(index, gpu));
            } else if matches!(gpu.driver.as_str(), "amdgpu" | "i915" | "xe") {
                gpu.backend = Backend::Fdinfo;
                self.sysfs(gpu);
            }
        }
        let clients = self.clients(own);
        let mut seen = HashMap::new();
        for ((slot, id), (pid, client)) in clients {
            let Some(index) = gpus.iter().position(|g| g.slot == slot && g.backend == Backend::Fdinfo) else {
                continue;
            };
            let busy = secs.and_then(|secs| {
                let last = self.last.get(&(slot.clone(), id))?;
                Some(busy(&client.engines, last, secs))
            });
            seen.insert((slot, id), client.engines);
            match uses.iter_mut().find(|u| u.pid == pid && u.gpu == index) {
                Some(u) => {
                    u.memory += client.memory;
                    u.busy = match (u.busy, busy) {
                        (Some(a), Some(b)) => Some((a + b).min(1.0)),
                        (a, b) => a.or(b),
                    };
                }
                None => uses.push(GpuUse {
                    pid,
                    gpu: index,
                    busy,
                    memory: client.memory,
                }),
            }
        }
        self.last = seen;
        self.at = Some(now);
        (gpus, busiest(uses))
    }

    /// Each card under `/sys/class/drm`, by its driver and slot, with a short name.
    fn devices(&self) -> Vec<Gpu> {
        let Ok(dir) = fs::read_dir(self.root.join("sys/class/drm")) else {
            return Vec::new();
        };
        let mut cards: Vec<_> = dir
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("card") && !n.contains('-'))
            .collect();
        cards.sort();
        let ids = fs::read_to_string(self.root.join("usr/share/hwdata/pci.ids")).unwrap_or_default();
        cards
            .into_iter()
            .filter_map(|card| {
                let device = self.root.join(format!("sys/class/drm/{card}/device"));
                let uevent = fs::read_to_string(device.join("uevent")).ok()?;
                let get = |k: &str| uevent.lines().find_map(|l| l.strip_prefix(k)).unwrap_or_default().to_owned();
                let (vendor, product) = get("PCI_ID=").split_once(':').map(|(v, p)| (v.to_lowercase(), p.to_lowercase()))?;
                Some(Gpu {
                    name: pci_name(&ids, &vendor, &product).unwrap_or_else(|| format!("GPU {vendor}:{product}")),
                    slot: get("PCI_SLOT_NAME="),
                    driver: get("DRIVER="),
                    ..Default::default()
                })
            })
            .collect()
    }

    /// The device's figures from NVML and its processes' use.
    fn nvml(&mut self, index: usize, gpu: &mut Gpu) -> Vec<GpuUse> {
        let Some(nvml) = &self.nvml else { return Vec::new() };
        // NVML names a slot with an eight-digit domain.
        let Ok(device) = nvml.device_by_pci_bus_id(format!("0000{}", gpu.slot)) else {
            return Vec::new();
        };
        gpu.backend = Backend::Nvml;
        if let Ok(name) = device.name() {
            gpu.name = short_name(&name);
        }
        gpu.busy = device.utilization_rates().ok().map(|u| u.gpu as f32 / 100.0);
        gpu.memory = device.memory_info().ok().map(|m| (m.used, m.total));
        gpu.temperature = device.temperature(TemperatureSensor::Gpu).ok().map(|t| t as f32);
        gpu.fan = device.fan_speed_rpm(0).ok();
        gpu.power = device.power_usage().ok().map(|mw| mw as f32 / 1000.0);
        gpu.clock = device.clock_info(Clock::Graphics).ok();
        let mut memory: HashMap<i32, u64> = HashMap::new();
        let graphics = device.running_graphics_processes().unwrap_or_default();
        let compute = device.running_compute_processes().unwrap_or_default();
        for p in graphics.into_iter().chain(compute) {
            let used = match p.used_gpu_memory {
                UsedGpuMemory::Used(b) => b,
                UsedGpuMemory::Unavailable => 0,
            };
            *memory.entry(p.pid as i32).or_default() += used;
        }
        let mut busy: HashMap<i32, f32> = HashMap::new();
        for s in device.process_utilization_stats(self.nvml_seen).unwrap_or_default() {
            self.nvml_seen = self.nvml_seen.max(s.timestamp);
            let b = busy.entry(s.pid as i32).or_default();
            *b = b.max(s.sm_util as f32 / 100.0);
        }
        let mut pids: Vec<i32> = memory.keys().chain(busy.keys()).copied().collect();
        pids.sort();
        pids.dedup();
        pids.into_iter()
            .map(|pid| GpuUse {
                pid,
                gpu: index,
                busy: busy.get(&pid).copied(),
                memory: memory.get(&pid).copied().unwrap_or(0),
            })
            .collect()
    }

    /// amdgpu's busy percentage and VRAM, and hwmon's temperature, clock, fan and power.
    fn sysfs(&self, gpu: &mut Gpu) {
        let device = self.root.join(format!("sys/bus/pci/devices/{}", gpu.slot));
        let read = |f: &str| fs::read_to_string(device.join(f)).ok().and_then(|s| s.trim().parse::<u64>().ok());
        gpu.busy = read("gpu_busy_percent").map(|p| p as f32 / 100.0);
        gpu.memory = read("mem_info_vram_used").zip(read("mem_info_vram_total"));
        let hwmon = fs::read_dir(device.join("hwmon")).ok().and_then(|mut d| d.next()).and_then(|e| e.ok()).map(|e| e.path());
        let Some(hwmon) = hwmon else { return };
        let read = |f: &str| fs::read_to_string(hwmon.join(f)).ok().and_then(|s| s.trim().parse::<u64>().ok());
        gpu.temperature = read("temp1_input").map(|m| m as f32 / 1000.0);
        gpu.clock = read("freq1_input").map(|hz| (hz / 1_000_000) as u32);
        gpu.fan = read("fan1_input").map(|r| r as u32);
        // An integrated GPU has no fan of its own and its hwmon's PPT is the package's.
        gpu.integrated = gpu.fan.is_none() && fs::metadata(hwmon.join("fan1_input")).is_err();
        if !gpu.integrated {
            gpu.power = read("power1_average").or_else(|| read("power1_input")).map(|uw| uw as f32 / 1e6);
        }
    }

    /// Every DRM client of `own`'s processes, once per device and client id.
    fn clients(&self, own: &[i32]) -> HashMap<(String, u64), (i32, Client)> {
        let mut out = HashMap::new();
        for pid in own {
            let fds = self.root.join(format!("proc/{pid}/fd"));
            let Ok(dir) = fs::read_dir(&fds) else { continue };
            for entry in dir.flatten() {
                let Ok(target) = fs::read_link(entry.path()) else { continue };
                if !target.starts_with("/dev/dri/") {
                    continue;
                }
                let info = self.root.join(format!("proc/{pid}/fdinfo")).join(entry.file_name());
                let Ok(text) = fs::read_to_string(info) else { continue };
                if let Some((slot, id, client)) = parse_fdinfo(&text) {
                    out.entry((slot, id)).or_insert((*pid, client));
                }
            }
        }
        out
    }
}

/// One fdinfo's device, client id, engines and resident memory.
fn parse_fdinfo(text: &str) -> Option<(String, u64, Client)> {
    let mut slot = None;
    let mut id = None;
    let mut client = Client::default();
    let mut cycles: HashMap<String, (u64, u64)> = HashMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        let number = || value.split_whitespace().next().and_then(|v| v.parse::<u64>().ok());
        if key == "drm-pdev" {
            slot = Some(value.to_owned());
        } else if key == "drm-client-id" {
            id = number();
        } else if let Some(engine) = key.strip_prefix("drm-engine-") {
            if engine != "capacity" {
                client.engines.insert(engine.to_owned(), (number().unwrap_or(0), 0));
            }
        } else if let Some(engine) = key.strip_prefix("drm-total-cycles-") {
            cycles.entry(engine.to_owned()).or_default().1 = number().unwrap_or(0);
        } else if let Some(engine) = key.strip_prefix("drm-cycles-") {
            cycles.entry(engine.to_owned()).or_default().0 = number().unwrap_or(0);
        } else if let Some(region) = key.strip_prefix("drm-resident-") {
            if region != "cpu" {
                client.memory += bytes(value);
            }
        }
    }
    client.engines.extend(cycles);
    Some((slot?, id?, client))
}

/// "12 KiB", "2 MiB", "4096" as bytes.
fn bytes(value: &str) -> u64 {
    let mut parts = value.split_whitespace();
    let n: u64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    match parts.next() {
        Some("KiB") => n << 10,
        Some("MiB") => n << 20,
        Some("GiB") => n << 30,
        _ => n,
    }
}

/// The busiest engine's share since the last sample: busy nanoseconds over the interval, or
/// busy cycles over total cycles where the driver counts cycles.
fn busy(now: &HashMap<String, (u64, u64)>, last: &HashMap<String, (u64, u64)>, secs: f64) -> f32 {
    now.iter()
        .filter_map(|(engine, (busy, total))| {
            let (b0, t0) = last.get(engine)?;
            let db = busy.saturating_sub(*b0) as f64;
            let share = if *total > 0 {
                let dt = total.saturating_sub(*t0) as f64;
                if dt > 0.0 { db / dt } else { 0.0 }
            } else {
                db / (secs * 1e9)
            };
            Some(share.min(1.0) as f32)
        })
        .fold(0.0, f32::max)
}

/// Each process once, on the GPU it is busiest on, else the one it holds most memory on.
fn busiest(uses: Vec<GpuUse>) -> Vec<GpuUse> {
    let mut by_pid: HashMap<i32, GpuUse> = HashMap::new();
    for u in uses {
        match by_pid.get(&u.pid) {
            Some(kept) if (kept.busy.unwrap_or(0.0), kept.memory) >= (u.busy.unwrap_or(0.0), u.memory) => {}
            _ => {
                by_pid.insert(u.pid, u);
            }
        }
    }
    let mut out: Vec<GpuUse> = by_pid.into_values().collect();
    out.sort_by_key(|u| u.pid);
    out
}

/// The device's name in `pci.ids`: its bracketed marketing name where it has one.
fn pci_name(ids: &str, vendor: &str, product: &str) -> Option<String> {
    let mut in_vendor = false;
    for line in ids.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if !line.starts_with('\t') {
            in_vendor = line.starts_with(vendor);
            continue;
        }
        if in_vendor
            && !line.starts_with("\t\t")
            && let Some(name) = line.trim_start().strip_prefix(product)
        {
            let name = name.trim();
            let short = name.rsplit_once('[').and_then(|(_, b)| b.strip_suffix(']')).unwrap_or(name);
            return Some(short_name(short));
        }
    }
    None
}

/// The name without its vendor: "NVIDIA GeForce RTX 5070 Ti" is "RTX 5070 Ti".
fn short_name(name: &str) -> String {
    let mut name = name.trim();
    for prefix in ["NVIDIA ", "GeForce ", "AMD ", "ATI ", "Intel(R) ", "Intel "] {
        name = name.strip_prefix(prefix).unwrap_or(name);
    }
    name.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use isle_fakes::tree::Tree;

    const PCI_IDS: &str = "1002  Advanced Micro Devices, Inc. [AMD/ATI]\n\t13c0  Granite Ridge [Radeon 610M]\n\
                           \t\t1462 7e59  Board\n10de  NVIDIA Corporation\n\t2c05  GB203 [GeForce RTX 5070 Ti]\n";

    fn fdinfo(client: u64, gfx_ns: u64) -> String {
        format!(
            "pos:\t0\nflags:\t0100002\ndrm-driver:\tamdgpu\ndrm-client-id:\t{client}\ndrm-pdev:\t0000:79:00.0\n\
             drm-engine-gfx:\t{gfx_ns} ns\ndrm-engine-compute:\t0 ns\ndrm-resident-cpu:\t4 MiB\n\
             drm-resident-gtt:\t2 MiB\ndrm-resident-vram:\t12 KiB\n"
        )
    }

    /// An amdgpu card and a virtio one, with one of the user's processes holding a render node
    /// twice through one client.
    fn tree(name: &str) -> Tree {
        let tree = Tree::new(name);
        tree.write("usr/share/hwdata/pci.ids", PCI_IDS)
            .write(
                "sys/class/drm/card1/device/uevent",
                "DRIVER=amdgpu\nPCI_ID=1002:13C0\nPCI_SLOT_NAME=0000:79:00.0\n",
            )
            .write("sys/class/drm/card1-DP-1/status", "connected\n")
            .write("sys/class/drm/card0/device/uevent", "DRIVER=virtio-pci\nPCI_ID=1AF4:1050\nPCI_SLOT_NAME=0000:00:01.0\n")
            .write("sys/bus/pci/devices/0000:79:00.0/gpu_busy_percent", "25\n")
            .write("sys/bus/pci/devices/0000:79:00.0/mem_info_vram_used", "1048576\n")
            .write("sys/bus/pci/devices/0000:79:00.0/mem_info_vram_total", "2147483648\n")
            .write("sys/bus/pci/devices/0000:79:00.0/hwmon/hwmon2/temp1_input", "45000\n")
            .write("sys/bus/pci/devices/0000:79:00.0/hwmon/hwmon2/freq1_input", "600000000\n")
            .write("sys/bus/pci/devices/0000:79:00.0/hwmon/hwmon2/power1_input", "36000\n");
        std::fs::create_dir_all(tree.root.join("proc/500/fd")).unwrap();
        std::os::unix::fs::symlink("/dev/dri/renderD128", tree.root.join("proc/500/fd/7")).unwrap();
        std::os::unix::fs::symlink("/dev/dri/renderD128", tree.root.join("proc/500/fd/8")).unwrap();
        std::os::unix::fs::symlink("/home/a/file", tree.root.join("proc/500/fd/9")).unwrap();
        tree
    }

    #[test]
    fn fdinfo_gives_each_client_once_with_its_engines_share_and_memory() {
        let tree = tree("gpus-fdinfo");
        tree.write("proc/500/fdinfo/7", &fdinfo(77, 1_000_000_000))
            .write("proc/500/fdinfo/8", &fdinfo(77, 1_000_000_000))
            .write("proc/500/fdinfo/9", "pos:\t0\n");
        let mut gpus = Gpus::new(tree.path(), None);
        let (found, uses) = gpus.sample(&[500]);
        let names: Vec<&str> = found.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["GPU 1af4:1050", "Radeon 610M"]);
        assert_eq!(found[0].backend, Backend::None);
        let amd = &found[1];
        assert_eq!((amd.backend, amd.integrated), (Backend::Fdinfo, true));
        assert_eq!(amd.busy, Some(0.25));
        assert_eq!(amd.temperature, Some(45.0));
        assert_eq!(amd.clock, Some(600));
        assert_eq!(amd.power, None, "an integrated GPU's PPT is the package's");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].memory, (2 << 20) + (12 << 10), "a dup'd fd is one client");
        assert_eq!(uses[0].busy, None);

        std::thread::sleep(std::time::Duration::from_millis(100));
        tree.write("proc/500/fdinfo/7", &fdinfo(77, 1_050_000_000))
            .write("proc/500/fdinfo/8", &fdinfo(77, 1_050_000_000));
        let (_, uses) = gpus.sample(&[500]);
        let busy = uses[0].busy.unwrap();
        assert!(busy > 0.1 && busy <= 0.5, "{busy}");
    }

    #[test]
    fn cycles_and_names() {
        let now = HashMap::from([("rcs".to_owned(), (300, 1000))]);
        let last = HashMap::from([("rcs".to_owned(), (100, 600))]);
        assert!((busy(&now, &last, 1.0) - 0.5).abs() < 1e-6);
        assert_eq!(short_name("NVIDIA GeForce RTX 5070 Ti"), "RTX 5070 Ti");
        assert_eq!(pci_name(PCI_IDS, "10de", "2c05").as_deref(), Some("RTX 5070 Ti"));
        assert_eq!(bytes("2048 KiB"), 2 << 20);
    }

    /// The host's NVIDIA card through NVML; skips where there is none.
    #[test]
    fn nvml_on_the_host() {
        let Ok(nvml) = Nvml::init() else { return };
        let mut gpus = Gpus::new(std::path::Path::new("/"), Some(nvml));
        let (found, _) = gpus.sample(&[]);
        let nvidia = found.iter().find(|g| g.driver == "nvidia").expect("NVML without an nvidia card");
        assert_eq!(nvidia.backend, Backend::Nvml);
        assert!(nvidia.memory.is_some_and(|(_, total)| total > 0));
    }
}
