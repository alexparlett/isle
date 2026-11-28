//! Finds Keychron hidraw nodes, asks them for their state and reads what they answer and push.

use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Read, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use isle_bus::Publisher;
use tracing::{debug, info};

use crate::{
    Model, Peripheral,
    protocol::{self, Channel, Kind, Report},
};

/// How often new or unplugged receivers are noticed.
const SCAN: Duration = Duration::from_secs(3);
/// How often battery and slots are asked for again; the devices push their changes between.
const REFRESH: Duration = Duration::from_secs(120);

struct Node {
    sysname: String,
    channel: Channel,
    /// The USB device's sysfs directory.
    usb: String,
    file: File,
}

impl Node {
    fn send(&mut self, command: &[u8]) -> std::io::Result<()> {
        let report = match self.channel {
            // Raw HID reports are unnumbered: a zero id, then 32 bytes.
            Channel::Raw => {
                let mut r = vec![0u8; 33];
                r[1..=command.len()].copy_from_slice(command);
                r
            }
            _ => {
                let mut r = vec![0u8; protocol::MOUSE_STATUS_LEN];
                r[..command.len()].copy_from_slice(command);
                r
            }
        };
        self.file.write_all(&report)
    }
}

pub fn run(publisher: &Publisher<Vec<Peripheral>>) -> Result<(), isle_bus::Error> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut model = Model::default();
    let mut scanned = Instant::now() - SCAN;
    let mut refreshed = Instant::now();
    // Runs until every reader has gone: a blocking task is not cancelled with its runtime.
    while !publisher.is_closed() {
        if scanned.elapsed() >= SCAN {
            scanned = Instant::now();
            for mut node in scan(&nodes) {
                info!(node = node.sysname, channel = ?node.channel, usb = node.usb, "keychron node");
                let product = read_product(Path::new(&node.usb)).unwrap_or(0);
                model
                    .devices
                    .entry(node.usb.clone())
                    .or_insert((product, Vec::new()));
                ask(&mut node, product);
                nodes.push(node);
            }
        }
        if refreshed.elapsed() >= REFRESH {
            refreshed = Instant::now();
            for node in &mut nodes {
                let product = model.devices.get(&node.usb).map_or(0, |d| d.0);
                ask(node, product);
            }
        }

        let mut fds: Vec<libc::pollfd> = nodes
            .iter()
            .map(|n| libc::pollfd {
                fd: n.file.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        // SAFETY: `fds` is a live array of `fds.len()` pollfd structs.
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 1000) };
        if ready < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() != ErrorKind::Interrupted {
                return Err(err.into());
            }
            continue;
        }
        let mut gone = Vec::new();
        for (i, fd) in fds.iter().enumerate() {
            if fd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                gone.push(i);
                continue;
            }
            if fd.revents & libc::POLLIN == 0 {
                continue;
            }
            let mut buf = [0u8; 128];
            loop {
                match nodes[i].file.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let Some(report) = protocol::parse(nodes[i].channel, &buf[..n]) else {
                            continue;
                        };
                        debug!(node = nodes[i].sysname, ?report, "keychron report");
                        let follow_up = matches!(&report, Report::Slots(s) if s.iter().any(|s| s.connected && Kind::of(s.product) == Kind::Keyboard));
                        model.apply(&nodes[i].usb.clone(), report);
                        // A receiver with a keyboard connected: ask the keyboard through it.
                        if follow_up && nodes[i].channel == Channel::Raw {
                            let _ = ask_keyboard(&mut nodes[i]);
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(_) => {
                        gone.push(i);
                        break;
                    }
                }
            }
        }
        for i in gone.into_iter().rev() {
            let node = nodes.remove(i);
            info!(node = node.sysname, "keychron node gone");
            if !nodes.iter().any(|n| n.usb == node.usb) {
                model.devices.remove(&node.usb);
            }
        }
        publisher.publish(model.peripherals());
    }
    Ok(())
}

/// Asks a node for what it knows: a receiver for its slots, a keyboard on its cable for its
/// battery and layer, the 8K receiver's mouse channel for the mouse's status.
fn ask(node: &mut Node, product: u16) {
    let result = match node.channel {
        Channel::Raw if protocol::is_receiver(product) => node.send(&protocol::SLOTS),
        Channel::Raw if Kind::of(product) == Kind::Keyboard && product != 0 => ask_keyboard(node),
        Channel::Mouse => node.send(&protocol::MOUSE_STATUS),
        _ => Ok(()),
    };
    if let Err(e) = result {
        debug!(node = node.sysname, error = %e, "keychron query");
    }
}

fn ask_keyboard(node: &mut Node) -> std::io::Result<()> {
    node.send(&protocol::BATTERY)?;
    node.send(&protocol::LAYER)
}

/// Opens the Keychron hidraw nodes not already in `open`.
fn scan(open: &[Node]) -> Vec<Node> {
    let Ok(entries) = std::fs::read_dir("/sys/class/hidraw") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let sysname = entry.file_name().to_string_lossy().into_owned();
        if open.iter().any(|n| n.sysname == sysname) {
            continue;
        }
        let Some((channel, usb)) = identify(&entry.path()) else {
            continue;
        };
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(Path::new("/dev").join(&sysname));
        match file {
            Ok(file) => found.push(Node {
                sysname,
                channel,
                usb,
                file,
            }),
            Err(e) => debug!(node = sysname, error = %e, "keychron node not opened"),
        }
    }
    found
}

/// A Keychron USB node's channel and its USB device's directory.
fn identify(hidraw: &Path) -> Option<(Channel, String)> {
    let device = std::fs::canonicalize(hidraw.join("device")).ok()?;
    // `0003:3434:D030.000F`: USB bus, vendor, product, instance.
    let id = device.file_name()?.to_str()?;
    let mut parts = id.split(':');
    if parts.next()? != "0003" || u16::from_str_radix(parts.next()?, 16).ok()? != protocol::VENDOR {
        return None;
    }
    let descriptor = std::fs::read(device.join("report_descriptor")).ok()?;
    let channel = Channel::of(&descriptor)?;
    // The HID device sits under the USB interface, which sits under the USB device.
    let usb: PathBuf = device.parent()?.parent()?.to_path_buf();
    read_product(&usb)?;
    Some((channel, usb.to_string_lossy().into_owned()))
}

fn read_product(usb: &Path) -> Option<u16> {
    let text = std::fs::read_to_string(usb.join("idProduct")).ok()?;
    u16::from_str_radix(text.trim(), 16).ok()
}
