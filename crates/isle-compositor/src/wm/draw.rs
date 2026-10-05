//! How a window is drawn: its surfaces clipped to the window's corners, and the shadow the
//! compositor gives it (`p1/surfaces/window-chrome.md`, Frame).

use smithay::{
    backend::renderer::{
        element::{
            Element, Id, Kind, RenderElement, UnderlyingStorage,
            surface::WaylandSurfaceRenderElement,
        },
        gles::{
            GlesError, GlesFrame, GlesPixelProgram, GlesRenderer, GlesTexProgram, Uniform,
            UniformName, UniformType,
        },
        utils::{CommitCounter, DamageSet, OpaqueRegions},
    },
    utils::{Buffer, Logical, Physical, Point, Rectangle, Scale, Transform},
};

/// Clips a surface to its window's geometry with rounded corners. `geo_scale` and
/// `geo_offset` take texture coordinates to the geometry's, from 0 to 1; `corners` are the
/// radii of the top-left, top-right, bottom-right and bottom-left corners in pixels.
const CLIP: &str = r#"
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
#if defined(DEBUG_FLAGS)
uniform float tint;
#endif
uniform vec2 geo_size;
uniform vec2 geo_scale;
uniform vec2 geo_offset;
uniform vec4 corners;

float corner(vec2 p, vec2 size, vec4 r) {
    vec2 c;
    float radius;
    if (p.x < r.x && p.y < r.x) {
        c = vec2(r.x, r.x);
        radius = r.x;
    } else if (p.x > size.x - r.y && p.y < r.y) {
        c = vec2(size.x - r.y, r.y);
        radius = r.y;
    } else if (p.x > size.x - r.z && p.y > size.y - r.z) {
        c = size - vec2(r.z, r.z);
        radius = r.z;
    } else if (p.x < r.w && p.y > size.y - r.w) {
        c = vec2(r.w, size.y - r.w);
        radius = r.w;
    } else {
        return 1.0;
    }
    return clamp(radius - distance(p, c) + 0.5, 0.0, 1.0);
}

void main() {
    vec4 color = texture2D(tex, v_coords);
#if defined(NO_ALPHA)
    color = vec4(color.rgb, 1.0);
#endif
    vec2 g = v_coords * geo_scale + geo_offset;
    if (g.x < 0.0 || g.y < 0.0 || g.x > 1.0 || g.y > 1.0) {
        color = vec4(0.0);
    } else {
        color *= corner(g * geo_size, geo_size, corners);
    }
    color *= alpha;
#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif
    gl_FragColor = color;
}
"#;

/// One layer of a window's shadow, in logical points across the element: a rounded box
/// blurred by `sigma`, nothing drawn where the window is.
const SHADOW: &str = r#"
precision highp float;
varying vec2 v_coords;
uniform float alpha;
#if defined(DEBUG_FLAGS)
uniform float tint;
#endif
uniform vec2 extent;
uniform vec4 shape;
uniform float shape_radius;
uniform vec4 shadow;
uniform float shadow_radius;
uniform float sigma;
uniform vec4 colour;

float rounded(vec2 p, vec4 box, float r) {
    vec2 half_size = box.zw * 0.5;
    vec2 q = abs(p - (box.xy + half_size)) - half_size + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

float erf(float x) {
    float s = sign(x);
    float a = abs(x);
    float t = 1.0 / (1.0 + 0.3275911 * a);
    float y = 1.0 - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t
        - 0.284496736) * t + 0.254829592) * t * exp(-a * a);
    return s * y;
}

void main() {
    vec2 p = v_coords * extent;
    float d = rounded(p, shadow, shadow_radius);
    float a = sigma < 0.25 ? clamp(0.5 - d, 0.0, 1.0) : 0.5 - 0.5 * erf(d / (sigma * 1.41421356));
    a *= clamp(rounded(p, shape, shape_radius) + 0.5, 0.0, 1.0);
    vec4 color = colour * a * alpha;
#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif
    gl_FragColor = color;
}
"#;

#[derive(Clone, Debug)]
pub struct Programs {
    pub clip: GlesTexProgram,
    pub shadow: GlesPixelProgram,
}

