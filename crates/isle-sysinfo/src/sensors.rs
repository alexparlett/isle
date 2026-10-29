//! Every hwmon reading, labelled for people (`p19a-monitor.md` choice 10).

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Kind {
    /// Degrees Celsius.
    Temperature,
    /// Revolutions a minute.
    Fan,
    /// Volts.
    Voltage,
    /// Amperes.
    Current,
    /// Watts.
    Power,
    /// Megahertz.
    Clock,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Sensor {
    /// The hwmon folder and the reading's file stem, as `hwmon3/fan1`: stable while booted.
    pub id: String,
    /// The chip's own name: `k10temp`, `nct6687`.
    pub chip: String,
    /// What the chip is, for people: "CPU", "Motherboard".
    pub group: String,
    pub label: String,
    pub kind: Kind,
    pub value: f32,
}

/// What a chip is, by its driver's name.
fn group(chip: &str) -> &str {
    match chip {
        "k10temp" | "coretemp" | "zenpower" => "CPU",
        "amdgpu" | "nouveau" | "i915" | "xe" => "GPU",
        "nvme" | "drivetemp" => "Storage",
        "spd5118" | "jc42" => "Memory",
        c if c.starts_with("nct") || c.starts_with("it87") || c.starts_with("asus") => "Motherboard",
        c if c.starts_with("iwlwifi") || c.starts_with("mt79") => "Wi-Fi",
        c if c.starts_with("r8169") || c.starts_with("igc") => "Ethernet",
        c if c.ends_with("_xhci") => "USB",
        c => c,
    }
}

/// A reading's label: the chip's own where it gives one, with the generic ones renamed.
fn label(chip: &str, stem: &str, given: Option<&str>, kind: Kind) -> String {
    match (chip, given) {
        ("k10temp" | "zenpower", Some("Tctl" | "Tdie")) => "CPU package".into(),
        ("k10temp", Some(l)) if l.starts_with("Tccd") => format!("CPU die {}", &l[4..]),
        ("coretemp", Some(l)) if l.starts_with("Package id") => "CPU package".into(),
        (_, Some(l)) if !l.is_empty() => l.into(),
        _ => {
            let n = stem.trim_start_matches(|c: char| c.is_ascii_alphabetic());
            match kind {
                Kind::Fan => format!("Fan {n}"),
                Kind::Temperature if n == "1" => "Temperature".into(),
                Kind::Temperature => format!("Temperature {n}"),
                Kind::Voltage => format!("Voltage {n}"),
                Kind::Current => format!("Current {n}"),
                Kind::Power => format!("Power {n}"),
                Kind::Clock => format!("Clock {n}"),
            }
        }
    }
}

/// How long a slow chip's readings are kept, for chips whose reads are commands to a drive,
/// transfers on a slow bus, or a Super I/O chip's full refresh, which its driver does once a
/// second. Reading an NVMe drive's temperature also keeps it out of its deeper power states.
fn slow(chip: &str) -> Option<Duration> {
    match chip {
        "nvme" | "drivetemp" | "spd5118" | "jc42" => Some(Duration::from_secs(10)),
        c if c.starts_with("iwlwifi") => Some(Duration::from_secs(10)),
        c if c.starts_with("nct") || c.starts_with("it87") => Some(Duration::from_secs(1)),
        _ => None,
    }
}

/// Slow chips' last readings, by hwmon folder, and those being read.
#[derive(Default)]
struct Kept {
    readings: HashMap<String, (Instant, Vec<Sensor>)>,
    reading: HashSet<String>,
}

/// Reads hwmon, remembering which fans have turned since it was made. A slow chip is read on
/// a thread of its own when its readings are old, and a sample takes the last it has.
pub struct Sensors {
    root: PathBuf,
    turned: HashSet<String>,
    kept: std::sync::Arc<std::sync::Mutex<Kept>>,
}

impl Sensors {
    pub fn new(root: &std::path::Path) -> Self {
        Self {
            root: root.to_owned(),
            turned: HashSet::new(),
            kept: Default::default(),
        }
    }

    pub fn read(&mut self) -> Vec<Sensor> {
        let mut out = Vec::new();
        let Ok(dir) = fs::read_dir(self.root.join("sys/class/hwmon")) else {
            return out;
        };
        let mut chips: Vec<PathBuf> = dir.flatten().map(|e| e.path()).collect();
        chips.sort_by_key(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy().into_owned();
            name.trim_start_matches("hwmon").parse::<u32>().unwrap_or(u32::MAX)
        });
        for path in chips {
            let chip = fs::read_to_string(path.join("name")).unwrap_or_default().trim().to_owned();
            let folder = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let Some(every) = slow(&chip) else {
                let readings = read_chip(&path, &chip, &folder);
                for r in &readings {
                    if r.kind == Kind::Fan && r.value > 0.0 {
                        self.turned.insert(r.id.clone());
                    }
                }
                out.extend(readings);
                continue;
            };
            let mut kept = self.kept.lock().unwrap();
            let fresh = kept.readings.get(&folder).is_some_and(|(at, _)| at.elapsed() < every);
            if let Some((_, readings)) = kept.readings.get(&folder) {
                for r in readings {
                    if r.kind == Kind::Fan && r.value > 0.0 {
                        self.turned.insert(r.id.clone());
                    }
                }
                out.extend(readings.iter().cloned());
            }
            if !fresh && kept.reading.insert(folder.clone()) {
                let shared = self.kept.clone();
                std::thread::spawn(move || {
                    let readings = read_chip(&path, &chip, &folder);
                    let mut kept = shared.lock().unwrap();
                    kept.reading.remove(&folder);
                    kept.readings.insert(folder, (Instant::now(), readings));
                });
            }
        }
        out
    }

    /// Whether `sensor`, a fan, has turned since sampling began.
    pub fn has_turned(&self, sensor: &Sensor) -> bool {
        self.turned.contains(&sensor.id)
    }

    /// The CPU's package temperature.
    pub fn cpu_temperature(sensors: &[Sensor]) -> Option<f32> {
        sensors
            .iter()
            .find(|s| s.group == "CPU" && s.kind == Kind::Temperature && s.label == "CPU package")
            .or_else(|| sensors.iter().find(|s| s.group == "CPU" && s.kind == Kind::Temperature))
            .map(|s| s.value)
    }

    /// The CPU's fan: one labelled CPU, else the first that has turned.
    pub fn cpu_fan<'a>(&self, sensors: &'a [Sensor]) -> Option<&'a Sensor> {
        let fans = || sensors.iter().filter(|s| s.kind == Kind::Fan);
        fans()
            .find(|s| s.label.to_lowercase().contains("cpu"))
            .or_else(|| fans().find(|s| self.has_turned(s)))
    }
}

