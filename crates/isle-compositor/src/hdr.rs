//! What a screen can take of HDR10, and the connector state that asks it for HDR (P22).

use libdisplay_info::info::Info;
use smithay::{
    backend::drm::{
        DrmDevice,
        color::{Colorspace, ConnectorColorState, CtaCoordinate, Eotf, HdrOutputMetadata},
    },
    reexports::drm::control::{Device as _, connector, property},
};

/// A peak for a screen whose EDID lists ST2084 but gives no luminance.
const UNKNOWN_PEAK: f64 = 400.0;

/// An HDR10 screen's luminances in cd/m² and its chromaticities as CIE xy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HdrCaps {
    pub peak: f64,
    pub max_fall: f64,
    pub min: f64,
    /// Red, green and blue; BT.2020's when the EDID gives none.
    pub primaries: [[f64; 2]; 3],
    pub white: [f64; 2],
}

impl HdrCaps {
    /// The screen's HDR10 capabilities, when its EDID lists the PQ transfer function.
    pub fn from_info(info: &Info) -> Option<HdrCaps> {
        let meta = info.hdr_static_metadata();
        if !meta.pq {
            return None;
        }
        let mut caps = HdrCaps::from_luminances(
            meta.desired_content_max_luminance as f64,
            meta.desired_content_max_frame_avg_luminance as f64,
            meta.desired_content_min_luminance as f64,
        );
        // The panel's own chromaticities, from the base block. libdisplay-info's default
        // colour primaries are sRGB's when the EDID says sRGB is its default colour space.
        if let Some(edid) = info.edid() {
            let c = edid.chromaticity_coords();
            let xy = [c.red_x, c.red_y, c.green_x, c.green_y, c.blue_x, c.blue_y, c.white_x, c.white_y];
            if xy.iter().all(|v| *v > 0.0) {
                caps.primaries = [
                    [c.red_x as f64, c.red_y as f64],
                    [c.green_x as f64, c.green_y as f64],
                    [c.blue_x as f64, c.blue_y as f64],
                ];
                caps.white = [c.white_x as f64, c.white_y as f64];
            }
        }
        Some(caps)
    }

    /// Caps with BT.2020's primaries and D65; a zero peak is taken as unknown.
    pub fn from_luminances(peak: f64, max_fall: f64, min: f64) -> HdrCaps {
        let peak = if peak > 0.0 { peak } else { UNKNOWN_PEAK };
        HdrCaps {
            peak,
            max_fall: if max_fall > 0.0 { max_fall.min(peak) } else { peak },
            min: min.max(0.0),
            primaries: [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]],
            white: [0.3127, 0.3290],
        }
    }

    /// The `HDR_OUTPUT_METADATA` the screen is sent: its own volume as the mastering display.
    pub fn metadata(&self) -> HdrOutputMetadata {
        let at = |[x, y]: [f64; 2]| CtaCoordinate::from_xy(x, y);
        let nits = |v: f64| v.round().clamp(0.0, u16::MAX as f64) as u16;
        HdrOutputMetadata {
            eotf: Eotf::SmpteSt2084,
            display_primaries: self.primaries.map(at),
            white_point: at(self.white),
            max_display_mastering_luminance: nits(self.peak),
            // In 0.0001 cd/m².
            min_display_mastering_luminance: (self.min * 10000.0).round().clamp(0.0, u16::MAX as f64) as u16,
            max_cll: nits(self.peak),
            max_fall: nits(self.max_fall),
        }
    }

    /// The connector state for HDR10 on this screen.
    pub fn colour_state(&self) -> ConnectorColorState {
        ConnectorColorState {
            colorspace: Colorspace::Bt2020Rgb,
            hdr_metadata: Some(self.metadata()),
            max_bpc: None,
        }
    }
}

/// A screen lit in HDR10, kept in its output's user data.
#[derive(Clone, Debug)]
pub struct OutputHdr {
    pub caps: HdrCaps,
    /// The reference white in cd/m², changed live by `sdr-white`.
    pub sdr_white: std::rc::Rc<std::cell::Cell<f64>>,
    /// `sdr-colour`: SDR content kept to BT.709 at 0, stretched to the panel's gamut at 1.
    pub sdr_colour: std::rc::Rc<std::cell::Cell<f64>>,
    /// False when the swapchain fell back to 8 bits.
    pub ten_bit: bool,
}

