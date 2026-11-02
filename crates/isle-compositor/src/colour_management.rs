//! wp-color-management-v1: clients say what colour their content is in, and each screen says
//! what it shows (P22 choice 19).

use smithay::{
    desktop::utils::surface_primary_scanout_output,
    output::Output,
    reexports::wayland_server::{DisplayHandle, protocol::wl_surface::WlSurface},
    wayland::{
        color::management::{
            Chromaticities, ColorManagementHandler, ColorManagementState, Feature, ImageDescription, Primaries,
            PrimariesOption, RenderIntent, TransferFunction, send_image_description_info,
        },
        compositor::with_states,
        seat::WaylandFocus,
    },
};
use smithay::reexports::wayland_protocols::wp::color_management::v1::server::wp_image_description_info_v1::WpImageDescriptionInfoV1;

use crate::state::{Isle, State};

/// What nested gamescope and Mesa's WSI look for before they offer HDR and scRGB.
pub fn create_state(dh: &DisplayHandle) -> ColorManagementState {
    ColorManagementState::new::<State, _>(
        dh,
        [
            TransferFunction::Gamma22,
            TransferFunction::St2084Pq,
            TransferFunction::ExtLinear,
            TransferFunction::Bt1886,
            TransferFunction::CompoundPower24,
        ],
        [Primaries::Srgb, Primaries::Bt2020, Primaries::DisplayP3],
        [
            Feature::Parametric,
            Feature::SetPrimaries,
            Feature::SetLuminances,
            Feature::SetMasteringDisplayPrimaries,
            Feature::ExtendedTargetVolume,
            Feature::WindowsScrgb,
            Feature::WindowsBt2100,
        ],
        [RenderIntent::Perceptual],
        |_| true,
    )
}

/// What `output` shows: BT.2020 PQ at its reference white and peak in HDR, else sRGB's
/// primaries with gamma 2.2.
pub fn output_description(output: &Output) -> ImageDescription {
    let Some(hdr) = crate::hdr::OutputHdr::of(output) else {
        return ImageDescription {
            transfer: TransferFunction::Gamma22,
            ..ImageDescription::SRGB
        };
    };
    let caps = hdr.caps;
    let at = |[x, y]: [f64; 2]| ((x * 1e6).round() as i32, (y * 1e6).round() as i32);
    ImageDescription {
        transfer: TransferFunction::St2084Pq,
        primaries: PrimariesOption {
            named: Some(Primaries::Bt2020),
            values: None,
        },
        max_cll: Some(caps.peak.round() as u32),
        max_fall: Some(caps.max_fall.round() as u32),
        mastering_luminance: Some(((caps.min * 10000.0).round() as u32, caps.peak.round() as u32)),
        mastering_primaries: Some(Chromaticities {
            red: at(caps.primaries[0]),
            green: at(caps.primaries[1]),
            blue: at(caps.primaries[2]),
            white: at(caps.white),
        }),
        // PQ's own range; the reference is the screen's.
        luminances: Some((50, 10_000, hdr.sdr_white.get().round() as u32)),
        windows_scrgb: false,
        windows_bt2100: false,
    }
}

/// The description a surface had at its last commit.
struct Committed(std::cell::Cell<Option<ImageDescription>>);

/// Whether this commit changed the surface's description. A change commits no pixels, so the
/// caller redraws.
pub fn changed_at_commit(surface: &WlSurface) -> bool {
    with_states(surface, |states| {
        let now = states
            .cached_state
            .get::<smithay::wayland::color::management::ColorManagementSurfaceCachedState>()
            .current()
            .description;
        states.data_map.insert_if_missing(|| Committed(std::cell::Cell::new(None)));
        let committed = states.data_map.get::<Committed>().expect("inserted above");
        committed.0.replace(now) != now
    })
}

impl ColorManagementHandler for State {
    fn color_management_state(&mut self) -> &mut ColorManagementState {
        &mut self.isle.color_management
    }

    fn description_for_output(&mut self, output: &Output) -> ImageDescription {
        output_description(output)
    }

    fn preferred_description_for_surface(&mut self, surface: &WlSurface) -> ImageDescription {
        self.isle
            .output_of_surface(surface)
            .map_or(ImageDescription { transfer: TransferFunction::Gamma22, ..ImageDescription::SRGB }, |o| {
                output_description(&o)
            })
    }

    fn schedule_image_description_info(&mut self, info: WpImageDescriptionInfoV1, desc: ImageDescription) {
        self.isle
            .loop_handle
            .insert_idle(move |_| send_image_description_info(&info, &desc));
    }
}

impl Isle {
    /// The screen a surface is mostly on: where its frames go, else its window's screen.
    fn output_of_surface(&self, surface: &WlSurface) -> Option<Output> {
        with_states(surface, |states| surface_primary_scanout_output(surface, states))
            .or_else(|| {
                self.space
                    .elements()
                    .find(|w| w.wl_surface().is_some_and(|s| *s == *surface))
                    .and_then(|w| self.space.outputs_for_element(w).into_iter().next())
            })
            .or_else(|| self.space.outputs().next().cloned())
    }

    /// Tells every screen's colour objects and every window's feedback what its screen now
    /// shows, after a screen's HDR or its reference white changed.
    pub fn refresh_colour_preferences(&mut self) {
        let outputs: Vec<Output> = self.space.outputs().cloned().collect();
        for output in &outputs {
            self.color_management.output_description_changed(output);
        }
        let surfaces: Vec<WlSurface> = self
            .space
            .elements()
            .filter_map(|w| w.wl_surface().map(|s| s.into_owned()))
            .collect();
        for surface in surfaces {
            if let Some(output) = self.output_of_surface(&surface) {
                self.color_management
                    .preferred_changed(&surface, output_description(&output));
            }
        }
    }
}
