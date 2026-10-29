//! The CPU package's power where it can be read without root, and energy impact
//! (`p19a-monitor.md` choices 11 and 12).

use std::{fs, path::PathBuf, time::Instant};

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Source {
    /// RAPL's package counter, when it can be read.
    Rapl,
    /// An integrated amdgpu's PPT: the socket's power.
    Ppt,
}

pub struct CpuPower {
    root: PathBuf,
    rapl: Option<(u64, Instant)>,
}

impl CpuPower {
    pub fn new(root: &std::path::Path) -> Self {
        Self {
            root: root.to_owned(),
            rapl: None,
        }
    }

    /// Watts and where they came from; none where neither can be read. RAPL needs two
    /// samples, so its first gives none.
    pub fn sample(&mut self, gpus: &[crate::gpus::Gpu]) -> Option<(f32, Source)> {
        let rapl = self.root.join("sys/class/powercap/intel-rapl:0");
        if let Some(energy) = fs::read_to_string(rapl.join("energy_uj")).ok().and_then(|v| v.trim().parse::<u64>().ok()) {
            let now = Instant::now();
            let last = self.rapl.replace((energy, now));
            let (e0, at) = last?;
            let range = fs::read_to_string(rapl.join("max_energy_range_uj"))
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
                .unwrap_or(u64::MAX);
            let used = if energy >= e0 { energy - e0 } else { energy + range - e0 };
            let secs = now.duration_since(at).as_secs_f64().max(1e-3);
            return Some(((used as f64 / 1e6 / secs) as f32, Source::Rapl));
        }
        let gpu = gpus.iter().find(|g| g.integrated && g.driver == "amdgpu")?;
        let dir = fs::read_dir(self.root.join(format!("sys/bus/pci/devices/{}/hwmon", gpu.slot)))
            .ok()?
            .flatten()
            .next()?
            .path();
        let raw = fs::read_to_string(dir.join("power1_input"))
            .or_else(|_| fs::read_to_string(dir.join("power1_average")))
            .ok()?
            .trim()
            .parse::<f64>()
            .ok()?;
        Some((crate::sensors::power_watts(raw), Source::Ppt))
    }
}

/// Activity Monitor's relative measure: 1 for every 10 ms of CPU a second, every percent of
/// a GPU's time and every 100 context switches a second. `cpu` is in cores, `gpu` 0 to 1.
pub fn energy_impact(cpu: f32, gpu: f32, switches: f32) -> f32 {
    cpu * 100.0 + gpu * 100.0 + switches / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpus::Gpu;
    use isle_fakes::tree::Tree;

    fn apu() -> Gpu {
        Gpu {
            slot: "0000:79:00.0".into(),
            driver: "amdgpu".into(),
            integrated: true,
            ..Default::default()
        }
    }

    #[test]
    fn rapl_when_readable_else_the_apus_ppt_else_none() {
        let tree = Tree::new("power");
        tree.write("sys/bus/pci/devices/0000:79:00.0/hwmon/hwmon2/power1_input", "38500\n");
        let mut power = CpuPower::new(tree.path());
        assert_eq!(power.sample(&[apu()]), Some((38.5, Source::Ppt)));
        assert_eq!(power.sample(&[]), None, "no APU, no RAPL");

        tree.write("sys/class/powercap/intel-rapl:0/energy_uj", "1000000\n")
            .write("sys/class/powercap/intel-rapl:0/max_energy_range_uj", "65532610987\n");
        assert_eq!(power.sample(&[apu()]), None, "RAPL's first sample has no interval");
        std::thread::sleep(std::time::Duration::from_millis(100));
        tree.write("sys/class/powercap/intel-rapl:0/energy_uj", "5000000\n");
        let (watts, source) = power.sample(&[apu()]).unwrap();
        assert_eq!(source, Source::Rapl);
        assert!(watts > 10.0 && watts < 45.0, "{watts}");
    }

    #[test]
    fn a_game_reads_about_250_and_an_idle_helper_under_1() {
        assert!((energy_impact(1.5, 0.99, 0.0) - 249.0).abs() < 1e-3);
        assert!(energy_impact(0.002, 0.0, 30.0) < 1.0);
    }
}