impl Programs {
    pub fn compile(renderer: &mut GlesRenderer) -> Result<Self, GlesError> {
        let clip = renderer.compile_custom_texture_shader(
            CLIP,
            &[
                UniformName::new("geo_size", UniformType::_2f),
                UniformName::new("geo_scale", UniformType::_2f),
                UniformName::new("geo_offset", UniformType::_2f),
                UniformName::new("corners", UniformType::_4f),
            ],
        )?;
        let shadow = renderer.compile_custom_pixel_shader(
            SHADOW,
            &[
                UniformName::new("extent", UniformType::_2f),
                UniformName::new("shape", UniformType::_4f),
                UniformName::new("shape_radius", UniformType::_1f),
                UniformName::new("shadow", UniformType::_4f),
                UniformName::new("shadow_radius", UniformType::_1f),
                UniformName::new("sigma", UniformType::_1f),
                UniformName::new("colour", UniformType::_4f),
            ],
        )?;
        Ok(Self { clip, shadow })
    }
}

/// A window's surface drawn through the clip program.
#[derive(Debug)]
pub struct ClippedSurface {
    inner: WaylandSurfaceRenderElement<GlesRenderer>,
    program: GlesTexProgram,
    /// The window's geometry in physical pixels, in the same space as `placed`.
    geometry: Rectangle<f64, Physical>,
    /// Where the element was made to be drawn; it may be drawn elsewhere and at another
    /// size, moved by a motion or into a blur's backdrop, and the geometry goes with it.
    placed: Rectangle<i32, Physical>,
    /// Corner radii in physical pixels: top left, top right, bottom right, bottom left.
    corners: [f32; 4],
}

impl ClippedSurface {
    /// The surface clipped, or as it is when its buffer is transformed, which the clip does
    /// not map.
    pub fn wrap(
        inner: WaylandSurfaceRenderElement<GlesRenderer>,
        program: GlesTexProgram,
        geometry: Rectangle<f64, Physical>,
        corners: [f32; 4],
        scale: f64,
    ) -> WindowPiece {
        if inner.transform() != Transform::Normal {
            return WindowPiece::Surface(inner);
        }
        let placed = inner.geometry(Scale::from(scale));
        WindowPiece::Clipped(Self {
            inner,
            program,
            geometry,
            placed,
            corners,
        })
    }
}

impl Element for ClippedSurface {
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

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.inner.damage_since(scale, commit)
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        // Rounded corners let what is behind show through.
        if self.corners.iter().any(|r| *r > 0.0) {
            OpaqueRegions::default()
        } else {
            self.inner.opaque_regions(scale)
        }
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

impl RenderElement<GlesRenderer> for ClippedSurface {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&smithay::utils::user_data::UserDataMap>,
    ) -> Result<(), GlesError> {
        // Texture coordinates v run over the whole buffer: v = (src.loc + p * k) / tex for a
        // point p of dst, k = src.size / dst.size. The program needs the geometry's fraction
        // of the same point, which is linear in v.
        let tex = match self.inner.texture() {
            smithay::backend::renderer::element::surface::WaylandSurfaceTexture::Texture(t) => {
                smithay::backend::renderer::Texture::size(t)
            }
            smithay::backend::renderer::element::surface::WaylandSurfaceTexture::SolidColor(_) => {
                return self
                    .inner
                    .draw(frame, src, dst, damage, opaque_regions, cache);
            }
        };
        let k = (
            dst.size.w as f64 / src.size.w,
            dst.size.h as f64 / src.size.h,
        );
        let ratio = (
            dst.size.w as f64 / self.placed.size.w.max(1) as f64,
            dst.size.h as f64 / self.placed.size.h.max(1) as f64,
        );
        let g = Rectangle::<f64, Physical>::new(
            (
                dst.loc.x as f64 + (self.geometry.loc.x - self.placed.loc.x as f64) * ratio.0,
                dst.loc.y as f64 + (self.geometry.loc.y - self.placed.loc.y as f64) * ratio.1,
            )
                .into(),
            (
                self.geometry.size.w * ratio.0,
                self.geometry.size.h * ratio.1,
            )
                .into(),
        );
        let corners = self.corners.map(|r| r * ratio.0 as f32);
        let scale = (tex.w as f64 * k.0 / g.size.w, tex.h as f64 * k.1 / g.size.h);
        let offset = (
            (dst.loc.x as f64 - src.loc.x * k.0 - g.loc.x) / g.size.w,
            (dst.loc.y as f64 - src.loc.y * k.1 - g.loc.y) / g.size.h,
        );
        frame.override_default_tex_program(
            self.program.clone(),
            vec![
                Uniform::new("geo_size", (g.size.w as f32, g.size.h as f32)),
                Uniform::new("geo_scale", (scale.0 as f32, scale.1 as f32)),
                Uniform::new("geo_offset", (offset.0 as f32, offset.1 as f32)),
                Uniform::new("corners", corners),
            ],
        );
        let result = self
            .inner
            .draw(frame, src, dst, damage, opaque_regions, cache);
        frame.clear_tex_program_override();
        result
    }

