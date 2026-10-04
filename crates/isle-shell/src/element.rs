//! A surface's texture as a render element, clipped to its corners.

use smithay::{
    backend::renderer::{
        element::{
            Element, Id, Kind, RenderElement, UnderlyingStorage,
            texture::{TextureRenderBuffer, TextureRenderElement},
        },
        gles::{GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform, UniformName,
            UniformType},
        utils::{CommitCounter, DamageSet, OpaqueRegions},
    },
    utils::{Buffer, Logical, Physical, Point, Rectangle, Scale, Size, Transform},
};

/// Clips a texture to a rounded rectangle the size of the texture.
const CLIP: &str = r#"
precision highp float;
uniform sampler2D tex;
uniform float alpha;
uniform vec2 size;
uniform float radius;
varying vec2 v_coords;

void main() {
    vec2 p = v_coords * size;
    vec2 q = abs(p - size * 0.5) - (size * 0.5 - vec2(radius));
    float d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
    float inside = clamp(0.5 - d, 0.0, 1.0);
    gl_FragColor = texture2D(tex, v_coords) * alpha * inside;
}
"#;

pub fn compile_clip(renderer: &mut GlesRenderer) -> Result<GlesTexProgram, GlesError> {
    renderer.compile_custom_texture_shader(
        CLIP,
        &[
            UniformName::new("size", UniformType::_2f),
            UniformName::new("radius", UniformType::_1f),
        ],
    )
}

/// A surface's texture at its place, faded and clipped; its id is the texture buffer's, so the
/// damage tracker sees one element across frames.
#[derive(Debug)]
pub struct SurfaceElement {
    inner: TextureRenderElement<GlesTexture>,
    texture: GlesTexture,
    clip: Option<(GlesTexProgram, f32)>,
}

impl SurfaceElement {
    /// `radius` is in physical pixels; with no program the corners stay square.
    pub fn new(
        location: Point<f64, Physical>,
        buffer: &TextureRenderBuffer<GlesTexture>,
        texture: GlesTexture,
        size: Size<i32, Logical>,
        alpha: f32,
        clip: Option<(GlesTexProgram, f32)>,
    ) -> Self {
        let inner = TextureRenderElement::from_texture_render_buffer(
            location,
            buffer,
            Some(alpha),
            None,
            Some(size),
            Kind::Unspecified,
        );
        Self {
            inner,
            texture,
            clip: clip.filter(|(_, radius)| *radius > 0.0),
        }
    }
}

impl Element for SurfaceElement {
    fn id(&self) -> &Id {
        self.inner.id()
    }

    fn current_commit(&self) -> CommitCounter {
        self.inner.current_commit()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.inner.geometry(scale)
    }

    fn transform(&self) -> Transform {
        self.inner.transform()
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        self.inner.src()
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

impl RenderElement<GlesRenderer> for SurfaceElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&smithay::utils::user_data::UserDataMap>,
    ) -> Result<(), GlesError> {
        let Some((program, radius)) = &self.clip else {
            return RenderElement::<GlesRenderer>::draw(
                &self.inner,
                frame,
                src,
                dst,
                damage,
                opaque_regions,
                cache,
            );
        };
        let size = smithay::backend::renderer::Texture::size(&self.texture);
        frame.render_texture_from_to(
            &self.texture,
            src,
            dst,
            damage,
            opaque_regions,
            self.transform(),
            self.alpha(),
            Some(program),
            &[
                Uniform::new("size", (size.w as f32, size.h as f32)),
                Uniform::new("radius", *radius),
            ],
        )
    }

    fn underlying_storage(&self, renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        self.inner.underlying_storage(renderer)
    }
}
