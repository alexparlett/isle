//! A surface's colour, as its client describes it, brought into the blend space: BT.709
//! primaries, gamma 2.2 mirrored below zero, 1.0 the screen's reference white (P22 choice 13).

use std::collections::HashMap;

use smithay::{
    backend::renderer::{
        element::{
            Element, Id, Kind, RenderElement, UnderlyingStorage,
            surface::{WaylandSurfaceRenderElement, WaylandSurfaceTexture},
        },
        gles::{GlesError, GlesFrame, GlesRenderer, GlesTexProgram, Uniform, UniformName, UniformType, UniformValue},
        utils::{CommitCounter, DamageSet, OpaqueRegions},
    },
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Buffer, Physical, Point, Rectangle, Scale, Transform},
    wayland::{
        color::management::{
            Chromaticities, ColorManagementSurfaceCachedState, ImageDescription, Primaries, TransferFunction,
        },
        compositor::{TraversalAction, with_surface_tree_downward},
    },
};

/// The reference white an SDR screen is taken to have, in cd/m².
pub const SDR_WHITE: f64 = 203.0;

/// A PQ description's content peak when it gives none, for tone mapping.
const PQ_ASSUMED_PEAK: f64 = 1000.0;

/// GLSL that brings a premultiplied texel into the blend space. Needs the `cm_*` uniforms of
/// [`uniform_names`].
pub const CONVERT: &str = r#"
uniform float cm_on;
uniform float cm_tf;
uniform mat3 cm_to709;
uniform float cm_scale;
uniform float cm_max;
uniform float cm_white;
uniform float cm_peak;

vec3 cm_pq_eotf(vec3 e) {
    vec3 p = pow(clamp(e, 0.0, 1.0), vec3(1.0 / 78.84375));
    return pow(max(p - 0.8359375, 0.0) / (18.8515625 - 18.6875 * p), vec3(1.0 / 0.1593017578125)) * 10000.0;
}

float cm_pq(float nits) {
    float y = pow(clamp(nits / 10000.0, 0.0, 1.0), 0.1593017578125);
    return pow((0.8359375 + 18.8515625 * y) / (1.0 + 18.6875 * y), 78.84375);
}

float cm_pq_eotf1(float e) {
    float p = pow(clamp(e, 0.0, 1.0), 1.0 / 78.84375);
    return pow(max(p - 0.8359375, 0.0) / (18.8515625 - 18.6875 * p), 1.0 / 0.1593017578125) * 10000.0;
}

vec3 cm_decode(vec3 v) {
    vec3 a = abs(v);
    vec3 s = sign(v);
    if (cm_tf < 0.5) return s * pow(a, vec3(2.2));
    if (cm_tf < 1.5) return cm_pq_eotf(v);
    if (cm_tf < 2.5) return v;
    if (cm_tf < 3.5) return s * pow(a, vec3(2.4));
    if (cm_tf < 4.5) return s * pow(a, vec3(2.8));
    vec3 low = a / 12.92;
    vec3 high = pow((a + 0.055) / 1.055, vec3(2.4));
    return s * mix(low, high, step(0.04045, a));
}

// BT.2390's EETF on the brightest channel, from [0, cm_max] onto [0, cm_peak], in PQ.
vec3 cm_tone(vec3 nits) {
    float m = max(max(nits.r, nits.g), nits.b);
    if (cm_max <= cm_peak || m <= 0.0) return nits;
    float lw = cm_pq(cm_max);
    float e1 = min(cm_pq(m) / lw, 1.0);
    float max_lum = cm_pq(cm_peak) / lw;
    float ks = 1.5 * max_lum - 0.5;
    float e2 = e1;
    if (e1 > ks) {
        float t = (e1 - ks) / (1.0 - ks);
        float t2 = t * t;
        float t3 = t2 * t;
        e2 = (2.0 * t3 - 3.0 * t2 + 1.0) * ks + (t3 - 2.0 * t2 + t) * (1.0 - ks) + (-2.0 * t3 + 3.0 * t2) * max_lum;
    }
    return nits * (cm_pq_eotf1(e2 * lw) / m);
}

vec4 cm_convert(vec4 premul) {
    if (cm_on < 0.5) return premul;
    float a = premul.a;
    vec3 v = a > 0.0 ? premul.rgb / a : premul.rgb;
    vec3 nits = cm_to709 * (cm_decode(v) * cm_scale) * cm_white;
    nits = cm_tone(nits);
    vec3 rel = nits / cm_white;
    vec3 blend = sign(rel) * pow(abs(rel), vec3(1.0 / 2.2));
    return vec4(blend * a, a);
}
"#;

