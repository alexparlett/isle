//! Yours' author tools: a new widget from a template, its check, and its picture
//! (`structure/widgets.md` The gallery, Yours; v1 D30).

use std::path::{Path, PathBuf};

use crate::state::State;

/// A new provider's folder under the user's own, `my-widget`, `my-widget-2`…
fn new_folder(root: &Path) -> (String, PathBuf) {
    (1..)
        .map(|n| if n == 1 { "my-widget".to_owned() } else { format!("my-widget-{n}") })
        .map(|id| (id.clone(), root.join(id)))
        .find(|(_, p)| !p.exists())
        .unwrap()
}

/// Writes the template: a text widget whose Python answers once a minute.
pub fn write_template(root: &Path) -> std::io::Result<(String, PathBuf)> {
    let (id, folder) = new_folder(root);
    std::fs::create_dir_all(&folder)?;
    std::fs::write(
        folder.join("widget.json"),
        format!(
            r#"{{
  "id": "{id}",
  "name": "My Widget",
  "description": "What it shows.",
  "version": "0.1.0",
  "author": {{ "name": "" }},
  "license": "MIT",
  "sizes": ["2x1", "2x2"],
  "default": "2x1",
  "template": "text",
  "script": "widget.py",
  "interval": 60,
  "permissions": {{}},
  "settings": []
}}
"#
        ),
    )?;
    std::fs::write(
        folder.join("widget.py"),
        "\"\"\"Prints one JSON answer for the text template; the dashboard's input is on stdin.\"\"\"\n\nimport json\nimport sys\n\nasked = json.loads(sys.stdin.readline() or \"{}\")\nprint(json.dumps({\"text\": \"Hello from \" + asked.get(\"instance\", \"my widget\")}))\n",
    )?;
    std::fs::write(folder.join("README.md"), "# My Widget\n\nWhat it shows, and why it asks for what it asks for.\n")?;
    std::fs::write(folder.join("CHANGELOG.md"), "# Changelog\n\n## 0.1.0\n\n- First version.\n")?;
    Ok((id, folder))
}

/// The first problem with a provider, or that it is ready.
pub fn check(folder: &Path) -> String {
    match isle_providers::Manifest::load(folder) {
        Ok(_) => "Ready".to_owned(),
        Err(problems) => problems.into_iter().next().unwrap_or_default(),
    }
}

impl State {
    pub fn new_widget(&mut self) {
        let root = isle_providers::manifest::own_folder();
        match write_template(&root) {
            Ok((id, folder)) => {
                self.isle.dashboard.providers = crate::providers::scan();
                self.close_dashboard();
                self.launch(crate::launch::Launch::Open(folder.join("widget.json").to_string_lossy().into_owned()));
                tracing::info!(id, "a new widget");
            }
            Err(e) => tracing::warn!("a new widget: {e}"),
        }
    }

    /// The placed widget of `id` shot from its screen into `screenshots/card.png`.
    pub fn update_picture(&mut self, id: &str) -> String {
        let Some(p) = self.isle.provider_of(id).cloned() else {
            return "It isn't installed".to_owned();
        };
        let Some(open) = self.isle.dashboard.open.as_ref() else {
            return String::new();
        };
        let Some(placed) = open.layout.widgets.iter().find(|w| w.kind() == p.manifest.id).cloned() else {
            return "Place it to take its picture".to_owned();
        };
        let Some(output) = self.isle.space.outputs().find(|o| o.name() == open.output).cloned() else {
            return String::new();
        };
        let (x, y, w, h) = open.extent.rect(placed.at);
        self.draw_shell(&output, false);
        let pixels = match self.backend.screenshot(&self.isle, &output) {
            Ok(p) => p,
            Err(e) => return e.to_string(),
        };
        let scale = output.current_scale().fractional_scale() as f32;
        let (x, y, w, h) = (
            (x * scale) as u32,
            (y * scale) as u32,
            ((w * scale) as u32).min(pixels.width),
            ((h * scale) as u32).min(pixels.height),
        );
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for row in y..(y + h).min(pixels.height) {
            let start = ((row * pixels.width + x) * 4) as usize;
            let end = start + (w.min(pixels.width - x) * 4) as usize;
            rgba.extend_from_slice(&pixels.rgba[start..end]);
        }
        let path = p.folder.join("screenshots/card.png");
        let written = (|| -> Result<(), Box<dyn std::error::Error>> {
            std::fs::create_dir_all(path.parent().unwrap())?;
            let file = std::io::BufWriter::new(std::fs::File::create(&path)?);
            let mut encoder = png::Encoder::new(file, w, (rgba.len() / (w as usize * 4)) as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header()?.write_image_data(&rgba)?;
            Ok(())
        })();
        match written {
            Ok(()) => "Picture updated".to_owned(),
            Err(e) => e.to_string(),
        }
    }
}
