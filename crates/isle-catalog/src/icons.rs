//! Catalogue icons as files Isle's widgets draw: the repositories' cached icons are JPEG XL
//! (`store-2026.md` §1), decoded once into PNG under a cache folder.

use std::path::{Path, PathBuf};

/// A drawable copy of `icon`: the file itself unless it is JPEG XL, else a PNG made once in
/// `cache`.
pub fn drawable(icon: &Path, cache: &Path) -> Option<PathBuf> {
    if icon.extension().is_none_or(|e| e != "jxl") {
        return icon.is_file().then(|| icon.to_path_buf());
    }
    let name = icon.file_stem()?.to_string_lossy().into_owned();
    let size = icon
        .parent()
        .and_then(Path::file_name)
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let out = cache.join(format!("{name}-{size}.png"));
    if out.is_file() {
        return Some(out);
    }
    let png = to_png(icon).ok()?;
    std::fs::create_dir_all(cache).ok()?;
    let tmp = out.with_extension("png.new");
    std::fs::write(&tmp, png).ok()?;
    std::fs::rename(&tmp, &out).ok()?;
    Some(out)
}

fn to_png(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let image = jxl_oxide::JxlImage::builder().open(path)?;
    let render = image.render_frame(0)?;
    let frame = render.image_all_channels();
    let (width, height, channels) = (frame.width(), frame.height(), frame.channels());
    let samples = frame.buf();
    let rgba: Vec<u8> = (0..width * height)
        .flat_map(|i| {
            let px = &samples[i * channels..(i + 1) * channels];
            let at = |c: usize| (px[c.min(channels - 1)].clamp(0., 1.) * 255.).round() as u8;
            let alpha = if channels == 4 || channels == 2 { at(channels - 1) } else { 255 };
            let (r, g, b) = if channels >= 3 { (at(0), at(1), at(2)) } else { (at(0), at(0), at(0)) };
            [r, g, b, alpha]
        })
        .collect();
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgba)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_jpeg_xl_icon_becomes_a_png() {
        let icon = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/icons/64x64/krita_krita.jxl"));
        let cache = std::env::temp_dir().join(format!("isle-catalog-icons-{}", std::process::id()));
        let png = super::drawable(icon, &cache).unwrap();
        let bytes = std::fs::read(&png).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let info = decoder.read_info().unwrap();
        assert_eq!((info.info().width, info.info().height), (64, 64));
        std::fs::remove_dir_all(cache).unwrap();
    }
}
