//! Glass: what lies behind a surface's glass shapes, blurred, saturated and tinted, and the tint
//! raised until secondary text holds its contrast over it (DESIGN.md, Materials and Contrast).
//!
//! The backdrop is drawn again into a texture of its own through a damage tracker, so the blur
//! runs only when something behind the glass changed; then dual Kawase passes down to an eighth
//! and back up to half size.

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            Bind, Color32F, ExportMem, Frame, Offscreen, Renderer, Texture,
            damage::OutputDamageTracker,
            element::{
                Element, Id, Kind, RenderElement, UnderlyingStorage,
                utils::{Relocate, RelocateRenderElement},
            },
            gles::{
                GlesError, GlesFrame, GlesMapping, GlesRenderer, GlesTexProgram, GlesTexture, Uniform,
                UniformName, UniformType,
            },
            utils::{CommitCounter, DamageSet, OpaqueRegions},
        },
    },
    utils::{Buffer, Logical, Physical, Point, Rectangle, Scale, Size, Transform},
};
use tracing::warn;

use isle_tokens::{Colour, Theme};

/// Halvings before the blur comes back up, and the sample offset, for a blur of standard
/// deviation `sigma` pixels: the passes reach about `2^levels * offset / 2`.
fn passes(sigma: f64) -> (usize, f32) {
    let levels = (sigma / 1.5).log2().ceil().clamp(1.0, 6.0) as usize;
    let offset = (sigma * 2.0 / (1usize << levels) as f64) as f32;
    (levels, offset.max(1.0))
}

const DOWN: &str = r#"
precision highp float;
uniform sampler2D tex;
uniform float alpha;
uniform vec2 halfpixel;
uniform float offset;
varying vec2 v_coords;

void main() {
    vec2 d = halfpixel * offset;
    vec4 sum = texture2D(tex, v_coords) * 4.0;
    sum += texture2D(tex, v_coords - d);
    sum += texture2D(tex, v_coords + d);
    sum += texture2D(tex, v_coords + vec2(d.x, -d.y));
    sum += texture2D(tex, v_coords - vec2(d.x, -d.y));
    gl_FragColor = sum / 8.0 * alpha;
}
"#;

const UP: &str = r#"
precision highp float;
uniform sampler2D tex;
uniform float alpha;
uniform vec2 halfpixel;
uniform float offset;
varying vec2 v_coords;

void main() {
    vec2 d = halfpixel * offset;
    vec4 sum = texture2D(tex, v_coords + vec2(-d.x * 2.0, 0.0));
    sum += texture2D(tex, v_coords + vec2(-d.x, d.y)) * 2.0;
    sum += texture2D(tex, v_coords + vec2(0.0, d.y * 2.0));
    sum += texture2D(tex, v_coords + vec2(d.x, d.y)) * 2.0;
    sum += texture2D(tex, v_coords + vec2(d.x * 2.0, 0.0));
    sum += texture2D(tex, v_coords + vec2(d.x, -d.y)) * 2.0;
    sum += texture2D(tex, v_coords + vec2(0.0, -d.y * 2.0));
    sum += texture2D(tex, v_coords + vec2(-d.x, -d.y)) * 2.0;
    gl_FragColor = sum / 12.0 * alpha;
}
"#;

/// The blurred backdrop saturated, the tint over it, clipped to the shape's rounded rectangle.
const GLASS: &str = r#"
precision highp float;
uniform sampler2D tex;
uniform float alpha;
uniform vec2 src_origin;
uniform vec2 src_size;
uniform vec2 size;
uniform float radius;
uniform float saturation;
uniform vec4 tint;
varying vec2 v_coords;

void main() {
    vec3 c = texture2D(tex, v_coords).rgb;
    float luma = dot(c, vec3(0.2126, 0.7152, 0.0722));
    c = clamp(mix(vec3(luma), c, saturation), 0.0, 1.0);
    c = mix(c, tint.rgb, tint.a);
    vec2 p = (v_coords - src_origin) / src_size * size;
    vec2 q = abs(p - size * 0.5) - (size * 0.5 - vec2(radius));
    float d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
    float inside = clamp(0.5 - d, 0.0, 1.0);
    gl_FragColor = vec4(c, 1.0) * inside * alpha;
}
"#;

pub struct Programs {
    down: GlesTexProgram,
    up: GlesTexProgram,
    glass: GlesTexProgram,
}

impl Programs {
    pub fn glass_program(&self) -> &GlesTexProgram {
        &self.glass
    }

