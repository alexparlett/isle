//! General and its About page: the computer's name, its system and its parts.

use freya::prelude::*;
use isle_ui::form;

use super::Nav;
use crate::{
    nav::Location,
    services::{Act, Follow, use_follow},
    ui::{self, Rows},
};

/// What About shows, read once on the worker.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct About {
    pub hostname: String,
    pub system: String,
    pub kernel: String,
    pub isle: String,
    pub computer: String,
    pub processor: String,
    pub graphics: Vec<String>,
    pub memory: String,
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// `PRETTY_NAME` from os-release.
pub fn os_name(text: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_owned())
        .unwrap_or_default()
}

/// The first CPU's model name.
pub fn processor(cpuinfo: &str) -> String {
    cpuinfo
        .lines()
        .find_map(|l| l.strip_prefix("model name"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim().to_owned())
        .unwrap_or_default()
}

/// Memory as installed: the kernel's total rounded up to what a machine is sold with.
pub fn memory(meminfo: &str) -> String {
    let kb: f64 = meminfo
        .lines()
        .find_map(|l| l.strip_prefix("MemTotal:"))
        .and_then(|v| v.split_whitespace().next()?.parse().ok())
        .unwrap_or(0.);
    let gib = (kb / 1024. / 1024.).ceil() as u64;
    let step = if gib > 8 { 4 } else { 1 };
    format!("{} GB", gib.div_ceil(step) * step)
}

/// A PCI device's name from `pci.ids`: the vendor's, then the device's.
pub fn pci_name(ids: &str, vendor: &str, device: &str) -> Option<String> {
    let mut in_vendor = None;
    for line in ids.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if !line.starts_with('\t') {
            in_vendor = line
                .strip_prefix(vendor)
                .map(|rest| rest.trim().to_owned())
                .filter(|_| line.len() > 4);
            continue;
        }
        if let Some(name) = &in_vendor
            && let Some(rest) = line.strip_prefix('\t').and_then(|l| l.strip_prefix(device))
            && !line.starts_with("\t\t")
        {
            return Some(format!("{name} {}", rest.trim()));
        }
    }
    in_vendor
}

fn graphics() -> Vec<String> {
    let ids = read("/usr/share/hwdata/pci.ids");
    let mut cards: Vec<String> = std::fs::read_dir("/sys/class/drm")
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.starts_with("card") && !name.contains('-')
        })
        .filter_map(|e| {
            let uevent = read(&format!("{}/device/uevent", e.path().display()));
            let id = uevent
                .lines()
                .find_map(|l| l.strip_prefix("PCI_ID="))?
                .to_lowercase();
            let (vendor, device) = id.split_once(':')?;
            Some(pci_name(&ids, vendor, device).unwrap_or(id.clone()))
        })
        .collect();
    cards.sort();
    cards.dedup();
    cards
}

async fn gather(bus: Option<zbus::Connection>) -> About {
    let mut about = About {
        system: os_name(&read("/etc/os-release")),
        kernel: read("/proc/sys/kernel/osrelease").trim().to_owned(),
        processor: processor(&read("/proc/cpuinfo")),
        graphics: graphics(),
        memory: memory(&read("/proc/meminfo")),
        hostname: read("/proc/sys/kernel/hostname").trim().to_owned(),
        ..About::default()
    };
    if let Some(bus) = bus
        && let Ok(version) = isle_ipc::version::VersionProxy::new(&bus).await
    {
        let (v, c) = (
            version.version().await.unwrap_or_default(),
            version.commit().await.unwrap_or_default(),
        );
        about.isle = if c.is_empty() {
            v
        } else {
            format!("{v} ({c})")
        };
    }
    if let Ok(system) = zbus::Connection::system().await
        && let Ok(props) = isle_bus::Props::read(
            &system,
            "org.freedesktop.hostname1",
            "/org/freedesktop/hostname1",
            "org.freedesktop.hostname1",
        )
        .await
    {
        let vendor = props.str("HardwareVendor").unwrap_or_default();
        let model = props.str("HardwareModel").unwrap_or_default();
        about.computer = format!("{vendor} {model}").trim().to_owned();
        if let Some(name) = props.str("StaticHostname").filter(|n| !n.is_empty()) {
            about.hostname = name;
        }
    }
    about
}

