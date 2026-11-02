//! The HDR composite: a screen's frame drawn into extended sRGB at FP16, then one pass to
//! BT.2020 PQ (P22 choice 10).

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            Bind, Offscreen, Texture as _,
            damage::OutputDamageTracker,
            element::{Element, Id, Kind, RenderElement, RenderElementStates, UnderlyingStorage},
            gles::{GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform, UniformName, UniformType},
            utils::{CommitCounter, DamageBag, DamageSet, DamageSnapshot, OpaqueRegions},
        },
    },
    output::Output,
    utils::{Buffer, Physical, Point, Rectangle, Scale, Size, Transform},
};

use crate::render::IsleElement;

/// The intermediate holds BT.709 primaries in a gamma 2.2 encoding mirrored below zero, 1.0
/// being the reference white, so SDR content is already in it. This decodes it, scales it to
/// cd/m², widens it to BT.2020, warms it for night light, clips it at the screen's peak and
/// encodes PQ.
const TO_PQ: &str = r#"
precision highp float;
uniform sampler2D tex;
uniform float alpha;
uniform float sdr_white;
uniform float peak;
uniform vec3 gains;
varying vec2 v_coords;

// Column-major.
const mat3 BT709_TO_BT2020 = mat3(
    0.6274, 0.0691, 0.0164,
    0.3293, 0.9195, 0.0880,
    0.0433, 0.0114, 0.8956);

vec3 pq(vec3 nits) {
    vec3 y = pow(clamp(nits / 10000.0, 0.0, 1.0), vec3(0.1593017578125));
    return pow((0.8359375 + 18.8515625 * y) / (1.0 + 18.6875 * y), vec3(78.84375));
}

void main() {
    vec3 c = texture2D(tex, v_coords).rgb;
    vec3 linear = sign(c) * pow(abs(c), vec3(2.2)) * sdr_white;
    vec3 wide = max(BT709_TO_BT2020 * linear, 0.0) * gains;
    gl_FragColor = vec4(pq(min(wide, vec3(peak))), 1.0);
}
"#;

/// How many frames of damage the pass remembers for the screen's swapchain.
const DAMAGE_HISTORY: usize = 8;

pub fn compile(renderer: &mut GlesRenderer) -> Result<GlesTexProgram, GlesError> {
    renderer.compile_custom_texture_shader(
        TO_PQ,
        &[
            UniformName::new("sdr_white", UniformType::_1f),
            UniformName::new("peak", UniformType::_1f),
            UniformName::new("gains", UniformType::_3f),
        ],
    )
}

/// One HDR screen's intermediate and what has changed in it.
pub struct HdrPass {
    texture: GlesTexture,
    tracker: OutputDamageTracker,
    program: GlesTexProgram,
    id: Id,
    damage: DamageBag<i32, Physical>,
    /// Physical, in the output's orientation, which is what elements are placed in.
    size: Size<i32, Physical>,
    scale: f64,
    drawn: bool,
    encoded: Option<Encode>,
    epoch: u64,
}

/// What the screen is told for the pass's uniforms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Encode {
    pub sdr_white: f32,
    pub peak: f32,
    /// Night light's, in linear light; ones when it is off.
    pub gains: [f32; 3],
}

impl HdrPass {
    pub fn new(renderer: &mut GlesRenderer, output: &Output) -> Result<HdrPass, String> {
        let mode = output.current_mode().ok_or("the output has no mode")?;
        let size = output.current_transform().transform_size(mode.size);
        let texture: GlesTexture = Offscreen::<GlesTexture>::create_buffer(
            renderer,
            Fourcc::Abgr16161616f,
            Size::from((size.w, size.h)),
        )
        .map_err(|e| format!("the HDR intermediate: {e}"))?;
        let program = compile(renderer).map_err(|e| format!("the PQ pass: {e}"))?;
        let scale = output.current_scale().fractional_scale();
        let tracker = OutputDamageTracker::new(size, scale, Transform::Normal);
        Ok(HdrPass {
            texture,
            tracker,
            program,
            id: Id::new(),
            damage: DamageBag::new(DAMAGE_HISTORY),
            size,
            scale,
            drawn: false,
            encoded: None,
            epoch: 0,
        })
    }

    /// Whether this pass still fits `output`: a new mode, rotation or scale needs a new one.
    pub fn fits(&self, output: &Output) -> bool {
        output
            .current_mode()
            .map(|m| output.current_transform().transform_size(m.size))
            == Some(self.size)
            && output.current_scale().fractional_scale() == self.scale
    }