pub fn uniform_names() -> [UniformName<'static>; 7] {
    [
        UniformName::new("cm_on", UniformType::_1f),
        UniformName::new("cm_tf", UniformType::_1f),
        UniformName::new("cm_to709", UniformType::Matrix3x3),
        UniformName::new("cm_scale", UniformType::_1f),
        UniformName::new("cm_max", UniformType::_1f),
        UniformName::new("cm_white", UniformType::_1f),
        UniformName::new("cm_peak", UniformType::_1f),
    ]
}

/// A texture program that only converts.
const CONVERT_MAIN: &str = r#"
void main() {
    gl_FragColor = cm_convert(texture2D(tex, v_coords)) * alpha;
}
"#;

pub fn compile(renderer: &mut GlesRenderer) -> Result<GlesTexProgram, GlesError> {
    let source = format!("{}\n{CONVERT}\n{CONVERT_MAIN}", program_head());
    renderer.compile_custom_texture_shader(source, &uniform_names())
}

/// The sampler and varyings Smithay's texture programs give, for either kind of texture.
pub fn program_head() -> &'static str {
    r#"
#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif
precision highp float;
#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif
uniform float alpha;
varying vec2 v_coords;
"#
}

/// The white and peak of the screen being drawn, in cd/m²; an SDR screen's peak is its white.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub white: f64,
    pub peak: f64,
}

impl Target {
    pub const SDR: Target = Target { white: SDR_WHITE, peak: SDR_WHITE };

    pub fn of(output: &Output) -> Target {
        crate::hdr::OutputHdr::of(output).map_or(Target::SDR, |h| Target {
            white: h.sdr_white.get(),
            peak: h.caps.peak,
        })
    }
}

/// The screen being drawn's target until dropped, then SDR's.
pub struct TargetFor<'a>(&'a std::cell::Cell<Target>);

impl<'a> TargetFor<'a> {
    pub fn set(cell: &'a std::cell::Cell<Target>, target: Target) -> Self {
        cell.set(target);
        TargetFor(cell)
    }
}

impl Drop for TargetFor<'_> {
    fn drop(&mut self) {
        self.0.set(Target::SDR);
    }
}

/// Elements built while this lives are for a picture, in SDR.
pub struct Capturing<'a>(&'a std::cell::Cell<bool>, bool);

impl<'a> Capturing<'a> {
    pub fn start(cell: &'a std::cell::Cell<bool>) -> Self {
        Capturing(cell, cell.replace(true))
    }
}

impl Drop for Capturing<'_> {
    fn drop(&mut self) {
        self.0.set(self.1);
    }
}

/// How a surface's texels are brought into the blend space: the `cm_*` uniforms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Conversion {
    tf: f32,
    /// Column-major.
    to709: [f32; 9],
    scale: f32,
    /// The content's peak in the screen's cd/m², 0 when it is never tone-mapped.
    max: f32,
    white: f32,
    peak: f32,
}

impl Conversion {
    /// None for content already in the blend space: sRGB primaries with a gamma 2.2 or sRGB
    /// curve and no luminances of its own.
    pub fn of(desc: &ImageDescription, target: Target) -> Option<Conversion> {
        let srgb_primaries = desc.primaries.named == Some(Primaries::Srgb)
            || desc.primaries.values == Some(Chromaticities::from_named(Primaries::Srgb));
        let sdr_curve = matches!(desc.transfer, TransferFunction::Gamma22 | TransferFunction::Srgb);
        if srgb_primaries && sdr_curve && desc.luminances.is_none() {
            return None;
        }
        let (_, max, reference) = desc.luminances_or_default();
        let (max, reference) = (max as f64, reference.max(1) as f64);
        let tf = match desc.transfer {
            TransferFunction::Gamma22 => 0.0,
            TransferFunction::St2084Pq => 1.0,
            TransferFunction::ExtLinear => 2.0,
            TransferFunction::Bt1886 => 3.0,
            TransferFunction::Gamma28 => 4.0,
            TransferFunction::Srgb | TransferFunction::ExtSrgb | TransferFunction::CompoundPower24 => 5.0,
            _ => 0.0,
        };
        // PQ decodes to cd/m² itself; the relative curves reach `max` at 1.0.
        let scale = if tf == 1.0 { 1.0 / reference } else { max / reference };
        let primaries = desc
            .primaries
            .values
            .or(desc.primaries.named.map(Chromaticities::from_named))
            .unwrap_or(Chromaticities::from_named(Primaries::Srgb));
        let display_referred = desc.windows_scrgb || desc.windows_bt2100;
        let content_peak = desc
            .mastering_luminance
            .map(|(_, max)| max as f64)
            .or(desc.max_cll.map(f64::from))
            .unwrap_or(if tf == 1.0 { PQ_ASSUMED_PEAK } else { max });
        let max = if display_referred {
            0.0
        } else {
            content_peak / reference * target.white
        };
        Some(Conversion {
            tf,
            to709: column_major(to_bt709(&primaries)),
            scale: scale as f32,
            max: max as f32,
            white: target.white as f32,
            peak: target.peak as f32,
        })
    }

