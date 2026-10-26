//! What Quick Look shows of a file, each kind read off the window's thread
//! (`p15-files.md` choice 28): images, text and code highlighted, PDF pages from the helper,
//! audio and video, archives' listings, and icon and facts for the rest.

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use bytes::Bytes;
use freya::prelude::*;
use isle_ui::{Glyph, colour, text, use_theme};
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme, ThemeSet},
    parsing::SyntaxSet,
};

use crate::window::off_thread;

/// Text and code are read this far.
const TEXT_LIMIT: usize = 256 * 1024;
/// Images larger than this are left to their icon.
const IMAGE_LIMIT: u64 = 128 * 1024 * 1024;

/// A highlighted line: its runs of colour and text.
pub type Line = Vec<((u8, u8, u8), String)>;

/// How a file is shown.
#[derive(Clone, Debug, PartialEq)]
pub enum Shown {
    Image(Bytes),
    Text(Vec<Line>),
    Pdf(usize),
    /// Audio, or video when true.
    Media(bool),
    Listing(Vec<String>),
    /// Its icon and facts.
    Facts,
}

fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme(dark: bool) -> &'static Theme {
    static THEMES: OnceLock<ThemeSet> = OnceLock::new();
    let set = THEMES.get_or_init(ThemeSet::load_defaults);
    &set.themes[if dark { "base16-ocean.dark" } else { "InspiredGitHub" }]
}

/// The file's text, highlighted by its name or its first line.
pub fn highlight(path: &Path, text: &str, dark: bool) -> Vec<Line> {
    let set = syntaxes();
    let syntax = path
        .extension()
        .and_then(|e| set.find_syntax_by_extension(&e.to_string_lossy()))
        .or_else(|| set.find_syntax_by_first_line(text.lines().next().unwrap_or("")))
        .unwrap_or_else(|| set.find_syntax_plain_text());
    let mut lines = HighlightLines::new(syntax, theme(dark));
    syntect::util::LinesWithEndings::from(text)
        .map(|line| match lines.highlight_line(line, set) {
            Ok(runs) => runs
                .into_iter()
                .map(|(style, run)| {
                    let c = style.foreground;
                    ((c.r, c.g, c.b), run.trim_end_matches('\n').to_owned())
                })
                .collect(),
            Err(_) => vec![((128, 128, 128), line.trim_end_matches('\n').to_owned())],
        })
        .collect()
}

fn is_archive(mime: &str) -> bool {
    ["zip", "x-tar", "compressed-tar", "x-7z", "x-rar", "x-bzip", "x-xz", "zstd", "gzip"].iter().any(|k| mime.contains(k))
}

/// Reads what Quick Look shows of `path`; blocks on the file.
pub fn read(path: &Path, dark: bool) -> Shown {
    if path.is_dir() {
        return Shown::Facts;
    }
    let mime = isle_apps::mimeapps::kind_of(path);
    let types = isle_apps::mime::types();
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if mime == "image/svg+xml" {
        return std::fs::read(path).ok().and_then(|b| svg_png(&b)).map(Shown::Image).unwrap_or(Shown::Facts);
    }
    if mime.starts_with("image/") && size <= IMAGE_LIMIT {
        return std::fs::read(path).map(|b| Shown::Image(Bytes::from(b))).unwrap_or(Shown::Facts);
    }
    if mime == "application/pdf" {
        return pdf_pages(path).map(Shown::Pdf).unwrap_or(Shown::Facts);
    }
    if mime.starts_with("video/") || mime.starts_with("audio/") {
        return Shown::Media(mime.starts_with("video/"));
    }
    if is_archive(&mime) {
        let listed = std::process::Command::new("bsdtar").arg("-tf").arg(path).stderr(std::process::Stdio::null()).output();
        if let Some(out) = listed.ok().filter(|o| o.status.success()) {
            return Shown::Listing(String::from_utf8_lossy(&out.stdout).lines().take(5000).map(str::to_owned).collect());
        }
        return Shown::Facts;
    }
    if mime.starts_with("text/") || types.is_a(&mime, isle_apps::mime::PLAIN) {
        use std::io::Read;
        let mut head = Vec::with_capacity(TEXT_LIMIT);
        if std::fs::File::open(path).and_then(|f| f.take(TEXT_LIMIT as u64).read_to_end(&mut head)).is_ok() {
            let text = String::from_utf8_lossy(&head);
            return Shown::Text(highlight(path, &text, dark));
        }
    }
    Shown::Facts
}

