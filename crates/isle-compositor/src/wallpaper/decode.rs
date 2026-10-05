//! Pictures read, fitted to a screen and measured, on the worker.

use std::path::{Path, PathBuf};

use image::{
    DynamicImage, ImageDecoder, ImageReader, Rgba, RgbaImage,
    imageops::{self, FilterType},
};
use isle_settings::schema::Fit;

/// The extensions a folder's pictures are found by.
const PICTURES: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// A picture to fit to a screen of `size` device pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub path: PathBuf,
    pub size: (u32, u32),
    pub fit: Fit,
    /// Around a fitted or centred picture; its mean colour when unset.
    pub colour: Option<[u8; 3]>,
    pub folder: Pick,
}

/// Which of a folder's pictures to show.
#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    /// The one shown last, or the first when it is gone.
    Keep(Option<PathBuf>),
    /// The one after the one shown last, or another at random.
    Next {
        current: Option<PathBuf>,
        shuffle: bool,
    },
}

/// A screen's pixels, RGBA, top row first.
#[derive(Debug)]
pub struct Decoded {
    pub file: PathBuf,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// The scaled picture's mean colour, averaged in linear light.
    pub mean: [u8; 3],
}

/// Reads the picture `request` names, or the folder's pick, and fits it to the screen.
pub fn load(request: &Request) -> Result<Decoded, String> {
    let file = if request.path.is_dir() {
        pick(&request.path, &request.folder)?
    } else {
        request.path.clone()
    };
    let picture = read(&file)?;
    let (rgba, mean) = fit(&picture, request.size, request.fit, request.colour);
    Ok(Decoded {
        file,
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
        mean,
    })
}

fn read(file: &Path) -> Result<RgbaImage, String> {
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.display().to_string());
    if !file.exists() {
        return Err(format!("“{name}” couldn't be found."));
    }
    let unreadable = |err: &dyn std::fmt::Display| format!("“{name}” couldn't be read: {err}");
    let reader = ImageReader::open(file)
        .map_err(|e| unreadable(&e))?
        .with_guessed_format()
        .map_err(|e| unreadable(&e))?;
    if reader.format().is_none() {
        return Err(format!(
            "“{name}” is not a picture Isle can show: JPEG, PNG and WebP are."
        ));
    }
    let mut decoder = reader.into_decoder().map_err(|e| unreadable(&e))?;
    let orientation = decoder.orientation().ok();
    let mut picture = DynamicImage::from_decoder(decoder).map_err(|e| unreadable(&e))?;
    if let Some(orientation) = orientation {
        picture.apply_orientation(orientation);
    }
    Ok(picture.into_rgba8())
}

/// The folder's pictures by name, and the one `pick` chooses.
fn pick(folder: &Path, pick: &Pick) -> Result<PathBuf, String> {
    let name = folder.display();
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
        .map_err(|e| format!("“{name}” couldn't be read: {e}"))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| PICTURES.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("“{name}” holds no pictures."));
    }
    let at = |current: &Option<PathBuf>| {
        current
            .as_ref()
            .and_then(|c| files.iter().position(|f| f == c))
    };
    let index = match pick {
        Pick::Keep(current) => at(current).unwrap_or(0),
        Pick::Next {
            current,
            shuffle: false,
        } => at(current).map_or(0, |i| (i + 1) % files.len()),
        Pick::Next {
            current,
            shuffle: true,
        } => {
            let now = at(current);
            let others = files.len() - usize::from(now.is_some() && files.len() > 1);
            let mut index = random() as usize % others.max(1);
            if let Some(now) = now.filter(|_| files.len() > 1)
                && index >= now
            {
                index += 1;
            }
            index
        }
    };
    Ok(files.swap_remove(index))
}

fn random() -> u64 {
    use std::hash::{BuildHasher, RandomState};
    RandomState::new().hash_one(std::time::SystemTime::now())
}

/// `picture` fitted to `size`, and the scaled picture's mean colour.
pub fn fit(
    picture: &RgbaImage,
    size: (u32, u32),
    fit: Fit,
    colour: Option<[u8; 3]>,
) -> (RgbaImage, [u8; 3]) {
    let (w, h) = (
        picture.width().max(1) as f64,
        picture.height().max(1) as f64,
    );
    let (sw, sh) = (size.0.max(1), size.1.max(1));
    if fit == Fit::Fill {
        // Crop the source to the screen's shape about its centre, then scale once.
        let scale = (sw as f64 / w).max(sh as f64 / h);
        let (cw, ch) = (
            ((sw as f64 / scale).round() as u32).clamp(1, picture.width()),
            ((sh as f64 / scale).round() as u32).clamp(1, picture.height()),
        );
        let (cx, cy) = ((picture.width() - cw) / 2, (picture.height() - ch) / 2);
        let crop = imageops::crop_imm(picture, cx, cy, cw, ch).to_image();
        let scaled = imageops::resize(&crop, sw, sh, FilterType::Lanczos3);
        let mean = mean(&scaled);
        return (scaled, mean);
    }
    let scaled = match fit {
        Fit::Fit => {
            let scale = (sw as f64 / w).min(sh as f64 / h);
            let (fw, fh) = (
                ((w * scale).round() as u32).max(1),
                ((h * scale).round() as u32).max(1),
            );
            imageops::resize(picture, fw, fh, FilterType::Lanczos3)
        }
        _ => picture.clone(),
    };
    let mean = mean(&scaled);
    let [r, g, b] = colour.unwrap_or(mean);
    let mut canvas = RgbaImage::from_pixel(sw, sh, Rgba([r, g, b, 255]));
    let x = (sw as i64 - scaled.width() as i64) / 2;
    let y = (sh as i64 - scaled.height() as i64) / 2;
    imageops::overlay(&mut canvas, &scaled, x, y);
    (canvas, mean)
}