    pub fn compile(renderer: &mut GlesRenderer) -> Result<Self, GlesError> {
        let pass = [
            UniformName::new("halfpixel", UniformType::_2f),
            UniformName::new("offset", UniformType::_1f),
        ];
        Ok(Self {
            down: renderer.compile_custom_texture_shader(DOWN, &pass)?,
            up: renderer.compile_custom_texture_shader(UP, &pass)?,
            glass: renderer.compile_custom_texture_shader(
                GLASS,
                &[
                    UniformName::new("src_origin", UniformType::_2f),
                    UniformName::new("src_size", UniformType::_2f),
                    UniformName::new("size", UniformType::_2f),
                    UniformName::new("radius", UniformType::_1f),
                    UniformName::new("saturation", UniformType::_1f),
                    UniformName::new("tint", UniformType::_4f),
                ],
            )?,
        })
    }
}

/// One glass shape's blur, kept between frames.
pub struct Blur {
    /// The backdrop's box on the output, in physical pixels: the shape and a margin around it.
    pub region: Rectangle<i32, Physical>,
    tracker: OutputDamageTracker,
    backdrop: GlesTexture,
    /// Half size, then each half again; the half is the result.
    levels: Vec<GlesTexture>,
    offset: f32,
    probe: GlesTexture,
    pending: Option<GlesMapping>,
    /// How far the tint has moved toward `glass-tinted` to hold the floor, 0 to 1.
    pub floor: f32,
    /// Times the blur ran.
    pub runs: u64,
    pub used: bool,
    id: Id,
    commit: CommitCounter,
}

impl Blur {
    /// `sigma` is the blur's standard deviation in physical pixels.
    pub fn new(
        renderer: &mut GlesRenderer,
        region: Rectangle<i32, Physical>,
        scale: f64,
        sigma: f64,
    ) -> Result<Self, GlesError> {
        let (count, offset) = passes(sigma);
        let size = region.size;
        let make = |r: &mut GlesRenderer, s: Size<i32, Physical>| -> Result<GlesTexture, GlesError> {
            Offscreen::<GlesTexture>::create_buffer(
                r,
                Fourcc::Abgr8888,
                (s.w.max(1), s.h.max(1)).into(),
            )
        };
        let backdrop = make(renderer, size)?;
        let mut levels = Vec::new();
        for level in 1..=count {
            let s = Size::from(((size.w >> level).max(1), (size.h >> level).max(1)));
            levels.push(make(renderer, s)?);
        }
        let probe = make(renderer, (4, 4).into())?;
        Ok(Self {
            region,
            tracker: OutputDamageTracker::new(size, scale, Transform::Normal),
            backdrop,
            levels,
            offset,
            probe,
            pending: None,
            floor: 0.0,
            runs: 0,
            used: true,
            id: Id::new(),
            commit: CommitCounter::default(),
        })
    }