    pub fn uniforms(conversion: Option<&Conversion>) -> Vec<Uniform<'static>> {
        let off = Conversion {
            tf: 0.0,
            to709: column_major(IDENTITY),
            scale: 1.0,
            max: 0.0,
            white: 1.0,
            peak: 1.0,
        };
        let c = conversion.unwrap_or(&off);
        vec![
            Uniform::new("cm_on", if conversion.is_some() { 1.0f32 } else { 0.0 }),
            Uniform::new("cm_tf", c.tf),
            Uniform::new(
                "cm_to709",
                UniformValue::Matrix3x3 { matrices: vec![c.to709], transpose: false },
            ),
            Uniform::new("cm_scale", c.scale),
            Uniform::new("cm_max", c.max),
            Uniform::new("cm_white", c.white),
            Uniform::new("cm_peak", c.peak),
        ]
    }
}

const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

fn column_major(m: [[f64; 3]; 3]) -> [f32; 9] {
    let mut out = [0.0; 9];
    for (col, chunk) in out.chunks_exact_mut(3).enumerate() {
        for (row, v) in chunk.iter_mut().enumerate() {
            *v = m[row][col] as f32;
        }
    }
    out
}

/// RGB in `c`'s primaries to XYZ, rows by XYZ.
fn rgb_to_xyz(c: &Chromaticities) -> [[f64; 3]; 3] {
    let xyz = |(x, y): (i32, i32)| {
        let (x, y) = (x as f64 / 1e6, y as f64 / 1e6);
        [x / y, 1.0, (1.0 - x - y) / y]
    };
    let (r, g, b, w) = (xyz(c.red), xyz(c.green), xyz(c.blue), xyz(c.white));
    let m = [[r[0], g[0], b[0]], [r[1], g[1], b[1]], [r[2], g[2], b[2]]];
    let s = mul_vec(&invert(&m), w);
    [0, 1, 2].map(|row| [0, 1, 2].map(|col| m[row][col] * s[col]))
}

/// From `c`'s primaries to BT.709's in linear light; the white points are taken as one.
pub fn to_bt709(c: &Chromaticities) -> [[f64; 3]; 3] {
    let srgb = Chromaticities::from_named(Primaries::Srgb);
    mul(&invert(&rgb_to_xyz(&srgb)), &rgb_to_xyz(c))
}

fn mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    [0, 1, 2].map(|r| [0, 1, 2].map(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
}

fn mul_vec(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|r| (0..3).map(|k| m[r][k] * v[k]).sum())
}

fn invert(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    [
        [(e * i - f * h) / det, (c * h - b * i) / det, (b * f - c * e) / det],
        [(f * g - d * i) / det, (a * i - c * g) / det, (c * d - a * f) / det],
        [(d * h - e * g) / det, (b * g - a * h) / det, (a * e - b * d) / det],
    ]
}

/// Each surface of the tree under `root` whose description needs converting, by element id.
pub fn conversions(root: &WlSurface, target: Target) -> HashMap<Id, Conversion> {
    let mut found = HashMap::new();
    with_surface_tree_downward(
        root,
        (),
        |_, _, _| TraversalAction::DoChildren(()),
        // The traversal holds each surface's state: reading it again through `with_states`
        // would deadlock.
        |surface, states, _| {
            let described = states
                .cached_state
                .get::<ColorManagementSurfaceCachedState>()
                .current()
                .description;
            if let Some(desc) = described
                && let Some(conversion) = Conversion::of(&desc, target)
            {
                found.insert(Id::from_wayland_resource(surface), conversion);
            }
        },
        |_, _, _| true,
    );
    found
}