/// The average of `picture`'s colour in linear light, weighted by alpha, as sRGB.
pub fn mean(picture: &RgbaImage) -> [u8; 3] {
    let lut: Vec<f64> = (0..=255).map(|v| to_linear(v as f64 / 255.0)).collect();
    let (mut sum, mut weight) = ([0.0f64; 3], 0.0f64);
    for p in picture.pixels() {
        let a = p[3] as f64 / 255.0;
        for (s, &c) in sum.iter_mut().zip(&p.0[..3]) {
            *s += lut[c as usize] * a;
        }
        weight += a;
    }
    if weight == 0.0 {
        return [0, 0, 0];
    }
    sum.map(|s| (to_srgb(s / weight) * 255.0).round() as u8)
}

fn to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
    .clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Left half one colour, right half another.
    fn halves(w: u32, h: u32, left: [u8; 3], right: [u8; 3]) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, _| {
            let [r, g, b] = if x < w / 2 { left } else { right };
            Rgba([r, g, b, 255])
        })
    }

    #[test]
    fn the_mean_is_taken_in_linear_light() {
        // Black and white average to mid grey in light, which is 188 in sRGB, not 128.
        let picture = halves(4, 2, [0, 0, 0], [255, 255, 255]);
        assert_eq!(mean(&picture), [188, 188, 188]);
    }

    #[test]
    fn fill_crops_a_wide_picture_to_a_narrow_screen_about_its_centre() {
        // A 4:1 picture, red, green, green, red in quarters, on a 1:1 screen keeps only the
        // middle half.
        let picture = RgbaImage::from_fn(400, 100, |x, _| {
            if (100..300).contains(&x) {
                Rgba([0, 255, 0, 255])
            } else {
                Rgba([255, 0, 0, 255])
            }
        });
        let (out, _) = fit(&picture, (50, 50), Fit::Fill, None);
        assert_eq!((out.width(), out.height()), (50, 50));
        assert!(
            out.pixels().all(|p| p[1] > 200 && p[0] < 50),
            "only green is left"
        );
    }

    #[test]
    fn fit_draws_bars_in_the_picture_s_mean_colour() {
        let picture = RgbaImage::from_pixel(100, 100, Rgba([200, 40, 40, 255]));
        let (out, mean) = fit(&picture, (300, 100), Fit::Fit, None);
        assert_eq!(mean, [200, 40, 40]);
        assert_eq!(*out.get_pixel(10, 50), Rgba([200, 40, 40, 255]));
        assert_eq!(*out.get_pixel(150, 50), Rgba([200, 40, 40, 255]));
    }

    #[test]
    fn centre_draws_one_pixel_to_one_on_the_colour() {
        let picture = RgbaImage::from_pixel(10, 10, Rgba([255, 255, 255, 255]));
        let (out, _) = fit(&picture, (30, 30), Fit::Centre, Some([0, 0, 255]));
        assert_eq!(*out.get_pixel(15, 15), Rgba([255, 255, 255, 255]));
        assert_eq!(*out.get_pixel(9, 15), Rgba([0, 0, 255, 255]));
        assert_eq!(*out.get_pixel(10, 15), Rgba([255, 255, 255, 255]));
        assert_eq!(*out.get_pixel(20, 15), Rgba([0, 0, 255, 255]));
    }

    #[test]
    fn a_folder_moves_on_by_name_and_wraps() {
        let dir = std::env::temp_dir().join(format!("isle-wallpaper-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["b.png", "a.jpg", "notes.txt", "c.webp"] {
            std::fs::write(dir.join(name), b"").unwrap();
        }
        let next = |current: Option<&str>| {
            pick(
                &dir,
                &Pick::Next {
                    current: current.map(|c| dir.join(c)),
                    shuffle: false,
                },
            )
            .unwrap()
        };
        assert_eq!(next(None), dir.join("a.jpg"));
        assert_eq!(next(Some("a.jpg")), dir.join("b.png"));
        assert_eq!(next(Some("c.webp")), dir.join("a.jpg"));
        assert_eq!(
            pick(&dir, &Pick::Keep(Some(dir.join("gone.png")))).unwrap(),
            dir.join("a.jpg")
        );
        for _ in 0..20 {
            let shuffled = pick(
                &dir,
                &Pick::Next {
                    current: Some(dir.join("b.png")),
                    shuffle: true,
                },
            )
            .unwrap();
            assert_ne!(shuffled, dir.join("b.png"));
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_file_says_so_by_name() {
        let err = read(Path::new("/nonexistent/sunset.jpg")).unwrap_err();
        assert_eq!(err, "“sunset.jpg” couldn't be found.");
    }
}
