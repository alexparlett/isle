//! Making one thumbnail: images, SVG and a PDF's first page decoded here, every other type by
//! the `.thumbnailer` entry that claims it, run with a deadline (`p15-files.md` choice 14).

use std::{
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use image::RgbaImage;

/// The largest picture decoded, in pixels, so one huge file cannot take the memory.
const MAX_PIXELS: u64 = 16384 * 16384;
const MAX_ALLOC: u64 = 512 * 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(10);

/// Fits `w` by `h` inside `max` across, never larger than it is.
fn fit(w: u32, h: u32, max: u32) -> (u32, u32) {
    if w <= max && h <= max {
        return (w.max(1), h.max(1));
    }
    let scale = max as f64 / w.max(h) as f64;
    (((w as f64 * scale).round() as u32).max(1), ((h as f64 * scale).round() as u32).max(1))
}

fn raster(path: &Path, pixels: u32) -> Option<RgbaImage> {
    let mut reader = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(MAX_ALLOC);
    reader.limits(limits);
    let (w, h) = reader.into_dimensions().ok()?;
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return None;
    }
    let mut reader = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_ALLOC);
    reader.limits(limits);
    let decoded = reader.decode().ok()?;
    let (tw, th) = fit(w, h, pixels);
    Some(decoded.thumbnail(tw, th).to_rgba8())
}

fn svg(path: &Path, pixels: u32) -> Option<RgbaImage> {
    let data = std::fs::read(path).ok()?;
    let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).ok()?;
    let size = tree.size();
    let (w, h) = fit(size.width().ceil() as u32, size.height().ceil() as u32, pixels);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    let transform = resvg::tiny_skia::Transform::from_scale(w as f32 / size.width(), h as f32 / size.height());
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let mut out = RgbaImage::new(w, h);
    // tiny-skia holds premultiplied colour.
    for (dst, src) in out.pixels_mut().zip(pixmap.pixels()) {
        let c = src.demultiply();
        *dst = image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]);
    }
    Some(out)
}

/// A PDF's page count.
pub fn pdf_pages(path: &Path) -> Option<usize> {
    let data = std::fs::read(path).ok()?;
    Some(hayro::hayro_syntax::Pdf::new(data).ok()?.pages().len())
}

/// A PDF's page on white, fitted to `pixels`.
pub fn pdf_page(path: &Path, page: usize, pixels: u32) -> Option<RgbaImage> {
    use hayro::{PixmapSettings, RenderCache, RenderSettings, hayro_interpret::InterpreterSettings, hayro_syntax::Pdf, render};
    let data = std::fs::read(path).ok()?;
    let pdf = Pdf::new(data).ok()?;
    let pages = pdf.pages();
    let page = pages.get(page)?;
    let (w, h) = page.render_dimensions();
    let scale = pixels as f32 / w.max(h);
    let settings = PixmapSettings {
        x_scale: scale,
        y_scale: scale,
        bg_color: hayro::vello_cpu::color::palette::css::WHITE,
    };
    let rendered = render(
        page,
        &RenderCache::new(),
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &settings,
    );
    let png = rendered.into_png().ok()?;
    Some(image::load_from_memory(&png).ok()?.to_rgba8())
}

/// A `.thumbnailer` entry: what it runs and the types it claims.
#[derive(Clone, Debug, PartialEq)]
pub struct Thumbnailer {
    pub exec: Vec<String>,
    pub try_exec: Option<String>,
    pub types: Vec<String>,
}

pub fn parse_thumbnailer(text: &str) -> Option<Thumbnailer> {
    let mut exec = None;
    let mut try_exec = None;
    let mut types = Vec::new();
    let mut in_entry = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Thumbnailer Entry]";
            continue;
        }
        let Some((key, value)) = line.split_once('=').filter(|_| in_entry) else { continue };
        match key.trim() {
            "Exec" => exec = Some(value.split_whitespace().map(str::to_owned).collect::<Vec<_>>()),
            "TryExec" => try_exec = Some(value.trim().to_owned()),
            "MimeType" => types = value.split(';').filter(|t| !t.is_empty()).map(str::to_owned).collect(),
            _ => {}
        }
    }
    Some(Thumbnailer {
        exec: exec.filter(|e| !e.is_empty())?,
        try_exec,
        types,
    })
}

fn on_path(program: &str) -> bool {
    if program.contains('/') {
        return Path::new(program).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(program).is_file()))
}