    fn underlying_storage(&self, renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        self.inner.underlying_storage(renderer)
    }
}

/// One layer of a window's shadow.
#[derive(Clone, Debug)]
pub struct ShadowElement {
    id: Id,
    commit: CommitCounter,
    program: GlesPixelProgram,
    /// Logical, relative to the output.
    area: Rectangle<i32, Logical>,
    uniforms: Vec<Uniform<'static>>,
    alpha: f32,
}

/// What a shadow layer looks like, to know when its uniforms change.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowShape {
    pub size: smithay::utils::Size<i32, Logical>,
    pub radius: f32,
    pub layer: isle_tokens::Shadow,
}

impl ShadowShape {
    /// How far the layer reaches past the window, on each side.
    fn reach(&self) -> i32 {
        let l = &self.layer;
        (l.blur * 1.5 + l.spread + l.x.abs().max(l.y.abs())).ceil() as i32 + 1
    }

    /// The element's box relative to the window's frame origin.
    pub fn area(&self) -> Rectangle<i32, Logical> {
        let reach = self.reach();
        Rectangle::new(
            (-reach, -reach).into(),
            (self.size.w + 2 * reach, self.size.h + 2 * reach).into(),
        )
    }

    fn uniforms(&self) -> Vec<Uniform<'static>> {
        let reach = self.reach() as f32;
        let l = &self.layer;
        let (w, h) = (self.size.w as f32, self.size.h as f32);
        let extent = (w + 2.0 * reach, h + 2.0 * reach);
        let shadow = [
            reach + l.x - l.spread,
            reach + l.y - l.spread,
            w + 2.0 * l.spread,
            h + 2.0 * l.spread,
        ];
        let c = l.colour;
        let colour = [
            c.r as f32 / 255.0 * c.a,
            c.g as f32 / 255.0 * c.a,
            c.b as f32 / 255.0 * c.a,
            c.a,
        ];
        vec![
            Uniform::new("extent", extent),
            Uniform::new("shape", [reach, reach, w, h]),
            Uniform::new("shape_radius", self.radius),
            Uniform::new("shadow", shadow),
            Uniform::new("shadow_radius", (self.radius + l.spread).max(0.0)),
            Uniform::new("sigma", l.blur / 2.0),
            Uniform::new("colour", colour),
        ]
    }
}

impl ShadowElement {
    pub fn new(program: GlesPixelProgram, shape: &ShadowShape) -> Self {
        Self {
            id: Id::new(),
            commit: CommitCounter::default(),
            program,
            area: shape.area(),
            uniforms: shape.uniforms(),
            alpha: 1.0,
        }
    }

    pub fn reshape(&mut self, shape: &ShadowShape) {
        self.area.size = shape.area().size;
        self.uniforms = shape.uniforms();
        self.commit.increment();
    }

    /// A copy placed with its frame at `frame` (logical, relative to the output), faded by
    /// `alpha`.
    pub fn at(&self, frame: Point<i32, Logical>, offset: Point<i32, Logical>, alpha: f32) -> Self {
        let mut out = self.clone();
        out.area.loc = frame + offset;
        out.alpha = alpha;
        out
    }
}

