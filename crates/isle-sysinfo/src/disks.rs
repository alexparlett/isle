//! Whole disks' reads and writes from `/proc/diskstats`, with each drive's temperature from
//! its hwmon's reading (`p19a-monitor.md` choice 6).

use std::{collections::HashMap, fs, path::PathBuf, time::Instant};

use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Disk {
    /// The kernel's name: `nvme0n1`.
    pub name: String,
    pub model: String,
    /// Each a second since the last sample.
    pub reads: f32,
    pub writes: f32,
    pub read_bytes: u64,
    pub written_bytes: u64,
    /// Degrees Celsius.
    pub temperature: Option<f32>,
}

/// Disks Activity Monitor would not show: memory, loops and zram.
fn skipped(name: &str) -> bool {
    ["loop", "ram", "zram", "sr", "fd"].iter().any(|p| name.starts_with(p))
}

pub struct Disks {
    root: PathBuf,
    /// Reads, sectors read, writes and sectors written by disk, and when.
    last: HashMap<String, [u64; 4]>,
    at: Option<Instant>,
    /// Each disk's model and hwmon folder, read once.
    known: HashMap<String, (String, Option<String>)>,
}

impl Disks {
    pub fn new(root: &std::path::Path) -> Self {
        Self {
            root: root.to_owned(),
            last: HashMap::new(),
            at: None,
            known: HashMap::new(),
        }
    }

    /// `sensors` are this sample's hwmon readings, which hold each drive's temperature.
    pub fn sample(&mut self, sensors: &[crate::sensors::Sensor]) -> Vec<Disk> {
        let now = Instant::now();
        let secs = self.at.map(|at| now.duration_since(at).as_secs_f64()).filter(|s| *s > 0.0);
        let stats = fs::read_to_string(self.root.join("proc/diskstats")).unwrap_or_default();
        let mut seen = HashMap::new();
        let mut out = Vec::new();
        for line in stats.lines() {
            let f: Vec<&str> = line.split_whitespace().collect();
            let Some(name) = f.get(2).copied() else { continue };
            // A whole disk is a folder of `/sys/block`; its partitions are not.
            if skipped(name) || !self.root.join("sys/block").join(name).is_dir() {
                continue;
            }
            let num = |i: usize| f.get(i).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            let now_counts = [num(3), num(5), num(7), num(9)];
            let root = self.root.clone();
            let (model, hwmon) = self.known.entry(name.to_owned()).or_insert_with(|| {
                let model = fs::read_to_string(root.join(format!("sys/block/{name}/device/model")))
                    .map(|m| m.trim().to_owned())
                    .unwrap_or_default();
                (model, hwmon(&root, name))
            });
            let temperature = hwmon.as_ref().and_then(|h| {
                let id = format!("{h}/temp1");
                sensors.iter().find(|s| s.id == id).map(|s| s.value)
            });
            let mut disk = Disk {
                name: name.to_owned(),
                model: model.clone(),
                temperature,
                ..Default::default()
            };
            if let (Some(secs), Some(last)) = (secs, self.last.get(name)) {
                let rate = |i: usize| now_counts[i].saturating_sub(last[i]) as f64 / secs;
                disk.reads = rate(0) as f32;
                disk.read_bytes = (rate(1) * 512.0) as u64;
                disk.writes = rate(2) as f32;
                disk.written_bytes = (rate(3) * 512.0) as u64;
            }
            seen.insert(name.to_owned(), now_counts);
            out.push(disk);
        }
        self.last = seen;
        self.at = Some(now);
        out
    }

}

/// The hwmon folder of a disk's device, as `hwmon0`.
fn hwmon(root: &std::path::Path, name: &str) -> Option<String> {
    fs::read_dir(root.join(format!("sys/block/{name}/device")))
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|n| n.starts_with("hwmon"))
}

#[cfg(test)]
mod tests {
    use isle_fakes::tree::Tree;

    fn stats(tree: &Tree, reads: u64, sectors: u64) {
        tree.write(
            "proc/diskstats",
            &format!(
                " 259 0 nvme0n1 {reads} 0 {sectors} 0 10 0 80 0 0 0 0 0 0 0 0 0 0\n \
                 259 3 nvme0n1p1 {reads} 0 {sectors} 0 10 0 80 0 0 0 0 0 0 0 0 0 0\n \
                 251 0 zram0 5 0 5 0 5 0 5 0 0 0 0 0 0 0 0 0 0\n"
            ),
        );
    }

    #[test]
    fn whole_disks_only_with_rates_and_temperature() {
        let tree = Tree::new("disks");
        tree.write("sys/block/nvme0n1/device/model", "WD_BLACK SN850X 1000GB\n")
            .write("sys/block/nvme0n1/device/hwmon0/name", "nvme\n")
            .write("sys/block/zram0/size", "1\n");
        let sensors = [crate::sensors::Sensor {
            id: "hwmon0/temp1".into(),
            chip: "nvme".into(),
            group: "Storage".into(),
            label: "Composite".into(),
            kind: crate::sensors::Kind::Temperature,
            value: 41.85,
        }];
        stats(&tree, 100, 1000);
        let mut disks = super::Disks::new(tree.path());
        let first = disks.sample(&sensors);
        assert_eq!(first.len(), 1, "{first:?}");
        assert_eq!(first[0].model, "WD_BLACK SN850X 1000GB");
        assert_eq!(first[0].temperature, Some(41.85));
        assert_eq!(first[0].reads, 0.0);
        std::thread::sleep(std::time::Duration::from_millis(100));
        stats(&tree, 200, 3000);
        let second = &disks.sample(&sensors)[0];
        assert!(second.reads > 0.0 && second.read_bytes > 0);
        assert_eq!(second.writes, 0.0);
    }
}