/// The system's thumbnailers whose programs are installed.
pub fn thumbnailers() -> Vec<Thumbnailer> {
    let dirs = std::env::var_os("XDG_DATA_DIRS")
        .map(|v| std::env::split_paths(&v).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["/usr/local/share".into(), "/usr/share".into()]);
    let mut found = Vec::new();
    for dir in dirs {
        let Ok(read) = std::fs::read_dir(dir.join("thumbnailers")) else { continue };
        for entry in read.flatten() {
            let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
            if let Some(t) = parse_thumbnailer(&text)
                && t.try_exec.as_deref().is_none_or(on_path)
                && on_path(&t.exec[0])
            {
                found.push(t);
            }
        }
    }
    found
}

fn run(thumbnailer: &Thumbnailer, path: &Path, pixels: u32) -> Option<RgbaImage> {
    static RUNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let run = RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let out = std::env::temp_dir().join(format!("isle-thumbnail-{}-{run}.png", std::process::id()));
    let args: Vec<String> = thumbnailer.exec[1..]
        .iter()
        .map(|a| {
            a.replace("%s", &pixels.to_string())
                .replace("%u", &crate::uri(path))
                .replace("%i", &path.to_string_lossy())
                .replace("%o", &out.to_string_lossy())
                .replace("%%", "%")
        })
        .collect();
    let mut child = Command::new(&thumbnailer.exec[0])
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < DEADLINE => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&out);
                return None;
            }
        }
    };
    let made = status.success().then(|| raster(&out, pixels)).flatten();
    let _ = std::fs::remove_file(&out);
    made
}

/// Makes a thumbnail of `path`, `pixels` across at most, or says there is none.
pub fn make(path: &Path, mime: &str, pixels: u32, thumbnailers: &[Thumbnailer]) -> Option<RgbaImage> {
    let types = isle_apps::mime::types();
    match mime {
        "image/svg+xml" | "image/svg+xml-compressed" => return svg(path, pixels),
        "application/pdf" => return pdf_page(path, 0, pixels),
        m if m.starts_with("image/") => {
            if let Some(made) = raster(path, pixels) {
                return Some(made);
            }
        }
        _ => {}
    }
    let claims = |t: &&Thumbnailer| t.types.iter().any(|claimed| types.is_a(mime, claimed));
    thumbnailers.iter().find(claims).and_then(|t| run(t, path, pixels))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One empty A4 page.
    const MINIMAL_PDF: &str = "%PDF-1.4
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >> endobj
trailer << /Root 1 0 R >>
%%EOF
";

    #[test]
    fn a_thumbnailer_entry_reads() {
        let text = "[Thumbnailer Entry]\nTryExec=ffmpegthumbnailer\nExec=ffmpegthumbnailer -i %i -o %o -s %s -f\nMimeType=video/mp4;video/webm;\n";
        let t = parse_thumbnailer(text).unwrap();
        assert_eq!(t.exec[0], "ffmpegthumbnailer");
        assert_eq!(t.types, ["video/mp4", "video/webm"]);
        assert!(parse_thumbnailer("[Other]\nExec=x\n").is_none());
    }

    #[test]
    fn pictures_fit_and_never_grow() {
        assert_eq!(fit(4000, 3000, 256), (256, 192));
        assert_eq!(fit(100, 50, 256), (100, 50));
        assert_eq!(fit(1, 5000, 128), (1, 128));
    }

    #[test]
    fn a_png_and_an_svg_are_made_here() {
        let dir = tempfile::tempdir().unwrap();
        let png = dir.path().join("big.png");
        image::RgbaImage::from_pixel(1000, 500, image::Rgba([200, 10, 10, 255])).save(&png).unwrap();
        let made = make(&png, "image/png", 256, &[]).unwrap();
        assert_eq!(made.dimensions(), (256, 128));
        let svg_path = dir.path().join("dot.svg");
        std::fs::write(&svg_path, r##"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="100"><rect width="50" height="100" fill="#00f"/></svg>"##).unwrap();
        let made = make(&svg_path, "image/svg+xml", 128, &[]).unwrap();
        assert_eq!(made.dimensions(), (50, 100));
        assert_eq!(made.get_pixel(10, 10).0, [0, 0, 255, 255]);
        let pdf = dir.path().join("page.pdf");
        std::fs::write(&pdf, MINIMAL_PDF).unwrap();
        let made = make(&pdf, "application/pdf", 128, &[]).unwrap();
        assert_eq!(made.height(), 128, "a portrait page fitted by its height");
        assert_eq!(made.get_pixel(5, 5).0, [255, 255, 255, 255], "on white");
        let junk = dir.path().join("junk.png");
        std::fs::write(&junk, b"not a png").unwrap();
        assert!(make(&junk, "image/png", 128, &[]).is_none());
    }
}
