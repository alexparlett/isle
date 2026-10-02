use smithay::backend::renderer::gles::{
    GlesError, GlesPixelProgram, GlesRenderer, GlesTexProgram, UniformName, UniformType,
};

pub struct Shaders {
    pub background: GlesPixelProgram,
    pub fill: GlesPixelProgram,
    pub clip: GlesTexProgram,
}

const BACKGROUND: &str = r#"
precision highp float;
varying vec2 v_coords;
uniform vec2 size;
uniform float alpha;
#if defined(DEBUG_FLAGS)
uniform float tint;
#endif
void main() {
    vec3 a = vec3(0.10, 0.12, 0.20);
    vec3 b = vec3(0.32, 0.16, 0.30);
    vec3 c = mix(a, b, v_coords.x * 0.7 + v_coords.y * 0.3);
    gl_FragColor = vec4(c, 1.0) * alpha;
}
"#;

// Rounded-rectangle fill with a hairline border, premultiplied.
const FILL: &str = r#"
precision highp float;
varying vec2 v_coords;
uniform vec2 size;
uniform float alpha;
uniform float radius;
#if defined(DEBUG_FLAGS)
uniform float tint;
#endif
float sd_round(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
void main() {
    vec2 p = v_coords * size - size * 0.5;
    float d = sd_round(p, size * 0.5, radius);
    float inside = clamp(0.5 - d, 0.0, 1.0);
    float edge = clamp(1.0 - abs(d + 1.0), 0.0, 1.0);
    vec4 fill = vec4(0.06, 0.06, 0.08, 0.86);
    vec4 border = vec4(1.0, 1.0, 1.0, 0.14);
    vec4 c = mix(fill, border, edge * border.a);
    gl_FragColor = vec4(c.rgb * c.a, c.a) * inside * alpha;
}
"#;

// Samples the content and masks it to the shape's rounded rectangle.
const CLIP: &str = r#"
#version 100
//_DEFINES_
precision highp float;
uniform sampler2D tex;
uniform float alpha;
varying vec2 v_coords;
uniform vec2 shape_size;
uniform float radius;
uniform vec2 crop_origin;
uniform vec2 crop_extent;
#if defined(DEBUG_FLAGS)
uniform float tint;
#endif
float sd_round(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
void main() {
    vec2 local = (v_coords - crop_origin) / crop_extent * shape_size;
    float d = sd_round(local - shape_size * 0.5, shape_size * 0.5, radius);
    float inside = clamp(0.5 - d, 0.0, 1.0);
    vec4 color = texture2D(tex, v_coords);
#if defined(NO_ALPHA)
    color = vec4(color.rgb, 1.0);
#endif
    gl_FragColor = color * alpha * inside;
}
"#;

impl Shaders {
    pub fn new(renderer: &mut GlesRenderer) -> Result<Self, GlesError> {
        Ok(Self {
            background: renderer.compile_custom_pixel_shader(BACKGROUND, &[])?,
            fill: renderer.compile_custom_pixel_shader(
                FILL,
                &[UniformName::new("radius", UniformType::_1f)],
            )?,
            clip: renderer.compile_custom_texture_shader(
                CLIP,
                &[
                    UniformName::new("shape_size", UniformType::_2f),
                    UniformName::new("radius", UniformType::_1f),
                    UniformName::new("crop_origin", UniformType::_2f),
                    UniformName::new("crop_extent", UniformType::_2f),
                ],
            )?,
        })
    }
}