/// A surface drawn through the conversion program.
#[derive(Debug)]
pub struct ColourSurface {
    inner: WaylandSurfaceRenderElement<GlesRenderer>,
    program: GlesTexProgram,
    conversion: Conversion,
}

impl ColourSurface {
    pub fn new(inner: WaylandSurfaceRenderElement<GlesRenderer>, program: GlesTexProgram, conversion: Conversion) -> Self {
        Self { inner, program, conversion }
    }
}

impl Element for ColourSurface {
    fn id(&self) -> &Id {
        self.inner.id()
    }
    fn current_commit(&self) -> CommitCounter {
        self.inner.current_commit()
    }
    fn src(&self) -> Rectangle<f64, Buffer> {
        self.inner.src()
    }
    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.inner.geometry(scale)
    }
    fn transform(&self) -> Transform {
        self.inner.transform()
    }
    fn damage_since(&self, scale: Scale<f64>, commit: Option<CommitCounter>) -> DamageSet<i32, Physical> {
        self.inner.damage_since(scale, commit)
    }
    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        self.inner.opaque_regions(scale)
    }
    fn alpha(&self) -> f32 {
        self.inner.alpha()
    }
    fn kind(&self) -> Kind {
        self.inner.kind()
    }
    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        self.inner.location(scale)
    }
}

impl RenderElement<GlesRenderer> for ColourSurface {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&smithay::utils::user_data::UserDataMap>,
    ) -> Result<(), GlesError> {
        let WaylandSurfaceTexture::Texture(texture) = self.inner.texture() else {
            return self.inner.draw(frame, src, dst, damage, opaque_regions, cache);
        };
        frame.render_texture_from_to(
            texture,
            src,
            dst,
            damage,
            opaque_regions,
            self.inner.transform(),
            self.inner.alpha(),
            Some(&self.program),
            &Conversion::uniforms(Some(&self.conversion)),
        )
    }

    fn underlying_storage(&self, renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        self.inner.underlying_storage(renderer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) {
        for r in 0..3 {
            for c in 0..3 {
                assert!((a[r][c] - b[r][c]).abs() < 2e-3, "{a:?} against {b:?}");
            }
        }
    }

    #[test]
    fn bt709_to_itself_is_the_identity() {
        near(to_bt709(&Chromaticities::from_named(Primaries::Srgb)), IDENTITY);
    }

    #[test]
    fn bt2020_to_bt709_is_the_known_matrix() {
        near(
            to_bt709(&Chromaticities::from_named(Primaries::Bt2020)),
            [[1.6605, -0.5876, -0.0728], [-0.1246, 1.1329, -0.0083], [-0.0182, -0.1006, 1.1187]],
        );
    }

    fn pq_hdr10() -> ImageDescription {
        ImageDescription {
            transfer: TransferFunction::St2084Pq,
            primaries: smithay::wayland::color::management::PrimariesOption {
                named: Some(Primaries::Bt2020),
                values: None,
            },
            max_cll: None,
            max_fall: None,
            mastering_luminance: Some((50, 1000)),
            mastering_primaries: None,
            luminances: None,
            windows_scrgb: false,
            windows_bt2100: false,
        }
    }

    #[test]
    fn plain_srgb_is_left_alone() {
        let mut desc = pq_hdr10();
        desc.transfer = TransferFunction::Gamma22;
        desc.primaries.named = Some(Primaries::Srgb);
        desc.mastering_luminance = None;
        assert_eq!(Conversion::of(&desc, Target::SDR), None);
    }

    #[test]
    fn hdr10_maps_its_reference_to_the_screens_white_and_tone_maps_its_peak() {
        let target = Target { white: 250.0, peak: 302.0 };
        let c = Conversion::of(&pq_hdr10(), target).unwrap();
        assert_eq!(c.tf, 1.0);
        assert!((c.scale - 1.0 / 203.0).abs() < 1e-6);
        // 1000 cd/m² mastered against 203 is about 4.9 times the screen's white.
        assert!((c.max - (1000.0 / 203.0 * 250.0) as f32).abs() < 0.5);
    }

    #[test]
    fn windows_scrgb_is_display_referred_at_80_over_203() {
        let c = Conversion::of(&ImageDescription::WINDOWS_SCRGB, Target::SDR).unwrap();
        assert_eq!(c.tf, 2.0);
        assert!((c.scale - 80.0 / 203.0).abs() < 1e-6);
        assert_eq!(c.max, 0.0);
    }
}