/// One chip's readings, each `<stem>_input` with its label.
fn read_chip(path: &std::path::Path, chip: &str, folder: &str) -> Vec<Sensor> {
    let Ok(files) = fs::read_dir(path) else { return Vec::new() };
    let mut stems: Vec<String> = files
        .flatten()
        .filter_map(|f| f.file_name().to_str()?.strip_suffix("_input").map(str::to_owned))
        .collect();
    stems.sort_by_key(|s| {
        let (kind, n) = s.split_at(s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len()));
        (kind.to_owned(), n.parse::<u32>().unwrap_or(0))
    });
    let mut out = Vec::new();
    for stem in stems {
        let (kind, scale) = match stem.trim_end_matches(|c: char| c.is_ascii_digit()) {
            "temp" => (Kind::Temperature, 1e-3),
            "fan" => (Kind::Fan, 1.0),
            "in" => (Kind::Voltage, 1e-3),
            "curr" => (Kind::Current, 1e-3),
            "power" => (Kind::Power, 1e-6),
            "freq" => (Kind::Clock, 1e-6),
            _ => continue,
        };
        let Some(raw) = fs::read_to_string(path.join(format!("{stem}_input")))
            .ok()
            .and_then(|v| v.trim().parse::<f64>().ok())
        else {
            continue;
        };
        let given = fs::read_to_string(path.join(format!("{stem}_label"))).ok();
        let value = if kind == Kind::Power { power_watts(raw) } else { (raw * scale) as f32 };
        out.push(Sensor {
            id: format!("{folder}/{stem}"),
            label: label(chip, &stem, given.as_deref().map(str::trim), kind),
            group: group(chip).to_owned(),
            chip: chip.to_owned(),
            kind,
            value,
        });
    }
    out
}