    /// Draws `elements` into the intermediate, keeping what did not change unless the colour
    /// epoch moved.
    pub fn composite(
        &mut self,
        renderer: &mut GlesRenderer,
        elements: &[IsleElement],
        clear: [f32; 4],
        epoch: u64,
    ) -> Result<RenderElementStates, String> {
        if std::mem::replace(&mut self.epoch, epoch) != epoch {
            self.drawn = false;
        }
        let mut framebuffer = renderer.bind(&mut self.texture).map_err(|e| e.to_string())?;
        let age = usize::from(self.drawn);
        let result = self
            .tracker
            .render_output(renderer, &mut framebuffer, age, elements, clear)
            .map_err(|e| format!("{e:?}"))?;
        self.drawn = true;
        if let Some(damage) = result.damage {
            self.damage.add(damage.iter().copied());
        }
        Ok(result.states)
    }

    /// The intermediate as one element covering the screen, drawn through the PQ pass; a new
    /// white, peak or warmth redraws all of it.
    pub fn element(&mut self, encode: Encode) -> HdrElement {
        if self.encoded.replace(encode) != Some(encode) {
            self.damage.add([Rectangle::from_size(self.size)]);
        }
        HdrElement {
            texture: self.texture.clone(),
            program: self.program.clone(),
            id: self.id.clone(),
            commit: self.damage.current_commit(),
            damage: self.damage.snapshot(),
            size: self.size,
            encode,
        }
    }
}

#[derive(Clone)]
pub struct HdrElement {
    texture: GlesTexture,
    program: GlesTexProgram,
    id: Id,
    commit: CommitCounter,
    damage: DamageSnapshot<i32, Physical>,
    size: Size<i32, Physical>,
    encode: Encode,
}

impl std::fmt::Debug for HdrElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HdrElement").field("commit", &self.commit).field("encode", &self.encode).finish()
    }
}

impl Element for HdrElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.commit
    }

    fn geometry(&self, _scale: Scale<f64>) -> Rectangle<i32, Physical> {
        Rectangle::from_size(self.size)
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        let size = self.texture.size();
        Rectangle::from_size((size.w as f64, size.h as f64).into())
    }

    fn damage_since(&self, _scale: Scale<f64>, commit: Option<CommitCounter>) -> DamageSet<i32, Physical> {
        self.damage
            .damage_since(commit)
            .unwrap_or_else(|| DamageSet::from_slice(&[Rectangle::from_size(self.size)]))
    }

    fn opaque_regions(&self, _scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        OpaqueRegions::from_slice(&[Rectangle::from_size(self.size)])
    }

    fn kind(&self) -> Kind {
        Kind::Unspecified
    }

    fn location(&self, _scale: Scale<f64>) -> Point<i32, Physical> {
        Point::from((0, 0))
    }
}