/// An SVG drawn at `SVG_PIXELS` along its longer side, as PNG, so that it scales and centres
/// as any image does.
fn svg_png(data: &[u8]) -> Option<Bytes> {
    let tree = resvg::usvg::Tree::from_data(data, &resvg::usvg::Options::default()).ok()?;
    let size = tree.size();
    let scale = SVG_PIXELS / size.width().max(size.height());
    let (w, h) = ((size.width() * scale).round().max(1.) as u32, (size.height() * scale).round().max(1.) as u32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap.encode_png().ok().map(Bytes::from)
}

const SVG_PIXELS: f32 = 1600.;

/// The PDF's page count, from the helper, which also renders page one to be sure it reads.
fn pdf_pages(path: &Path) -> Option<usize> {
    let out = std::env::temp_dir().join(format!("isle-look-{}.png", std::process::id()));
    let ran = std::process::Command::new(crate::thumbs::helper())
        .args(["page".as_ref(), path.as_os_str(), "0".as_ref(), "64".as_ref(), out.as_os_str()])
        .output()
        .ok()?;
    let _ = std::fs::remove_file(&out);
    String::from_utf8_lossy(&ran.stdout).trim().parse().ok()
}

/// A PDF page as PNG bytes at `pixels` across.
fn pdf_page(path: &Path, page: usize, pixels: u32) -> Option<Bytes> {
    let out = std::env::temp_dir().join(format!("isle-look-{}-{page}-{pixels}.png", std::process::id()));
    let ran = std::process::Command::new(crate::thumbs::helper())
        .args(["page".as_ref(), path.as_os_str(), page.to_string().as_ref(), pixels.to_string().as_ref(), out.as_os_str()])
        .status()
        .ok()?;
    let bytes = ran.success().then(|| std::fs::read(&out).ok()).flatten();
    let _ = std::fs::remove_file(&out);
    bytes.map(Bytes::from)
}

/// The file's content as Quick Look shows it, read when the path changes.
#[derive(PartialEq)]
pub struct Look {
    pub path: PathBuf,
    /// Shown instead when the file has nothing of its own to show.
    pub facts: Element,
}

impl Component for Look {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let dark = theme.appearance.mode == isle_tokens::Mode::Dark;
        let mut shown = use_state(|| None::<(PathBuf, Shown)>);
        use_side_effect_with_deps(&(self.path.clone(), dark), move |(path, dark)| {
            let (path, dark) = (path.clone(), *dark);
            spawn(async move {
                let read_path = path.clone();
                if let Some(read) = off_thread(move || read(&read_path, dark)).await {
                    shown.set(Some((path, read)));
                }
            });
        });
        let now = shown.read().clone().filter(|(p, _)| *p == self.path).map(|(_, s)| s);
        let body: Element = match now {
            None => rect().expanded().into_element(),
            Some(Shown::Image(bytes)) => ImageViewer::new(bytes).expanded().aspect_ratio(AspectRatio::Min).image_cover(ImageCover::Center).into_element(),
            Some(Shown::Text(lines)) => TextView { lines }.into_element(),
            Some(Shown::Pdf(count)) => Pages { path: self.path.clone(), count }.into_element(),
            Some(Shown::Media(video)) => Media { path: self.path.clone(), video }.into_element(),
            Some(Shown::Listing(names)) => {
                let lines = names.into_iter().map(|n| vec![((140, 140, 140), n)]).collect();
                TextView { lines }.into_element()
            }
            Some(Shown::Facts) => self.facts.clone(),
        };
        rect().expanded().center().background(colour(t.colour_view)).child(body)
    }
}

#[derive(PartialEq)]
struct TextView {
    lines: Vec<Line>,
}

impl Component for TextView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let size = text::metrics(&theme, text::Style::Callout).0;
        let lines = std::rc::Rc::new(self.lines.clone());
        let length = lines.len();
        let lines = VirtualScrollView::new_with_data(lines, move |item: VirtualItem, lines: &std::rc::Rc<Vec<Line>>| {
            let runs = lines.get(item.index).cloned().unwrap_or_default();
            rect()
                .width(Size::fill())
                .height(Size::px(size * 1.5))
                .padding((0., 16.))
                .child(
                    paragraph()
                        .font_family("JetBrains Mono")
                        .font_size(size)
                        .max_lines(1)
                        .spans_iter(runs.into_iter().map(|((r, g, b), run)| Span::new(run).color(Color::from_rgb(r, g, b)))),
                )
                .into_element()
        })
        .length(length)
        .item_size(size * 1.5)
        .expanded();
        rect().expanded().padding((theme.tokens.space_m, 0.)).child(lines)
    }
}