#[derive(PartialEq)]
pub struct General {
    pub at: Location,
}

impl Component for General {
    fn render(&self) -> impl IntoElement {
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        if self.at.sub.as_deref() == Some("About") {
            return AboutPage {
                at: self.at.clone(),
            }
            .into_element();
        }
        let at = self.at.clone();
        form::group(
            None,
            vec![
                r.row("about", "About", None)
                    .opens(None, move |_| super::open(nav, &at, "About"))
                    .into_element(),
            ],
        )
        .into_element()
    }
}

#[derive(PartialEq)]
struct AboutPage {
    at: Location,
}

impl Component for AboutPage {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Displays]);
        let r = Rows::new(&self.at);
        let worker = isle_app::worker();
        let bus = isle_app::session_bus();
        let mut about = use_state(About::default);
        let name = use_state(String::new);
        use_hook(move || {
            let job = worker.spawn(gather(bus));
            let mut name = name;
            spawn(async move {
                if let Ok(found) = job.await {
                    name.set(found.hostname.clone());
                    about.set(found);
                }
            })
        });
        let a = about.read().clone();
        let mut hardware = vec![
            r.info("computer", "Computer", a.computer.clone())
                .into_element(),
            r.info("processor", "Processor", a.processor.clone())
                .into_element(),
        ];
        for (i, gpu) in a.graphics.iter().enumerate() {
            hardware.push(
                r.info(&format!("graphics-{i}"), "Graphics", gpu.clone())
                    .into_element(),
            );
        }
        hardware.push(r.info("memory", "Memory", a.memory.clone()).into_element());
        let displays: Vec<Element> = services
            .displays
            .read()
            .state
            .iter()
            .filter(|d| d.enabled)
            .map(|d| {
                let inches = ((d.width_mm.pow(2) + d.height_mm.pow(2)) as f64).sqrt() / 25.4;
                let label = if d.model.is_empty() {
                    d.name.clone()
                } else {
                    d.model.clone()
                };
                let value = if inches > 0. {
                    format!("{}-inch, {} × {}", inches.round(), d.width, d.height)
                } else {
                    format!("{} × {}", d.width, d.height)
                };
                r.info(&d.name, &label, value).into_element()
            })
            .collect();
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(ui::sub_header(
                "monitor",
                "About",
                "This computer's name, its system and its parts.",
            ))
            .child(form::group(
                None,
                vec![
                    r.row("name", "Name", None)
                        .control(
                            isle_ui::TextField::new(name, "Name")
                                .width(Size::px(220.))
                                .on_submit(move |text: String| {
                                    let text = text.trim().to_owned();
                                    if !text.is_empty() {
                                        services.act(Act::Hostname(text))
                                    }
                                }),
                        )
                        .into_element(),
                ],
            ))
            .child(form::group(
                Some("System".into()),
                vec![
                    r.info("system", "System", a.system.clone()).into_element(),
                    r.info("kernel", "Kernel", a.kernel.clone()).into_element(),
                    r.info("isle", "Isle", a.isle.clone()).into_element(),
                ],
            ))
            .child(form::group(Some("Hardware".into()), hardware))
            .maybe(!displays.is_empty(), |el| {
                el.child(form::group(Some("Displays".into()), displays))
            })
            .into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_reads_the_systems_files() {
        assert_eq!(os_name("NAME=x\nPRETTY_NAME=\"CachyOS\"\n"), "CachyOS");
        assert_eq!(
            processor("processor : 0\nmodel name\t: AMD Ryzen 9 9950X3D\n"),
            "AMD Ryzen 9 9950X3D"
        );
        assert_eq!(memory("MemTotal:       65739512 kB\n"), "64 GB");
        assert_eq!(memory("MemTotal:       16251136 kB\n"), "16 GB");
        let ids = "10de  NVIDIA Corporation\n\t2b85  GB202 [GeForce RTX 5090]\n1002  AMD\n";
        assert_eq!(
            pci_name(ids, "10de", "2b85").as_deref(),
            Some("NVIDIA Corporation GB202 [GeForce RTX 5090]")
        );
    }
}
