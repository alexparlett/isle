//! The full-output launcher drawn by Slint's software renderer, uploaded by dirty region.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use island_slint::{
    Surface,
    slint_interpreter::{Struct, Value},
};
use slint::{Brush, Color, Model, ModelRc, SharedString, VecModel};
use spike_base::{
    IslandContent, UiEvent,
    smithay::{
        backend::renderer::gles::{GlesRenderer, GlesTexture},
        utils::{Physical, Size},
    },
};

const APPS: [&str; 48] = [
    "Firefox", "Files", "Terminal", "Settings", "Steam", "Heroic", "Lutris", "Spotify", "Discord", "Signal",
    "Thunderbird", "Calendar", "Calculator", "Clocks", "Weather", "Maps", "Photos", "Camera", "Music", "Videos",
    "Podcasts", "Books", "Notes", "Text Editor", "Code", "Zed", "Blender", "GIMP", "Inkscape", "Krita",
    "Kdenlive", "OBS Studio", "Audacity", "Ardour", "Bottles", "ProtonUp", "MangoHud", "Monitor", "Disks",
    "Keychain", "Cleaner", "Archive", "Fonts", "Characters", "Contacts", "Remote Desktop", "Torrent", "Backups",
];
const SHOWN: usize = 24;
/// Matches the base's window for counting a frame as the open or close.
const TRANSITION: Duration = Duration::from_millis(400);

/// `hsl(h, 55%, 55%)` with `h` in degrees.
fn tile_color(h: f64) -> Color {
    let (s, l) = (0.55, 0.55);
    let c = (1.0 - (2.0 * l - 1.0f64).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let byte = |v: f64| ((v + m) * 255.0).round() as u8;
    Color::from_rgb_u8(byte(r), byte(g), byte(b))
}

/// The app list, filtered on the Rust side into the model the grid shows.
struct Apps {
    all: Vec<(String, Value)>,
    shown: Rc<VecModel<Value>>,
}

impl Apps {
    fn new() -> Rc<Self> {
        let all = APPS
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let app: Struct = [
                    ("name".to_string(), Value::String(SharedString::from(*name))),
                    ("color".to_string(), Value::Brush(Brush::SolidColor(tile_color(i as f64 * 360.0 / 48.0)))),
                ]
                .into_iter()
                .collect();
                (name.to_lowercase(), Value::Struct(app))
            })
            .collect();
        let apps = Rc::new(Self { all, shown: Rc::new(VecModel::default()) });
        apps.filter("");
        apps
    }

    fn filter(&self, query: &str) {
        let query = query.to_lowercase();
        let matches: Vec<Value> =
            self.all.iter().filter(|(name, _)| name.contains(&query)).take(SHOWN).map(|(_, v)| v.clone()).collect();
        // Typing that does not change the matches leaves the grid alone.
        if matches.len() != self.shown.row_count() || matches.iter().enumerate().any(|(i, v)| self.shown.row_data(i).as_ref() != Some(v)) {
            self.shown.set_vec(matches);
        }
    }
}

struct SlintLauncher {
    surface: Surface,
    apps: Rc<Apps>,
    open: bool,
    animated: bool,
    toggled_at: Instant,
}

impl SlintLauncher {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut surface = Surface::new("launcher.slint", "Launcher")?;
        let apps = Apps::new();
        let for_setup = apps.clone();
        surface.on_instance(move |instance| {
            let _ = instance.set_property("apps", Value::Model(ModelRc::from(for_setup.shown.clone())));
            let apps = for_setup.clone();
            let _ = instance.set_callback("query-edited", move |args| {
                if let Some(Value::String(query)) = args.first() {
                    apps.filter(query);
                }
                Value::Void
            });
        });
        Ok(Self { surface, apps, open: false, animated: true, toggled_at: Instant::now() - TRANSITION })
    }

    fn set_query(&self, query: &str) {
        self.surface.set("query", Value::String(query.into()));
        self.apps.filter(query);
    }
}

impl IslandContent for SlintLauncher {
    fn name(&self) -> &'static str {
        "slint"
    }

    fn init(&mut self, renderer: &mut GlesRenderer, size: Size<i32, Physical>, _scale: f64) {
        // The spec's sizes are logical pixels at scale 1 on the output-sized texture.
        self.surface.init(renderer, size, 1.0);
        self.surface.set("animated", Value::Bool(self.animated));
    }

    // Called before init.
    fn set_animated(&mut self, animated: bool) {
        self.animated = animated;
    }

    fn set_expanded(&mut self, open: bool) {
        self.open = open;
        self.toggled_at = Instant::now();
        if open {
            self.set_query("");
            self.surface.invoke("focus-search");
        } else {
            self.surface.invoke("blur-search");
        }
        self.surface.set("open", Value::Bool(open));
    }

    fn set_layout_size(&mut self, _size: Size<i32, Physical>) {}

    fn event(&mut self, event: UiEvent) {
        self.surface.event(event);
    }

    fn tick(&mut self, _now: Instant) -> bool {
        self.surface.draw()
    }

    fn render(&mut self, renderer: &mut GlesRenderer) -> GlesTexture {
        let kind = match (self.toggled_at.elapsed() < TRANSITION, self.open) {
            (true, true) => "opening",
            (true, false) => "closing",
            (false, true) => "open (typing, caret)",
            (false, false) => "closed",
        };
        self.surface.upload(renderer, kind)
    }

    fn poll_reload(&mut self) -> bool {
        self.surface.poll_reload()
    }

    fn prepare_shot(&mut self) {
        self.set_query("co");
        self.surface.invoke("focus-search");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    spike_base::run_launcher(SlintLauncher::new()?)
}