/// A PDF's pages, each rendered by the helper as it scrolls into view.
#[derive(PartialEq)]
struct Pages {
    path: PathBuf,
    count: usize,
}

const PAGE_WIDTH: f32 = 720.;

impl Component for Pages {
    fn render(&self) -> impl IntoElement {
        let data = (self.path.clone(), self.count);
        VirtualScrollView::new_with_data(data, |item: VirtualItem, (path, _): &(PathBuf, usize)| {
            Page { path: path.clone(), index: item.index }.into_element()
        })
        .length(self.count)
        .item_size(PAGE_WIDTH * 1.32)
        .expanded()
    }
}

#[derive(PartialEq)]
struct Page {
    path: PathBuf,
    index: usize,
}

impl Component for Page {
    fn render(&self) -> impl IntoElement {
        let mut bytes = use_state(|| None::<Bytes>);
        use_hook({
            let (path, index) = (self.path.clone(), self.index);
            move || {
                spawn(async move {
                    if let Some(Some(png)) = off_thread(move || pdf_page(&path, index, (PAGE_WIDTH * 2.) as u32)).await {
                        bytes.set(Some(png));
                    }
                });
            }
        });
        rect()
            .width(Size::fill())
            .height(Size::px(PAGE_WIDTH * 1.32))
            .padding(8.)
            .center()
            .maybe_child(bytes.read().clone().map(|b| ImageViewer::new(b).expanded().aspect_ratio(AspectRatio::Min).image_cover(ImageCover::Center)))
    }
}

/// Audio and video with play and pause and the time played.
#[derive(PartialEq)]
struct Media {
    path: PathBuf,
    video: bool,
}

impl Component for Media {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let player = freya::video::use_video({
            let path = self.path.clone();
            move || path
        });
        use_drop({
            let mut player = player.clone();
            move || player.stop()
        });
        let playing = player.state() == freya::video::PlaybackState::Playing;
        let toggle = {
            let player = player.clone();
            move |_| {
                let mut player = player.clone();
                player.toggle();
            }
        };
        let progress = player.progress().clamp(0., 1.) as f32;
        rect()
            .expanded()
            .content(Content::flex())
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .center()
                    .maybe_child(player.frame().map(|frame| image(frame).expanded().aspect_ratio(AspectRatio::Min).image_cover(ImageCover::Center)))
                    .maybe_child((!self.video).then(|| Glyph::new("music-notes", t.size_app_icon_hero, colour(t.colour_text_3)))),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_row_large))
                    .padding((0., t.space_l))
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .spacing(t.space_m)
                    .child(
                        rect()
                            .width(Size::px(t.size_control))
                            .height(Size::px(t.size_control))
                            .center()
                            .a11y_role(AccessibilityRole::Button)
                            .a11y_alt(if playing { "Pause" } else { "Play" })
                            .on_press(toggle)
                            .child(Glyph::new(if playing { "pause" } else { "play" }, t.size_glyph, colour(t.colour_text)).fill(true)),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .height(Size::px(4.))
                            .corner_radius(2.)
                            .background(colour(t.colour_fill_strong))
                            .child(rect().width(Size::percent(progress * 100.)).height(Size::fill()).corner_radius(2.).background(colour(t.colour_accent))),
                    )
                    .child(text::line(&theme, text::Style::Callout, colour(t.colour_text_2), clock(player.position()))),
            )
    }
}

fn clock(at: std::time::Duration) -> String {
    let s = at.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_is_highlighted_by_its_extension() {
        let lines = highlight(Path::new("main.rs"), "fn main() {}\n", true);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].len() > 1, "more than one colour: {:?}", lines[0]);
        let joined: String = lines[0].iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(joined, "fn main() {}");
    }

    #[test]
    fn kinds_are_read_as_quick_look_shows_them() {
        let dir = tempfile::tempdir().unwrap();
        let note = dir.path().join("a.md");
        std::fs::write(&note, "# Title\n\nwords\n").unwrap();
        assert!(matches!(read(&note, false), Shown::Text(lines) if lines.len() == 3));
        assert_eq!(read(dir.path(), false), Shown::Facts);
    }
}
