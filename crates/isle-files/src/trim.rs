//! Trim in Quick Look: the kept range between two handles on a strip of frames, cut exactly by
//! re-encoding (`p15-files.md` choice 30).

use std::path::{Path, PathBuf};

use bytes::Bytes;
use freya::prelude::*;
use isle_ui::{colour, text, use_theme};

use crate::window::off_thread;

/// Frames on the strip.
const FRAMES: u32 = 10;

/// `path` cut to `start`..`end` seconds and written to `out`: video through NVENC, else x264,
/// with AAC sound; audio in the codec its extension names.
pub fn encode(path: &Path, start: f64, end: f64, out: &Path, video: bool) -> std::io::Result<()> {
    let run = |codec: Option<&str>| {
        let mut command = std::process::Command::new("ffmpeg");
        command.args(["-v", "error", "-y", "-i"]).arg(path);
        command.args(["-ss", &format!("{start:.3}"), "-to", &format!("{end:.3}")]);
        match codec {
            Some(codec) => command.args(["-c:v", codec, "-c:a", "aac"]),
            None => command.arg("-vn"),
        };
        command.arg(out).status()
    };
    let codecs: &[Option<&str>] = if video { &[Some("h264_nvenc"), Some("libx264")] } else { &[None] };
    for codec in codecs {
        if run(*codec)?.success() {
            return Ok(());
        }
    }
    Err(std::io::Error::other("ffmpeg could not encode it"))
}

/// Where the trimmed file is written before it takes the original's place.
pub fn beside(path: &Path) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let (stem, ext) = isle_fs::jobs::split_ext(&name);
    path.with_file_name(format!(".{stem}.trim{ext}"))
}