impl RenderElement<GlesRenderer> for HdrElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        _cache: Option<&smithay::utils::user_data::UserDataMap>,
    ) -> Result<(), GlesError> {
        frame.render_texture_from_to(
            &self.texture,
            src,
            dst,
            damage,
            opaque_regions,
            Transform::Normal,
            1.0,
            Some(&self.program),
            &[
                Uniform::new("sdr_white", self.encode.sdr_white),
                Uniform::new("peak", self.encode.peak),
                Uniform::new("gains", (self.encode.gains[0], self.encode.gains[1], self.encode.gains[2])),
            ],
        )
    }

    fn underlying_storage(&self, _renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use smithay::{
        backend::{
            egl::{EGLContext, EGLDevice, EGLDisplay},
            renderer::{ExportMem, element::solid::SolidColorRenderElement},
        },
        output::{Mode, PhysicalProperties, Subpixel},
    };

    use super::*;

    const SIZE: i32 = 16;

    fn renderer() -> GlesRenderer {
        let device = EGLDevice::enumerate().expect("EGL devices").next().expect("an EGL device");
        let display = unsafe { EGLDisplay::new(device) }.expect("an EGL display");
        let context = EGLContext::new(&display).expect("an EGL context");
        unsafe { GlesRenderer::new(context) }.expect("a GLES renderer")
    }

    fn output() -> Output {
        let output = Output::new(
            "HDR-1".into(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "Test".into(),
                model: "HDR".into(),
                serial_number: "1".into(),
            },
        );
        let mode = Mode { size: (SIZE, SIZE).into(), refresh: 60_000 };
        output.change_current_state(Some(mode), Some(Transform::Normal), None, None);
        output
    }

    fn pq(nits: f64) -> f64 {
        let y = (nits / 10000.0).clamp(0.0, 1.0).powf(2610.0 / 16384.0);
        ((0.8359375 + 18.8515625 * y) / (1.0 + 18.6875 * y)).powf(78.84375)
    }

    fn code(nits: f64) -> i32 {
        (pq(nits) * 1023.0).round() as i32
    }

    /// The 10-bit code values of the screen's pixel for a frame filled with `colour`.
    fn screen(renderer: &mut GlesRenderer, pass: &mut HdrPass, colour: [f32; 4], encode: Encode) -> [i32; 3] {
        let solid = SolidColorRenderElement::new(
            Id::new(),
            Rectangle::from_size((SIZE, SIZE).into()),
            CommitCounter::default(),
            colour,
            Kind::Unspecified,
        );
        pass.composite(renderer, &[IsleElement::Solid(solid)], [0.0, 0.0, 0.0, 1.0], 0)
            .expect("composite");
        let element = pass.element(encode);
        let mut screen: GlesTexture =
            Offscreen::<GlesTexture>::create_buffer(renderer, Fourcc::Abgr2101010, (SIZE, SIZE).into())
                .expect("a 10-bit target");
        let mut framebuffer = renderer.bind(&mut screen).expect("bind");
        let mut tracker = OutputDamageTracker::new((SIZE, SIZE), 1.0, Transform::Normal);
        tracker
            .render_output(renderer, &mut framebuffer, 0, &[element], [0.0, 0.0, 0.0, 1.0])
            .expect("the pass");
        let mapping = renderer
            .copy_framebuffer(&framebuffer, Rectangle::from_size((SIZE, SIZE).into()), Fourcc::Abgr2101010)
            .expect("read back");
        let bytes = renderer.map_texture(&mapping).expect("map");
        let px = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        [px & 0x3ff, (px >> 10) & 0x3ff, (px >> 20) & 0x3ff].map(|v| v as i32)
    }

    fn near(got: [i32; 3], want: [i32; 3]) {
        for (g, w) in got.iter().zip(want) {
            assert!((g - w).abs() <= 2, "got {got:?}, want {want:?}");
        }
    }

    const PLAIN: Encode = Encode { sdr_white: 203.0, peak: 302.0, gains: [1.0; 3] };

    #[test]
    fn sdr_white_is_the_reference_white() {
        let mut renderer = renderer();
        let mut pass = HdrPass::new(&mut renderer, &output()).unwrap();
        let white = code(203.0);
        near(screen(&mut renderer, &mut pass, [1.0, 1.0, 1.0, 1.0], PLAIN), [white; 3]);
    }

    #[test]
    fn sdr_grey_decodes_with_gamma_2_2() {
        let mut renderer = renderer();
        let mut pass = HdrPass::new(&mut renderer, &output()).unwrap();
        let grey = code(0.5f64.powf(2.2) * 203.0);
        near(screen(&mut renderer, &mut pass, [0.5, 0.5, 0.5, 1.0], PLAIN), [grey; 3]);
    }

    #[test]
    fn values_above_white_reach_the_peak_and_stop() {
        let mut renderer = renderer();
        let mut pass = HdrPass::new(&mut renderer, &output()).unwrap();
        // 1.2 encoded is 1.49 times the reference: 302 cd/m², the peak.
        let peak = code(302.0);
        near(screen(&mut renderer, &mut pass, [1.2, 1.2, 1.2, 1.0], PLAIN), [peak; 3]);
        near(screen(&mut renderer, &mut pass, [4.0, 4.0, 4.0, 1.0], PLAIN), [peak; 3]);
    }

    #[test]
    fn sdr_red_is_widened_to_bt2020() {
        let mut renderer = renderer();
        let mut pass = HdrPass::new(&mut renderer, &output()).unwrap();
        let want = [0.6274, 0.0691, 0.0164].map(|share| code(share * 203.0));
        near(screen(&mut renderer, &mut pass, [1.0, 0.0, 0.0, 1.0], PLAIN), want);
    }

    #[test]
    fn night_light_warms_in_linear_light_and_a_new_warmth_redraws() {
        let mut renderer = renderer();
        let mut pass = HdrPass::new(&mut renderer, &output()).unwrap();
        let white = [1.0, 1.0, 1.0, 1.0];
        near(screen(&mut renderer, &mut pass, white, PLAIN), [code(203.0); 3]);
        let commit = pass.damage.current_commit();
        let warm = Encode { gains: [1.0, 0.8, 0.5], ..PLAIN };
        let got = screen(&mut renderer, &mut pass, white, warm);
        assert_ne!(pass.damage.current_commit(), commit);
        near(got, [code(203.0), code(203.0 * 0.8), code(203.0 * 0.5)]);
    }

    #[test]
    fn an_unchanged_frame_damages_nothing() {
        let mut renderer = renderer();
        let mut pass = HdrPass::new(&mut renderer, &output()).unwrap();
        screen(&mut renderer, &mut pass, [0.5, 0.5, 0.5, 1.0], PLAIN);
        let commit = pass.damage.current_commit();
        let element = pass.element(PLAIN);
        assert_eq!(element.current_commit(), commit);
        assert!(element.damage_since(Scale::from(1.0), Some(commit)).is_empty());
    }
}