    /// Draws `below`, the elements behind the glass relative to the output, into the backdrop,
    /// and blurs it again if any of it changed. Reads the last run's luminance back first.
    pub fn update<E: RenderElement<GlesRenderer>>(
        &mut self,
        renderer: &mut GlesRenderer,
        programs: &Programs,
        below: Vec<E>,
        theme: &Theme,
    ) -> Result<(), GlesError> {
        self.used = true;
        // Solid glass hides whatever is behind it, so there is nothing to blur.
        if theme.appearance.reduce_transparency {
            self.pending = None;
            if self.floor != 0.0 {
                self.floor = 0.0;
                self.commit.increment();
            }
            return Ok(());
        }
        if let Some(mapping) = self.pending.take() {
            match renderer.map_texture(&mapping) {
                Ok(pixels) => {
                    let floor = floor(pixels, theme);
                    if (floor - self.floor).abs() > 1e-3 {
                        self.floor = floor;
                        self.commit.increment();
                    }
                }
                Err(err) => warn!("glass luminance read back failed: {err}"),
            }
        }

        let offset = Point::<i32, Physical>::from((-self.region.loc.x, -self.region.loc.y));
        let below: Vec<_> = below
            .into_iter()
            .map(|e| RelocateRenderElement::from_element(e, offset, Relocate::Relative))
            .collect();
        let damaged = {
            let mut fb = renderer.bind(&mut self.backdrop)?;
            let result = self
                .tracker
                .render_output(renderer, &mut fb, 1, &below, Color32F::TRANSPARENT)
                .map_err(|e| match e {
                    smithay::backend::renderer::damage::Error::Rendering(e) => e,
                    smithay::backend::renderer::damage::Error::OutputNoMode(_) => {
                        GlesError::UnknownSize
                    }
                })?;
            result.damage.is_some()
        };
        if !damaged && self.runs > 0 {
            return Ok(());
        }
        self.runs += 1;
        self.commit.increment();

        let offset = self.offset;
        let pass = |renderer: &mut GlesRenderer,
                    src: &GlesTexture,
                    dst: &mut GlesTexture,
                    program: &GlesTexProgram|
         -> Result<(), GlesError> {
            let src_size = src.size();
            let dst_size = dst.size();
            let mut fb = renderer.bind(dst)?;
            let dst_rect = Rectangle::from_size((dst_size.w, dst_size.h).into());
            let mut frame = renderer.render(&mut fb, dst_rect.size, Transform::Normal)?;
            frame.clear(Color32F::TRANSPARENT, &[dst_rect])?;
            frame.render_texture_from_to(
                src,
                Rectangle::from_size(src_size.to_f64()),
                dst_rect,
                &[dst_rect],
                &[],
                Transform::Normal,
                1.0,
                Some(program),
                &[
                    Uniform::new(
                        "halfpixel",
                        (0.5 / src_size.w.max(1) as f32, 0.5 / src_size.h.max(1) as f32),
                    ),
                    Uniform::new("offset", offset),
                ],
            )?;
            // One context: what samples this texture later is ordered after it.
            let _ = frame.finish()?;
            Ok(())
        };
        let backdrop = self.backdrop.clone();
        let count = self.levels.len();
        pass(renderer, &backdrop, &mut self.levels[0], &programs.down)?;
        for i in 1..count {
            let src = self.levels[i - 1].clone();
            pass(renderer, &src, &mut self.levels[i], &programs.down)?;
        }
        for i in (1..count).rev() {
            let src = self.levels[i].clone();
            pass(renderer, &src, &mut self.levels[i - 1], &programs.up)?;
        }

        // A four by four reduction of the blur, read back on the next update.
        let smallest = self.levels[count - 1].clone();
        {
            let mut fb = renderer.bind(&mut self.probe)?;
            let rect = Rectangle::from_size((4, 4).into());
            let mut frame = renderer.render(&mut fb, rect.size, Transform::Normal)?;
            frame.clear(Color32F::TRANSPARENT, &[rect])?;
            frame.render_texture_from_to(
                &smallest,
                Rectangle::from_size(smallest.size().to_f64()),
                rect,
                &[rect],
                &[],
                Transform::Normal,
                1.0,
                None,
                &[],
            )?;
            let _ = frame.finish()?;
        }
        let fb = renderer.bind(&mut self.probe)?;
        self.pending = Some(renderer.copy_framebuffer(
            &fb,
            Rectangle::from_size((4, 4).into()),
            Fourcc::Abgr8888,
        )?);
        Ok(())
    }

    pub fn result(&self) -> &GlesTexture {
        &self.levels[0]
    }
}