/// hwmon's power in watts. The ABI says microwatts, but amdgpu on some APUs reports
/// milliwatts; a reading under a watt as microwatts is milliwatts (`apps-2026.md` §3).
pub fn power_watts(raw: f64) -> f32 {
    if raw < 1e6 { (raw / 1e3) as f32 } else { (raw / 1e6) as f32 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use isle_fakes::tree::Tree;

    fn host_like(name: &str) -> Tree {
        let tree = Tree::new(name);
        tree.write("sys/class/hwmon/hwmon6/name", "k10temp\n")
            .write("sys/class/hwmon/hwmon6/temp1_input", "61250\n")
            .write("sys/class/hwmon/hwmon6/temp1_label", "Tctl\n")
            .write("sys/class/hwmon/hwmon6/temp3_input", "55000\n")
            .write("sys/class/hwmon/hwmon6/temp3_label", "Tccd1\n")
            .write("sys/class/hwmon/hwmon3/name", "nct6687\n")
            .write("sys/class/hwmon/hwmon3/fan1_input", "0\n")
            .write("sys/class/hwmon/hwmon3/fan2_input", "1542\n")
            .write("sys/class/hwmon/hwmon3/in0_input", "1208\n")
            .write("sys/class/hwmon/hwmon3/in0_label", "+12V\n")
            .write("sys/class/hwmon/hwmon2/name", "amdgpu\n")
            .write("sys/class/hwmon/hwmon2/power1_input", "36000\n")
            .write("sys/class/hwmon/hwmon2/power1_label", "PPT\n")
            .write("sys/class/hwmon/hwmon2/freq1_input", "600000000\n")
            .write("sys/class/hwmon/hwmon10/name", "iwlwifi_1\n")
            .write("sys/class/hwmon/hwmon10/temp1_input", "40000\n");
        tree
    }

    /// Reads until the slow chips' readings have come: `count` readings in all.
    fn settled(sensors: &mut Sensors, count: usize) -> Vec<Sensor> {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let got = sensors.read();
            if got.len() >= count || Instant::now() > deadline {
                return got;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn readings_are_labelled_scaled_and_the_cpus_found() {
        let tree = host_like("sensors");
        let mut sensors = Sensors::new(tree.path());
        let all = settled(&mut sensors, 8);
        let get = |id: &str| all.iter().find(|s| s.id == id).unwrap();
        assert_eq!(get("hwmon6/temp1").label, "CPU package");
        assert_eq!(get("hwmon6/temp3").label, "CPU die 1");
        assert_eq!(get("hwmon6/temp1").value, 61.25);
        assert_eq!(get("hwmon3/fan2").label, "Fan 2");
        assert_eq!(get("hwmon3/fan2").group, "Motherboard");
        assert!((get("hwmon3/in0").value - 1.208).abs() < 1e-6);
        assert_eq!(get("hwmon2/power1").value, 36.0, "the APU's milliwatts");
        assert_eq!(get("hwmon2/freq1").value, 600.0);
        assert_eq!(get("hwmon10/temp1").label, "Temperature");
        assert_eq!(get("hwmon10/temp1").group, "Wi-Fi");
        assert_eq!(Sensors::cpu_temperature(&all), Some(61.25));
        assert_eq!(sensors.cpu_fan(&all).map(|s| s.id.as_str()), Some("hwmon3/fan2"));
        assert!(!sensors.has_turned(get("hwmon3/fan1")));
    }

    #[test]
    fn a_slow_chip_is_read_aside_and_given_on_a_later_read() {
        let tree = Tree::new("sensors-slow");
        tree.write("sys/class/hwmon/hwmon0/name", "nvme\n")
            .write("sys/class/hwmon/hwmon0/temp1_input", "32850\n")
            .write("sys/class/hwmon/hwmon0/temp1_label", "Composite\n");
        let mut sensors = Sensors::new(tree.path());
        assert!(sensors.read().is_empty(), "the first read only asks");
        let got = settled(&mut sensors, 1);
        assert_eq!(got[0].label, "Composite");
        tree.write("sys/class/hwmon/hwmon0/temp1_input", "50000\n");
        assert_eq!(sensors.read()[0].value, 32.85, "kept until it is old");
    }

    #[test]
    fn power_is_read_in_either_unit() {
        assert_eq!(power_watts(36_000.0), 36.0);
        assert_eq!(power_watts(36_000_000.0), 36.0);
        assert_eq!(power_watts(4_500_000.0), 4.5);
    }
}
