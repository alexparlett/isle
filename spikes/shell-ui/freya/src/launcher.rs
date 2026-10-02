//! The full-output launcher as Freya components.

use freya::{animation::*, prelude::*};

use crate::widgets::{PRIMARY, TextField};

const BACKDROP: Color = Color::from_argb(115, 0, 0, 0);
const PANEL: Color = Color::from_argb(235, 20, 20, 26);
const BORDER: Color = Color::from_argb(26, 255, 255, 255);
const COLUMNS: usize = 6;
const SHOWN: usize = 24;

pub const APPS: [&str; 48] = [
    "Firefox",
    "Files",
    "Terminal",
    "Settings",
    "Steam",
    "Heroic",
    "Lutris",
    "Spotify",
    "Discord",
    "Signal",
    "Thunderbird",
    "Calendar",
    "Calculator",
    "Clocks",
    "Weather",
    "Maps",
    "Photos",
    "Camera",
    "Music",
    "Videos",
    "Podcasts",
    "Books",
    "Notes",
    "Text Editor",
    "Code",
    "Zed",
    "Blender",
    "GIMP",
    "Inkscape",
    "Krita",
    "Kdenlive",
    "OBS Studio",
    "Audacity",
    "Ardour",
    "Bottles",
    "ProtonUp",
    "MangoHud",
    "Monitor",
    "Disks",
    "Keychain",
    "Cleaner",
    "Archive",
    "Fonts",
    "Characters",
    "Contacts",
    "Remote Desktop",
    "Torrent",
    "Backups",
];

/// What the compositor drives, provided at the root.
#[derive(Clone, Copy, PartialEq)]
pub struct Launcher {
    pub open: State<bool>,
    /// False when the compositor animates the surface and the content snaps.
    pub animated: State<bool>,
    pub query: State<String>,
    pub field: AccessibilityId,
}

impl Launcher {
    pub fn new() -> Self {
        Self {
            open: State::create(false),
            animated: State::create(true),
            query: State::create(String::new()),
            field: AccessibilityId::new_unique(),
        }
    }
}

pub fn app() -> impl IntoElement {
    let launcher = use_consume::<Launcher>();
    let open = *launcher.open.read();
    let animated = *launcher.animated.read();

    use_side_effect(move || {
        if *launcher.open.read() {
            launcher.field.request_focus();
        } else {
            launcher.field.request_unfocus();
        }
    });

    let shown = use_animation_with_dependencies(&(open, animated), |conf, (open, animated)| {
        conf.on_creation(OnCreation::Finish);
        conf.on_change(if *animated {
            OnChange::Rerun
        } else {
            OnChange::Finish
        });
        if *open {
            AnimNum::new(0., 1.)
                .time(260)
                .ease(Ease::Out)
                .function(Function::Cubic)
        } else {
            AnimNum::new(1., 0.)
                .time(180)
                .ease(Ease::In)
                .function(Function::Cubic)
        }
    });
    let t = shown.get().value();

    // Freya fades each element through its own layer, so a closed launcher is unmounted
    // instead of drawn at zero opacity. It mounts on open so the field can take focus.
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .maybe(open || t > 0., |el| {
            el.background(BACKDROP)
                .opacity(t)
                .center()
                .color(PRIMARY)
                .child(
                    rect()
                        .offset_y(24. * (1. - t))
                        .scale(0.96 + 0.04 * t)
                        .child(Panel {
                            query: launcher.query,
                            field: launcher.field,
                        }),
                )
        })
}

#[derive(PartialEq)]
struct Panel {
    query: State<String>,
    field: AccessibilityId,
}

impl Component for Panel {
    fn render(&self) -> impl IntoElement {
        let query = self.query.read().to_lowercase();
        let matches: Vec<(usize, &str)> = APPS
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, name)| name.to_lowercase().contains(&query))
            .take(SHOWN)
            .collect();

        rect()
            .width(Size::px(1200.))
            .height(Size::px(760.))
            .corner_radius(28.)
            .background(PANEL)
            .border(
                Border::new()
                    .fill(BORDER)
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .padding(32.)
            .spacing(24.)
            .child(TextField {
                text: self.query,
                id: self.field,
                placeholder: "Search apps",
                height: 48.,
                radius: 14.,
                padding: 16.,
                font_size: 18.,
            })
            .child(
                rect()
                    .spacing(16.)
                    .children(matches.chunks(COLUMNS).map(|row| {
                        rect()
                            .horizontal()
                            .spacing(16.)
                            .children(row.iter().map(|&(index, name)| tile(index, name)))
                    })),
            )
    }
}

fn tile(index: usize, name: &'static str) -> impl IntoElement {
    rect()
        .key(index)
        .width(Size::px(160.))
        .height(Size::px(140.))
        .cross_align(Alignment::Center)
        .spacing(12.)
        .child(
            rect()
                .width(Size::px(64.))
                .height(Size::px(64.))
                .corner_radius(16.)
                .background(hsl(index as f32 * 360. / APPS.len() as f32, 0.55, 0.55)),
        )
        .child(
            label()
                .text(name)
                .font_size(14.)
                .width(Size::fill())
                .text_align(TextAlign::Center)
                .text_overflow(TextOverflow::Ellipsis),
        )
}

fn hsl(h: f32, s: f32, l: f32) -> Color {
    let c = (1. - (2. * l - 1.).abs()) * s;
    let x = c * (1. - ((h / 60.) % 2. - 1.).abs());
    let (r, g, b) = match (h / 60.) as u32 {
        0 => (c, x, 0.),
        1 => (x, c, 0.),
        2 => (0., c, x),
        3 => (0., x, c),
        4 => (x, 0., c),
        _ => (c, 0., x),
    };
    let m = l - c / 2.;
    let byte = |v: f32| ((v + m) * 255.).round() as u8;
    Color::from_rgb(byte(r), byte(g), byte(b))
}