/// The share of the way from `material-glass` to `glass-tinted` the tint must go so `text-2`
/// holds 4.5:1 over the glass, over the worst of the sixteen cells read back.
fn floor(pixels: &[u8], theme: &Theme) -> f32 {
    let t = &theme.tokens;
    if theme.appearance.reduce_transparency || theme.appearance.increase_contrast {
        return 0.0;
    }
    let saturation = t.material_saturation;
    let mut worst = 0.0f32;
    for cell in pixels.chunks_exact(4) {
        let a = cell[3] as f32 / 255.0;
        if a <= 0.0 {
            continue;
        }
        // Premultiplied in the texture.
        let rgb = [0, 1, 2].map(|i| (cell[i] as f32 / 255.0 / a).min(1.0));
        let luma = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
        let backdrop = rgb.map(|c| (luma + (c - luma) * saturation).clamp(0.0, 1.0));
        let holds = |share: f32| {
            let tint = t.material_glass.mix(t.material_glass_tinted, share);
            let glass = composite(tint, backdrop);
            contrast(composite(t.colour_text_2, glass), glass) >= 4.5
        };
        if holds(0.0) {
            continue;
        }
        let (mut lo, mut hi) = (0.0f32, 1.0f32);
        for _ in 0..12 {
            let mid = (lo + hi) / 2.0;
            if holds(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        worst = worst.max(hi);
    }
    worst
}

/// `top` over an opaque `below`, both in 0..1 sRGB.
fn composite(top: Colour, below: [f32; 3]) -> [f32; 3] {
    let channel = |t: u8, b: f32| t as f32 / 255.0 * top.a + b * (1.0 - top.a);
    [
        channel(top.r, below[0]),
        channel(top.g, below[1]),
        channel(top.b, below[2]),
    ]
}

fn luminance(c: [f32; 3]) -> f32 {
    let linear = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c[0]) + 0.7152 * linear(c[1]) + 0.0722 * linear(c[2])
}

fn contrast(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// A glass shape drawn from its blur, with an id of its own kept across frames.
#[derive(Debug)]
pub struct GlassElement {
    id: Id,
    commit: CommitCounter,
    texture: GlesTexture,
    /// In the result texture's pixels.
    src: Rectangle<f64, Buffer>,
    /// On the output.
    dst: Rectangle<i32, Physical>,
    program: GlesTexProgram,
    radius: f32,
    saturation: f32,
    tint: Colour,
    alpha: f32,
}

impl GlassElement {
    pub fn new(
        blur: &Blur,
        shape: Rectangle<i32, Physical>,
        program: GlesTexProgram,
        radius: f32,
        theme: &Theme,
        alpha: f32,
        clear: bool,
        faded: f32,
    ) -> Self {
        let t = &theme.tokens;
        // Glass without text keeps the Glass setting's tint; the floor raises the rest only as
        // far as its text needs (B58).
        let tint = if clear {
            t.material_glass
        } else {
            t.material_glass.mix(t.material_glass_tinted, blur.floor)
        };
        let tint = isle_tokens::Colour {
            a: tint.a * (1.0 - faded),
            ..tint
        };
        let local = Rectangle::<f64, Buffer>::new(
            (
                (shape.loc.x - blur.region.loc.x) as f64 / 2.0,
                (shape.loc.y - blur.region.loc.y) as f64 / 2.0,
            )
                .into(),
            (shape.size.w as f64 / 2.0, shape.size.h as f64 / 2.0).into(),
        );
        Self {
            id: blur.id.clone(),
            commit: blur.commit,
            texture: blur.result().clone(),
            src: local,
            dst: shape,
            program,
            radius,
            saturation: t.material_saturation,
            tint,
            alpha,
        }
    }
}

impl Element for GlassElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.commit
    }

    fn geometry(&self, _scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.dst
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        self.src
    }

    fn transform(&self) -> Transform {
        Transform::Normal
    }

    fn damage_since(&self, scale: Scale<f64>, commit: Option<CommitCounter>) -> DamageSet<i32, Physical> {
        if commit == Some(self.commit) {
            DamageSet::default()
        } else {
            DamageSet::from_slice(&[Rectangle::from_size(self.geometry(scale).size)])
        }
    }

    fn opaque_regions(&self, _scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        OpaqueRegions::default()
    }

    fn alpha(&self) -> f32 {
        self.alpha
    }

    fn kind(&self) -> Kind {
        Kind::Unspecified
    }
}

impl RenderElement<GlesRenderer> for GlassElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        _cache: Option<&smithay::utils::user_data::UserDataMap>,
    ) -> Result<(), GlesError> {
        let size = self.texture.size().to_f64();
        frame.render_texture_from_to(
            &self.texture,
            src,
            dst,
            damage,
            opaque_regions,
            Transform::Normal,
            self.alpha,
            Some(&self.program),
            &[
                Uniform::new(
                    "src_origin",
                    ((self.src.loc.x / size.w) as f32, (self.src.loc.y / size.h) as f32),
                ),
                Uniform::new(
                    "src_size",
                    ((self.src.size.w / size.w) as f32, (self.src.size.h / size.h) as f32),
                ),
                Uniform::new("size", (self.dst.size.w as f32, self.dst.size.h as f32)),
                Uniform::new("radius", self.radius),
                Uniform::new("saturation", self.saturation),
                Uniform::new(
                    "tint",
                    (
                        self.tint.r as f32 / 255.0,
                        self.tint.g as f32 / 255.0,
                        self.tint.b as f32 / 255.0,
                        self.tint.a,
                    ),
                ),
            ],
        )
    }

    fn underlying_storage(&self, _renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        None
    }
}

/// The physical box of a logical rectangle on an output of `scale`.
pub fn physical(rect: Rectangle<f64, Logical>, scale: f64) -> Rectangle<i32, Physical> {
    rect.to_physical(scale).to_i32_round()
}