/// The length in seconds and the strip's frames, from the helper.
fn strip(path: &Path) -> Option<(f64, Vec<Bytes>)> {
    let dir = std::env::temp_dir().join(format!("isle-trim-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let prefix = dir.join("frame");
    let out = std::process::Command::new(crate::thumbs::helper())
        .args(["frames".as_ref(), path.as_os_str(), FRAMES.to_string().as_ref(), "80".as_ref(), prefix.as_os_str()])
        .output()
        .ok()?;
    let length: f64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
    let frames = (0..FRAMES)
        .filter_map(|n| std::fs::read(dir.join(format!("frame-{n}.png"))).ok().map(Bytes::from))
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    Some((length, frames))
}

fn clock(seconds: f64) -> String {
    let s = seconds.max(0.).round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

#[derive(PartialEq)]
pub struct TrimView {
    pub path: PathBuf,
    pub video: bool,
    /// Trim ended: true once Done wrote the file.
    pub on_end: EventHandler<bool>,
}

impl Component for TrimView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut found = use_state(|| None::<(f64, Vec<Bytes>)>);
        // The kept range as shares of the length.
        let mut range = use_state(|| (0f64, 1f64));
        let mut dragging = use_state(|| None::<bool>);
        let mut strip_area = use_state(|| (0f32, 0f32));
        let saving = use_state(|| false);
        use_hook({
            let path = self.path.clone();
            move || {
                spawn(async move {
                    if let Some(Some(read)) = off_thread(move || strip(&path)).await {
                        found.set(Some(read));
                    }
                });
            }
        });
        let length = found.read().as_ref().map_or(0., |(l, _)| *l);
        let (start, end) = *range.read();
        let share_at = move |x: f64| {
            let (left, width) = *strip_area.peek();
            ((x as f32 - left) / width.max(1.)).clamp(0., 1.) as f64
        };
        let grab = move |e: Event<MouseEventData>| {
            let at = share_at(e.global_location.x);
            let (s, e2) = *range.peek();
            dragging.set(Some((at - s).abs() <= (at - e2).abs()));
        };
        let slide = move |e: Event<PointerEventData>| {
            let Some(first) = *dragging.peek() else { return };
            let at = share_at(e.global_location().x);
            let (s, e2) = *range.peek();
            range.set(if first { (at.min(e2 - 0.01), e2) } else { (s, at.max(s + 0.01)) });
        };
        let done = {
            let (path, video, on_end) = (self.path.clone(), self.video, self.on_end.clone());
            move |_| {
                let (s, e) = *range.peek();
                if s <= 0. && e >= 1. {
                    on_end.call(false);
                    return;
                }
                let (path, on_end, length) = (path.clone(), on_end.clone(), length);
                let mut saving = saving;
                saving.set(true);
                spawn(async move {
                    let wrote = off_thread(move || {
                        let out = beside(&path);
                        encode(&path, s * length, e * length, &out, video)?;
                        crate::markup::replace_with(&path, &out)
                    })
                    .await;
                    saving.set(false);
                    match wrote {
                        Some(Ok(())) => on_end.call(true),
                        Some(Err(err)) => tracing::warn!("Trim was not saved: {err}"),
                        None => {}
                    }
                });
            }
        };
        let frames = found.read().as_ref().map(|(_, f)| f.clone()).unwrap_or_default();
        let strip = rect()
            .width(Size::flex(1.))
            .height(Size::px(t.size_trim_strip))
            .corner_radius(t.radius_control)
            .overflow(Overflow::Clip)
            .background(colour(t.colour_fill))
            .on_sized(move |e: Event<SizedEventData>| strip_area.set((e.area.origin.x, e.area.size.width)))
            .on_mouse_down(grab)
            .on_global_pointer_move(slide)
            .on_global_pointer_up(move |_| dragging.set(None))
            .child(
                rect()
                    .expanded()
                    .horizontal()
                    .children(frames.into_iter().enumerate().map(|(n, f)| {
                        ImageViewer::new(f)
                            .key(n)
                            .width(Size::percent(100. / FRAMES as f32))
                            .height(Size::fill())
                            .aspect_ratio(AspectRatio::Max)
                            .into_element()
                    })),
            )
            .child(
                rect()
                    .position(Position::new_absolute().top(0.).left(0.))
                    .layer(Layer::Overlay)
                    .expanded()
                    .horizontal()
                    .child(rect().width(Size::percent((start * 100.) as f32)).height(Size::fill()).background(colour(t.colour_scrim)))
                    .child(
                        rect()
                            .width(Size::percent(((end - start) * 100.) as f32))
                            .height(Size::fill())
                            .border(Border::new().fill(colour(t.colour_accent)).width(3.).alignment(BorderAlignment::Inner)),
                    )
                    .child(rect().width(Size::flex(1.)).height(Size::fill()).background(colour(t.colour_scrim))),
            );
        let bar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_trim_bar))
            .padding(t.space_m)
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .background(colour(t.colour_titlebar))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth { top: 1., right: 0., bottom: 0., left: 0. })
                    .alignment(BorderAlignment::Inner),
            )
            .child(text::line(&theme, text::Style::Callout, colour(t.colour_text_2), clock((end - start) * length)))
            .child(strip)
            .child(rect().width(Size::px(t.space_l)))
            .child(isle_ui::Button::new("Revert").on_press(move |_| range.set((0., 1.))))
            .child(
                isle_ui::Button::new(if *saving.read() { "Saving…" } else { "Done" })
                    .kind(isle_ui::button::Kind::Default)
                    .enabled(!*saving.read() && length > 0.)
                    .on_press(done),
            );
        rect()
            .expanded()
            .content(Content::flex())
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .center()
                    .maybe_child(found.read().as_ref().and_then(|(_, f)| f.get(((start * FRAMES as f64) as usize).min(FRAMES as usize - 1)).cloned()).map(|f| {
                        ImageViewer::new(f).expanded().aspect_ratio(AspectRatio::Min).image_cover(ImageCover::Center)
                    })),
            )
            .child(bar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cut_is_written_beside_the_file() {
        assert_eq!(beside(Path::new("/v/Recording 1.mp4")), PathBuf::from("/v/.Recording 1.trim.mp4"));
        assert_eq!(clock(75.4), "1:15");
    }
}
