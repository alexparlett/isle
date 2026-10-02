use std::{fmt, time::Duration};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Morph,
    Expanded,
    Rest,
    Idle,
}

const PHASES: [Phase; 4] = [Phase::Morph, Phase::Expanded, Phase::Rest, Phase::Idle];

#[derive(Default)]
struct Samples {
    content: Vec<f64>,
    frame: Vec<f64>,
    swap: Vec<f64>,
    last: Option<std::time::Instant>,
    span: f64,
}

pub struct Recorder {
    samples: [Samples; 4],
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            samples: Default::default(),
        }
    }

    /// Times are CPU time: the content's update and draw, the whole frame
    /// before the swap, and the swap itself.
    pub fn record(&mut self, phase: Phase, content: Duration, frame: Duration, swap: Duration) {
        let s = &mut self.samples[PHASES.iter().position(|p| *p == phase).unwrap()];
        s.content.push(content.as_secs_f64() * 1000.0);
        s.frame.push(frame.as_secs_f64() * 1000.0);
        s.swap.push(swap.as_secs_f64() * 1000.0);
        let now = std::time::Instant::now();
        if let Some(last) = s.last {
            let gap = (now - last).as_secs_f64();
            if gap < 0.1 {
                s.span += gap;
            }
        }
        s.last = Some(now);
    }

    pub fn print(&self) {
        println!(
            "{:<9} {:>6}  {:>22}  {:>22}",
            "phase", "frames", "content ms p50/p95/max", "frame ms p50/p95/max"
        );
        for (phase, s) in PHASES.iter().zip(&self.samples) {
            println!(
                "{:<9} {:>6}  {:>22}  {:>22}",
                format!("{phase:?}"),
                s.content.len(),
                stats(&s.content),
                stats(&s.frame)
            );
        }
        for (phase, s) in PHASES.iter().zip(&self.samples) {
            if !s.swap.is_empty() {
                let fps = if s.span > 0.0 {
                    s.content.len() as f64 / s.span
                } else {
                    0.0
                };
                println!(
                    "{:<9} swap ms {}  ~{fps:.0} fps",
                    format!("{phase:?}"),
                    stats(&s.swap)
                );
            }
        }
        let idle = &self.samples[3];
        println!("idle frames over the last 10 s: {}", idle.content.len());
    }
}

fn stats(v: &[f64]) -> String {
    if v.is_empty() {
        return "-".into();
    }
    let mut v = v.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    format!("{:.2}/{:.2}/{:.2}", at(0.5), at(0.95), v[v.len() - 1])
}

pub struct Memory {
    rss_kb: u64,
    pss_kb: u64,
}

impl Memory {
    pub fn read() -> Self {
        let text = std::fs::read_to_string("/proc/self/smaps_rollup").unwrap_or_default();
        let field = |name: &str| {
            text.lines()
                .find(|l| l.starts_with(name))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|n| n.parse().ok())
                .unwrap_or(0)
        };
        Self {
            rss_kb: field("Rss:"),
            pss_kb: field("Pss:"),
        }
    }
}

impl fmt::Display for Memory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RSS {:.1} MB, PSS {:.1} MB",
            self.rss_kb as f64 / 1024.0,
            self.pss_kb as f64 / 1024.0
        )
    }
}

/// This process's line from `nvidia-smi`'s process table.
pub fn gpu_memory() -> String {
    let pid = std::process::id().to_string();
    let out = std::process::Command::new("nvidia-smi").output();
    match out {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .find(|l| l.split_whitespace().any(|w| w == pid))
            .map(|l| l.split_whitespace().last().unwrap_or("?").to_string())
            .map(|_| {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .find(|l| l.split_whitespace().any(|w| w == pid))
                    .unwrap()
                    .split_whitespace()
                    .rev()
                    .nth(1)
                    .unwrap_or("?")
                    .to_string()
            })
            .unwrap_or_else(|| "not listed".into()),
        Err(_) => "nvidia-smi unavailable".into(),
    }
}

/// Writes bottom-up GL rows as a top-down PNG.
pub fn write_png(
    path: &str,
    w: u32,
    h: u32,
    pixels: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let stride = (w * 4) as usize;
    let mut flipped = Vec::with_capacity(pixels.len());
    for row in pixels.chunks(stride).rev() {
        flipped.extend_from_slice(row);
    }
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&flipped)?;
    Ok(())
}