impl Element for ShadowElement {
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

    fn alpha(&self) -> f32 {
        self.alpha
    }

    fn kind(&self) -> Kind {
        Kind::Unspecified
    }
}

impl RenderElement<GlesRenderer> for ShadowElement {
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

smithay::backend::renderer::element::render_elements! {
    /// A part of a window as drawn: a surface, clipped or not, a shadow layer, or its
    /// last frame.
    pub WindowPiece<=GlesRenderer>;
    Clipped = ClippedSurface,
    Surface = WaylandSurfaceRenderElement<GlesRenderer>,
    Shadow = ShadowElement,
    Solid = smithay::backend::renderer::element::solid::SolidColorRenderElement,
    Bar = isle_shell::ShellElement,
    Snapshot = smithay::backend::renderer::element::texture::TextureRenderElement<smithay::backend::renderer::gles::GlesTexture>,
}

/// A window piece drawn with the frame `from` moved and scaled onto `to`.
pub type Placed = smithay::backend::renderer::element::utils::RelocateRenderElement<
    smithay::backend::renderer::element::utils::RescaleRenderElement<WindowPiece>,
>;

/// `piece`, drawn for a frame at `from` as if it were at `to`; both logical, relative to the
/// output.
pub fn place(
    piece: WindowPiece,
    from: Rectangle<i32, Logical>,
    to: Rectangle<f64, Logical>,
    scale: f64,
) -> Placed {
    use smithay::backend::renderer::element::utils::{
        Relocate, RelocateRenderElement, RescaleRenderElement,
    };
    let origin = from.loc.to_f64().to_physical(scale).to_i32_round();
    let factor = Scale {
        x: to.size.w / from.size.w.max(1) as f64,
        y: to.size.h / from.size.h.max(1) as f64,
    };
    let shift = (to.loc - from.loc.to_f64())
        .to_physical(scale)
        .to_i32_round();
    RelocateRenderElement::from_element(
        RescaleRenderElement::from_element(piece, origin, factor),
        shift,
        Relocate::Relative,
    )
}

impl crate::state::Isle {
    /// The pieces of window `id` front to back, placed relative to `origin` in the global
    /// space: its popups, its surfaces clipped to its corners, and its shadow.
    #[allow(clippy::too_many_arguments)]
    pub fn window_pieces(
        &self,
        renderer: &mut GlesRenderer,
        id: u64,
        origin: Point<i32, Logical>,
        scale: f64,
        shadow: bool,
        alpha: f32,
        push: &mut dyn FnMut(WindowPiece),
    ) {
        use smithay::{
            backend::renderer::element::surface::render_elements_from_surface_tree,
            desktop::PopupManager, wayland::seat::WaylandFocus,
        };
        let Some(record) = self.wm.get(id) else {
            return;
        };
        let window = &record.window;
        let Some(surface) = window.wl_surface() else {
            return;
        };
        let geometry = window.geometry();
        let rect = self.space.element_geometry(window).unwrap_or(record.rect);
        // The surface's origin is its geometry's top left less the geometry's offset in it.
        let surface_at = (rect.loc - geometry.loc - origin).to_physical_precise_round(scale);

        for (popup, offset) in PopupManager::popups_for_surface(&surface) {
            let at = (rect.loc - origin + offset - popup.geometry().loc)
                .to_physical_precise_round(scale);
            for element in
                render_elements_from_surface_tree::<_, WaylandSurfaceRenderElement<GlesRenderer>>(
                    renderer,
                    popup.wl_surface(),
                    at,
                    scale,
                    alpha,
                    Kind::Unspecified,
                )
            {
                push(WindowPiece::Surface(element));
            }
        }

        // A window with a modal dialog attached under its top is dimmed below it.
        let dimmed = self
            .wm
            .records
            .iter()
            .any(|r| r.parent == Some(id) && r.modal && self.wm.is_shown(r.id));
        if dimmed {
            let c = self.shell.theme().tokens.colour_scrim;
            let colour = [
                c.r as f32 / 255.0 * c.a,
                c.g as f32 / 255.0 * c.a,
                c.b as f32 / 255.0 * c.a,
                c.a,
            ];
            let size: smithay::utils::Size<i32, Logical> = rect.size;
            let mut scrims = self.scrims.borrow_mut();
            let buffer = scrims.entry(id).or_default();
            buffer.update(size, colour);
            let at = (rect.loc - origin).to_physical_precise_round(scale);
            push(WindowPiece::Solid(
                smithay::backend::renderer::element::solid::SolidColorRenderElement::from_buffer(
                    buffer,
                    at,
                    scale,
                    alpha,
                    Kind::Unspecified,
                ),
            ));
        }

        let frame = self.frame(id).unwrap_or(rect);
        if record.bar.is_some()
            && let Some(bar) = self.shell.attached_element(
                id,
                (frame.loc - origin).to_f64().to_physical(scale),
                alpha,
            )
        {
            push(WindowPiece::Bar(bar));
        }

        let style = self.window_style(id);
        let geo_px = Rectangle::new(
            (rect.loc - origin).to_f64().to_physical(scale),
            rect.size.to_f64().to_physical(scale),
        );
        let corners = style.corners.map(|r| (r as f64 * scale) as f32);
        let programs = self.window_programs.as_ref();
        for element in
            render_elements_from_surface_tree::<_, WaylandSurfaceRenderElement<GlesRenderer>>(
                renderer,
                &surface,
                surface_at,
                scale,
                alpha,
                Kind::Unspecified,
            )
        {
            match programs.filter(|_| style.clip) {
                Some(programs) => push(ClippedSurface::wrap(
                    element,
                    programs.clip.clone(),
                    geo_px,
                    corners,
                    scale,
                )),
                None => push(WindowPiece::Surface(element)),
            }
        }

        if !shadow || style.shadows.is_empty() {
            return;
        }
        let Some(programs) = programs else {
            return;
        };
        let mut cache = self.shadow_cache.borrow_mut();
        for (layer, spec) in style.shadows.iter().enumerate() {
            let shape = ShadowShape {
                size: frame.size,
                radius: style.corners[2],
                layer: *spec,
            };
            let entry = cache.entry((id, layer)).or_insert_with(|| {
                (
                    shape.clone(),
                    ShadowElement::new(programs.shadow.clone(), &shape),
                )
            });
            if entry.0 != shape {
                entry.1.reshape(&shape);
                entry.0 = shape.clone();
            }
            push(WindowPiece::Shadow(entry.1.at(
                frame.loc - origin,
                shape.area().loc,
                alpha,
            )));
        }
    }
}

/// How one window is drawn: which corners are rounded, and its shadow.
pub struct WindowStyle {
    pub clip: bool,
    /// Radii in logical points: top left, top right, bottom right, bottom left.
    pub corners: [f32; 4],
    pub shadows: Vec<isle_tokens::Shadow>,
}

impl crate::state::Isle {
    /// How window `id` is drawn now. A window whose geometry is inset from its surface draws
    /// its own shadow and corners, and gets neither; a full-screen one is square and bare.
    pub fn window_style(&self, id: u64) -> WindowStyle {
        let bare = WindowStyle {
            clip: false,
            corners: [0.0; 4],
            shadows: Vec::new(),
        };
        let Some(record) = self.wm.get(id) else {
            return bare;
        };
        if matches!(record.mode, super::Mode::FullScreen { .. }) {
            return bare;
        }
        let geometry = record.window.geometry();
        let inset =
            geometry.loc != Point::from((0, 0)) || record.window.bbox().size != geometry.size;
        if inset {
            return bare;
        }
        let tokens = &self.shell.theme().tokens;
        let radius = tokens.radius_window;
        let active = self
            .seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
            .and_then(|f| f.surface().cloned())
            .is_some_and(|s| record.surface().as_ref() == Some(&s));
        // Under Isle's bar the client's top corners are square; the bar rounds the frame's.
        let corners = if record.bar.is_some() {
            [0.0, 0.0, radius, radius]
        } else {
            [radius; 4]
        };
        WindowStyle {
            clip: true,
            corners,
            shadows: if active {
                tokens.shadow_window.clone()
            } else {
                tokens.shadow_window_inactive.clone()
            },
        }
    }
}
