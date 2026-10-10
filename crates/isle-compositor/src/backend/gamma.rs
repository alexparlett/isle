//! A CRTC's gamma ramp, written outside `DrmCompositor`'s commits, which never touch it:
//! `GAMMA_LUT` where the driver has it, else the legacy ramp. What was there before is kept
//! and put back when the session leaves the console or ends.

use std::os::fd::AsFd;

use smithay::{
    backend::drm::DrmDevice,
    reexports::drm::control::{Device as _, crtc, property},
};
use tracing::{debug, warn};

pub enum Gamma {
    Lut {
        prop: property::Handle,
        size: usize,
        /// The blob in place when Isle took the screen; 0 for none.
        found: u64,
        /// The blob Isle set last.
        ours: Option<u64>,
    },
    Legacy {
        size: usize,
        found: Vec<u16>,
    },
}

impl Gamma {
    /// How `crtc` takes a ramp, with what is on it now; `None` when it takes none.
    pub fn probe(device: &DrmDevice, crtc: crtc::Handle) -> Option<Gamma> {
        let (mut lut, mut size) = (None, None);
        if let Ok(props) = device.get_properties(crtc) {
            for (prop, value) in props {
                let Ok(info) = device.get_property(prop) else {
                    continue;
                };
                match info.name().to_str() {
                    Ok("GAMMA_LUT") => lut = Some((prop, value)),
                    Ok("GAMMA_LUT_SIZE") => size = Some(value as usize),
                    _ => {}
                }
            }
        }
        if let (Some((prop, found)), Some(size)) = (lut, size)
            && size > 1
        {
            return Some(Gamma::Lut {
                prop,
                size,
                found,
                ours: None,
            });
        }
        let size = device.get_crtc(crtc).ok()?.gamma_length() as usize;
        if size < 2 {
            return None;
        }
        let mut found = vec![0u16; size * 3];
        let (red, rest) = found.split_at_mut(size);
        let (green, blue) = rest.split_at_mut(size);
        if let Err(err) = device.get_gamma(crtc, red, green, blue) {
            debug!("no legacy gamma to keep: {err}");
            return None;
        }
        Some(Gamma::Legacy { size, found })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Gamma::Lut { .. } => "gamma-lut",
            Gamma::Legacy { .. } => "legacy",
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Gamma::Lut { size, .. } | Gamma::Legacy { size, .. } => *size,
        }
    }

    /// Puts `ramp` (red, green, blue, `size` each) on the CRTC.
    pub fn set(&mut self, device: &DrmDevice, crtc: crtc::Handle, ramp: &[u16]) {
        match self {
            Gamma::Lut { prop, size, ours, .. } => {
                let mut bytes = Vec::with_capacity(*size * 8);
                for i in 0..*size {
                    for channel in [ramp[i], ramp[*size + i], ramp[2 * *size + i], 0] {
                        bytes.extend_from_slice(&channel.to_ne_bytes());
                    }
                }
                let blob = match drm_ffi::mode::create_property_blob(device.as_fd(), &mut bytes) {
                    Ok(blob) => blob.blob_id as u64,
                    Err(err) => {
                        warn!("could not make a gamma blob: {err}");
                        return;
                    }
                };
                if let Err(err) = device.set_property(crtc, *prop, property::Value::Blob(blob).into()) {
                    warn!("could not set GAMMA_LUT: {err}");
                    let _ = device.destroy_property_blob(blob);
                    return;
                }
                if let Some(old) = ours.replace(blob) {
                    let _ = device.destroy_property_blob(old);
                }
            }
            Gamma::Legacy { size, .. } => {
                let (red, rest) = ramp.split_at(*size);
                let (green, blue) = rest.split_at(*size);
                if let Err(err) = device.set_gamma(crtc, red, green, blue) {
                    warn!("could not set the gamma ramp: {err}");
                }
            }
        }
    }

    /// Puts back what was there when Isle took the screen.
    pub fn restore(&mut self, device: &DrmDevice, crtc: crtc::Handle) {
        match self {
            Gamma::Lut { prop, found, ours, .. } => {
                if let Err(err) = device.set_property(crtc, *prop, property::Value::Blob(*found).into()) {
                    debug!("could not put GAMMA_LUT back: {err}");
                }
                if let Some(old) = ours.take() {
                    let _ = device.destroy_property_blob(old);
                }
            }
            Gamma::Legacy { size, found } => {
                let (red, rest) = found.split_at(*size);
                let (green, blue) = rest.split_at(*size);
                let _ = device.set_gamma(crtc, red, green, blue);
            }
        }
    }
}
