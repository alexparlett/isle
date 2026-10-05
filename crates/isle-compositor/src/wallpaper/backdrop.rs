//! Frost's drawn backdrop: a shader over the output from the neutral tokens, which needs no
//! picture and so can be the session's first frame (`p1/surfaces/wallpaper.md`).

use isle_tokens::{Colour, Theme};
use smithay::{
    backend::renderer::{
        element::{Element, Id, Kind, RenderElement, UnderlyingStorage},
        gles::{
            GlesError, GlesFrame, GlesPixelProgram, GlesRenderer, Uniform, UniformName, UniformType,
        },
        utils::{CommitCounter, OpaqueRegions},
    },
    utils::{Buffer, Logical, Physical, Rectangle, Scale, Transform},
};

/// A grey floor, a soft light from above the top centre, darker corners, and grain against
/// banding. `extent` is the output's logical size; colours are sRGB, the falloff's with its
/// alpha.
const BACKDROP: &str = r#"
precision highp float;
varying vec2 v_coords;
uniform float alpha;
#if defined(DEBUG_FLAGS)
uniform float tint;
#endif
uniform vec2 extent;
uniform vec3 base;
uniform vec3 light;
uniform vec4 falloff;
uniform float grain;

float hash(vec2 p) {
    p = fract(p * vec2(443.897, 441.423));
    p += dot(p, p.yx + 19.19);
    return fract((p.x + p.y) * p.x);
}

void main() {
    vec2 p = v_coords * extent;
    // The light: an ellipse centred 10% above the top edge, 70% of the width and 95% of
    // the height across, eased to nothing at its edge.
    vec2 d = (p - vec2(0.5 * extent.x, -0.1 * extent.y)) / vec2(0.7 * extent.x, 0.95 * extent.y);
    vec3 c = mix(base, light, 1.0 - smoothstep(0.0, 1.0, length(d)));
    // The falloff: nothing to two thirds of the way from the centre to a corner.
    vec2 q = (p - 0.5 * extent) / (0.5 * extent);
    float f = smoothstep(2.0 / 3.0, 1.0, length(q) / 1.41421356);
    c = mix(c, falloff.rgb, f * falloff.a);
    c += (hash(gl_FragCoord.xy) - 0.5) * grain;
    vec4 color = vec4(c, 1.0) * alpha;
#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif
    gl_FragColor = color;
}
"#;

pub fn compile(renderer: &mut GlesRenderer) -> Result<GlesPixelProgram, GlesError> {
    renderer.compile_custom_pixel_shader(
        BACKDROP,
        &[
            UniformName::new("extent", UniformType::_2f),
            UniformName::new("base", UniformType::_3f),
            UniformName::new("light", UniformType::_3f),
            UniformName::new("falloff", UniformType::_4f),
            UniformName::new("grain", UniformType::_1f),
        ],
    )
}

/// The colours the backdrop is drawn from, which change with the appearance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    base: Colour,
    light: Colour,
    falloff: Colour,
    grain: f32,
}

impl Look {
    pub fn of(theme: &Theme) -> Self {
        let t = &theme.tokens;
        let dark = theme.appearance.mode == isle_tokens::Mode::Dark;
        Self {
            base: t.colour_window_sunken,
            light: if dark {
                t.colour_window_raised
            } else {
                t.colour_view
            },
            falloff: if dark {
                t.colour_scrim
            } else {
                t.colour_fill_strong
            },
            grain: t.material_noise_opacity,
        }
    }
}

fn rgb(c: Colour) -> [f32; 3] {
    [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0]
}

#[derive(Clone, Debug)]
pub struct BackdropElement {
    id: Id,
    commit: CommitCounter,
    program: GlesPixelProgram,
    /// Logical, relative to the output.
    area: Rectangle<i32, Logical>,
    uniforms: Vec<Uniform<'static>>,
    alpha: f32,
}

impl BackdropElement {
    pub fn new(
        program: GlesPixelProgram,
        size: smithay::utils::Size<i32, Logical>,
        look: Look,
    ) -> Self {
        let [r, g, b] = rgb(look.falloff);
        Self {
            id: Id::new(),
            commit: CommitCounter::default(),
            program,
            area: Rectangle::from_size(size),
            uniforms: vec![
                Uniform::new("extent", (size.w as f32, size.h as f32)),
                Uniform::new("base", rgb(look.base)),
                Uniform::new("light", rgb(look.light)),
                Uniform::new("falloff", [r, g, b, look.falloff.a]),
                Uniform::new("grain", look.grain),
            ],
            alpha: 1.0,
        }
    }

    pub fn with_alpha(&self, alpha: f32) -> Self {
        let mut out = self.clone();
        out.alpha = alpha;
        out
    }
}

impl Element for BackdropElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.commit
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        Rectangle::from_size(self.area.size.to_f64().to_buffer(1.0, Transform::Normal))
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.area.to_physical_precise_round(scale)
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        if self.alpha >= 1.0 {
            OpaqueRegions::from_slice(&[Rectangle::from_size(self.geometry(scale).size)])
        } else {
            OpaqueRegions::default()
        }
    }

    fn alpha(&self) -> f32 {
        self.alpha
    }

    fn kind(&self) -> Kind {
        Kind::Unspecified
    }
}

impl RenderElement<GlesRenderer> for BackdropElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        _opaque_regions: &[Rectangle<i32, Physical>],
        _cache: Option<&smithay::utils::user_data::UserDataMap>,
    ) -> Result<(), GlesError> {
        frame.render_pixel_shader_to(
            &self.program,
            src,
            dst,
            self.area.size.to_buffer(1, Transform::Normal),
            Some(damage),
            self.alpha,
            &self.uniforms,
        )
    }

    fn underlying_storage(&self, _renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        None
    }
}
