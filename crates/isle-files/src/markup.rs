//! Markup's marks over an image and Done's full-resolution render (`p15-files.md` choice 29):
//! the marks are a vector layer in the image's pixels; Done writes the image with them beside
//! the original, moves the original to the Trash and renames the new one into its place.

use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageFormat, RgbaImage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Pen,
    Highlighter,
    Line,
    Arrow,
    Rectangle,
    Ellipse,
    Text,
    Redact,
    Crop,
}

/// The tools in the row, with their glyphs and names.
pub const TOOLS: [(Tool, &str, &str); 9] = [
    (Tool::Pen, "pencil-simple", "Pen"),
    (Tool::Highlighter, "highlighter", "Highlighter"),
    (Tool::Line, "line-segment", "Line"),
    (Tool::Arrow, "arrow-up-right", "Arrow"),
    (Tool::Rectangle, "rectangle", "Rectangle"),
    (Tool::Ellipse, "circle", "Ellipse"),
    (Tool::Text, "text-t", "Text"),
    (Tool::Redact, "selection-slash", "Redact"),
    (Tool::Crop, "crop", "Crop"),
];

pub type Rgb = (u8, u8, u8);
pub type Point = (f32, f32);

#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    /// Pen and highlighter: the points drawn through.
    Stroke { tool: Tool, colour: Rgb, points: Vec<Point> },
    /// Line, arrow, rectangle and ellipse: from where the drag began to where it ended.
    Shape { tool: Tool, colour: Rgb, from: Point, to: Point },
    Text { colour: Rgb, at: Point, text: String },
    /// A box blurring what it covers.
    Redact { from: Point, to: Point },
}

/// What Markup has done to an image, in its pixels as they stand after its turns.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edits {
    pub marks: Vec<Mark>,
    pub crop: Option<(Point, Point)>,
    /// Quarter turns to the left.
    pub turns: u8,
}

impl Edits {
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty() && self.crop.is_none() && self.turns % 4 == 0
    }

    /// Turns the image a quarter to the left, the marks with it; `width` is the image's width
    /// before the turn.
    pub fn rotate_left(&mut self, width: f32) {
        let turn = |(x, y): Point| (y, width - x);
        for mark in &mut self.marks {
            match mark {
                Mark::Stroke { points, .. } => points.iter_mut().for_each(|p| *p = turn(*p)),
                Mark::Shape { from, to, .. } | Mark::Redact { from, to } => {
                    *from = turn(*from);
                    *to = turn(*to);
                }
                Mark::Text { at, .. } => *at = turn(*at),
            }
        }
        if let Some((a, b)) = &mut self.crop {
            *a = turn(*a);
            *b = turn(*b);
        }
        self.turns = (self.turns + 1) % 4;
    }
}

/// The rectangle two corners span: its top leading corner and its size.
pub fn span((ax, ay): Point, (bx, by): Point) -> (f32, f32, f32, f32) {
    (ax.min(bx), ay.min(by), (ax - bx).abs(), (ay - by).abs())
}

/// An arrow's head at `to`: its tip and the two corners `size` back along the shaft.
fn arrow_head(from: Point, to: Point, size: f32) -> (Point, Point, Point) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = (dx * dx + dy * dy).sqrt().max(0.001);
    let (ux, uy) = (dx / length, dy / length);
    let back = (to.0 - ux * size, to.1 - uy * size);
    let (px, py) = (-uy * size * 0.6, ux * size * 0.6);
    (to, (back.0 + px, back.1 + py), (back.0 - px, back.1 - py))
}