impl OutputHdr {
    pub fn new(caps: HdrCaps, sdr_white: f64, sdr_colour: f64, ten_bit: bool) -> OutputHdr {
        OutputHdr {
            caps,
            sdr_white: std::rc::Rc::new(std::cell::Cell::new(sdr_white)),
            sdr_colour: std::rc::Rc::new(std::cell::Cell::new(sdr_colour)),
            ten_bit,
        }
    }

    /// The output's HDR state, when it is lit in HDR10.
    pub fn of(output: &smithay::output::Output) -> Option<&OutputHdr> {
        output.user_data().get::<OutputHdr>()
    }
}

impl crate::state::Isle {
    /// Redraws every HDR screen, whose final pass carries night light's warmth.
    pub fn queue_redraw_hdr(&mut self) {
        let outputs: Vec<_> = self
            .space
            .outputs()
            .filter(|o| OutputHdr::of(o).is_some())
            .cloned()
            .collect();
        for output in outputs {
            self.queue_redraw(&output);
        }
    }
}

/// Whether the connector has `Colorspace` with BT2020_RGB and `HDR_OUTPUT_METADATA`.
pub fn connector_takes_hdr(drm: &DrmDevice, conn: connector::Handle) -> bool {
    let Ok(props) = drm.get_properties(conn) else {
        return false;
    };
    let (handles, _) = props.as_props_and_values();
    let mut colorspace = false;
    let mut metadata = false;
    for &handle in handles {
        let Ok(info) = drm.get_property(handle) else {
            continue;
        };
        match info.name().to_bytes() {
            b"HDR_OUTPUT_METADATA" => metadata = true,
            b"Colorspace" => {
                if let property::ValueType::Enum(values) = info.value_type() {
                    colorspace = values.values().1.iter().any(|e| e.name().to_bytes() == b"BT2020_RGB");
                }
            }
            _ => {}
        }
    }
    colorspace && metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_is_the_screens_own_volume() {
        let caps = HdrCaps {
            primaries: [[0.6758, 0.3115], [0.2803, 0.6387], [0.1514, 0.0703]],
            white: [0.3135, 0.3291],
            ..HdrCaps::from_luminances(301.83, 301.83, 0.0)
        };
        let meta = caps.metadata();
        assert_eq!(meta.eotf, Eotf::SmpteSt2084);
        assert_eq!(meta.max_display_mastering_luminance, 302);
        assert_eq!(meta.max_cll, 302);
        assert_eq!(meta.max_fall, 302);
        assert_eq!(meta.min_display_mastering_luminance, 0);
        assert_eq!(meta.display_primaries[0], CtaCoordinate { x: 33790, y: 15575 });
        assert_eq!(meta.white_point, CtaCoordinate { x: 15675, y: 16455 });
    }

    #[test]
    fn an_edid_defaulting_to_srgb_still_gives_the_panels_primaries() {
        // The AOC U34G2G4R3's: its default colour space is sRGB, its panel near DCI-P3.
        let edid = include_bytes!("../tests/data/aoc-u34g2g4r3.edid");
        let info = Info::parse_edid(edid).unwrap();
        let caps = HdrCaps::from_info(&info).unwrap();
        assert!((caps.peak - 301.8).abs() < 0.1, "{caps:?}");
        assert!((caps.primaries[0][0] - 0.676).abs() < 0.002, "{caps:?}");
        assert!((caps.primaries[1][1] - 0.639).abs() < 0.002, "{caps:?}");
    }

    #[test]
    fn an_unknown_peak_is_assumed() {
        let caps = HdrCaps::from_luminances(0.0, 0.0, 0.05);
        assert_eq!(caps.peak, UNKNOWN_PEAK);
        assert_eq!(caps.max_fall, UNKNOWN_PEAK);
        assert_eq!(caps.metadata().min_display_mastering_luminance, 500);
    }

    #[test]
    fn hdr_asks_for_bt2020_without_touching_bpc() {
        let state = HdrCaps::from_luminances(1000.0, 400.0, 0.0).colour_state();
        assert_eq!(state.colorspace, Colorspace::Bt2020Rgb);
        assert_eq!(state.max_bpc, None);
        assert_eq!(state.hdr_metadata.unwrap().max_fall, 400);
    }
}