fn hex((r, g, b): Rgb) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// The marks as an SVG over a `width` by `height` image; `preview` draws redactions as frosted
/// boxes and the crop as a veil outside it, which Done renders for real instead.
pub fn svg(edits: &Edits, width: f32, height: f32, preview: bool) -> String {
    let line = (width.max(height) / 320.).max(2.);
    let mut out = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    );
    for mark in &edits.marks {
        match mark {
            Mark::Stroke { tool, colour, points } => {
                let (w, opacity) = if *tool == Tool::Highlighter { (line * 4., 0.4) } else { (line, 1.) };
                let d: Vec<String> = points.iter().enumerate().map(|(i, (x, y))| format!("{}{x:.1},{y:.1}", if i == 0 { "M" } else { "L" })).collect();
                out.push_str(&format!(
                    r#"<path d="{}" fill="none" stroke="{}" stroke-opacity="{opacity}" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round"/>"#,
                    d.join(" "),
                    hex(*colour)
                ));
            }
            Mark::Shape { tool, colour, from, to } => {
                let stroke = format!(r#"fill="none" stroke="{}" stroke-width="{line}""#, hex(*colour));
                let (x, y, w, h) = span(*from, *to);
                out.push_str(&match tool {
                    Tool::Rectangle => format!(r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" {stroke}/>"#),
                    Tool::Ellipse => format!(r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" {stroke}/>"#, x + w / 2., y + h / 2., w / 2., h / 2.),
                    Tool::Arrow => {
                        let ((ax, ay), (bx, by), (cx, cy)) = arrow_head(*from, *to, line * 4.);
                        format!(
                            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" {stroke} stroke-linecap="round"/><path d="M{ax},{ay} L{bx},{by} L{cx},{cy} z" fill="{}"/>"#,
                            from.0,
                            from.1,
                            to.0,
                            to.1,
                            hex(*colour)
                        )
                    }
                    _ => format!(r#"<line x1="{}" y1="{}" x2="{}" y2="{}" {stroke} stroke-linecap="round"/>"#, from.0, from.1, to.0, to.1),
                });
            }
            Mark::Text { colour, at, text } => {
                let size = line * 9.;
                let escaped = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
                out.push_str(&format!(
                    r#"<text x="{}" y="{}" font-family="Inter, sans-serif" font-size="{size}" font-weight="600" fill="{}">{escaped}</text>"#,
                    at.0,
                    at.1,
                    hex(*colour)
                ));
            }
            Mark::Redact { from, to } if preview => {
                let (x, y, w, h) = span(*from, *to);
                out.push_str(&format!(r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="#8e8e93" fill-opacity="0.92"/>"##));
            }
            Mark::Redact { .. } => {}
        }
    }
    if preview && let Some((a, b)) = edits.crop {
        let (x, y, w, h) = span(a, b);
        out.push_str(&format!(
            r##"<path d="M0,0 H{width} V{height} H0 z M{x},{y} V{} H{} V{y} z" fill="#000000" fill-opacity="0.5" fill-rule="evenodd"/><rect x="{x}" y="{y}" width="{w}" height="{h}" fill="none" stroke="#ffffff" stroke-width="{line}" stroke-dasharray="{} {}"/>"##,
            y + h,
            x + w,
            line * 3.,
            line * 2.
        ));
    }
    out.push_str("</svg>");
    out
}

/// The image with the edits in it, at full resolution.
pub fn render(original: &DynamicImage, edits: &Edits) -> Option<RgbaImage> {
    let mut image = original.clone();
    for _ in 0..edits.turns % 4 {
        image = image.rotate270();
    }
    let mut pixels = image.to_rgba8();
    let (width, height) = pixels.dimensions();
    for mark in &edits.marks {
        if let Mark::Redact { from, to } = mark {
            let (x, y, w, h) = span(*from, *to);
            let (x, y) = (x.max(0.) as u32, y.max(0.) as u32);
            let (w, h) = ((w as u32).min(width.saturating_sub(x)), (h as u32).min(height.saturating_sub(y)));
            if w > 0 && h > 0 {
                let region = image::imageops::crop_imm(&pixels, x, y, w, h).to_image();
                let blurred = image::imageops::blur(&region, (w.max(h) as f32 / 12.).max(8.));
                image::imageops::replace(&mut pixels, &blurred, x as i64, y as i64);
            }
        }
    }
    let layer = svg(edits, width as f32, height as f32, false);
    let mut options = resvg::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_str(&layer, &options).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    let marks = RgbaImage::from_fn(width, height, |x, y| {
        match pixmap.pixel(x, y).map(|p| p.demultiply()) {
            Some(p) => image::Rgba([p.red(), p.green(), p.blue(), p.alpha()]),
            None => image::Rgba([0, 0, 0, 0]),
        }
    });
    image::imageops::overlay(&mut pixels, &marks, 0, 0);
    if let Some((a, b)) = edits.crop {
        let (x, y, w, h) = span(a, b);
        let (x, y) = (x.max(0.) as u32, y.max(0.) as u32);
        let (w, h) = ((w as u32).min(width.saturating_sub(x)), (h as u32).min(height.saturating_sub(y)));
        if w > 0 && h > 0 {
            pixels = image::imageops::crop_imm(&pixels, x, y, w, h).to_image();
        }
    }
    Some(pixels)
}

/// Whether Markup edits this kind: PNG, JPEG and WebP.
pub fn editable(mime: &str) -> bool {
    matches!(mime, "image/png" | "image/jpeg" | "image/webp")
}

/// Writes `image` over `path` the safe way: beside it first, the original to the Trash, the
/// new one renamed into its place.
pub fn replace(path: &Path, image: &RgbaImage) -> std::io::Result<()> {
    let format = ImageFormat::from_path(path).map_err(std::io::Error::other)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let beside: PathBuf = path.with_file_name(format!(".{name}.markup"));
    let written = match format {
        ImageFormat::Jpeg => DynamicImage::ImageRgba8(image.clone()).to_rgb8().save_with_format(&beside, format),
        _ => image.save_with_format(&beside, format),
    };
    written.map_err(std::io::Error::other)?;
    replace_with(path, &beside)
}

/// Puts `beside` in `path`'s place, the original to the Trash first; `beside` goes if the
/// original cannot.
pub fn replace_with(path: &Path, beside: &Path) -> std::io::Result<()> {
    if let Err(err) = isle_fs::trash::trash(path).map(drop) {
        let _ = std::fs::remove_file(beside);
        return Err(err);
    }
    std::fs::rename(beside, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_left_turn_carries_the_marks() {
        let mut edits = Edits::default();
        edits.marks.push(Mark::Shape { tool: Tool::Line, colour: (255, 0, 0), from: (10., 20.), to: (30., 40.) });
        edits.rotate_left(100.);
        assert_eq!(edits.marks[0], Mark::Shape { tool: Tool::Line, colour: (255, 0, 0), from: (20., 90.), to: (40., 70.) });
        assert_eq!(edits.turns, 1);
    }

    #[test]
    fn done_draws_redacts_crops_and_turns_at_full_size() {
        let original = DynamicImage::ImageRgba8(RgbaImage::from_fn(200, 100, |x, _| {
            if x < 100 { image::Rgba([255, 255, 255, 255]) } else { image::Rgba([0, 0, 0, 255]) }
        }));
        let mut edits = Edits::default();
        edits.marks.push(Mark::Shape { tool: Tool::Rectangle, colour: (255, 0, 0), from: (10., 10.), to: (60., 60.) });
        let drawn = render(&original, &edits).unwrap();
        assert_eq!(drawn.dimensions(), (200, 100));
        assert_eq!(drawn.get_pixel(10, 30).0[0], 255);
        assert!(drawn.get_pixel(10, 30).0[1] < 100, "the red edge is drawn");

        edits.marks.push(Mark::Redact { from: (90., 0.), to: (110., 100.) });
        let blurred = render(&original, &edits).unwrap();
        let mid = blurred.get_pixel(100, 50).0[0];
        assert!(mid > 30 && mid < 225, "the edge is blurred: {mid}");

        edits.crop = Some(((0., 0.), (50., 40.)));
        assert_eq!(render(&original, &edits).unwrap().dimensions(), (50, 40));
        let turned = Edits { turns: 1, ..Default::default() };
        assert_eq!(render(&original, &turned).unwrap().dimensions(), (100, 200));
    }

    #[test]
    fn replacing_keeps_the_original_in_the_trash() {
        let dir = tempfile::tempdir().unwrap();
        // SAFETY: tests in this binary that read XDG_DATA_HOME all set it to their own folder.
        unsafe { std::env::set_var("XDG_DATA_HOME", dir.path().join("data")) };
        let path = dir.path().join("shot.png");
        RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 255, 255])).save(&path).unwrap();
        replace(&path, &RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))).unwrap();
        assert_eq!(image::open(&path).unwrap().width(), 2);
        assert!(dir.path().join("data/Trash/files/shot.png").exists());
    }
}
