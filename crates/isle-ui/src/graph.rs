//! A graph of samples in a well (`p1/surfaces/monitor.md` § The footer): series as lines
//! over areas, stacked or mirrored about a centre line, gaps where nothing was sampled, a
//! scale in 1, 2 and 5 steps for rates, and a hover line read by pointer or by keys.

use std::rc::Rc;

use bytes::Bytes;
use freya::{animation::*, prelude::*};
use isle_tokens::Colour;

use crate::{colour, text, use_theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Each series from the baseline.
    Lines,
    /// Each series on the ones before it.
    Stacked,
    /// The first series above a centre line, the second below it.
    Mirrored,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    /// Oldest first; none where nothing was sampled.
    pub values: Vec<Option<f32>>,
    pub colour: Colour,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphData {
    pub series: Vec<Series>,
    pub shape: Shape,
    /// The values' top; none scales to the largest shown, rounded up to a 1, 2 or 5 step.
    pub max: Option<f32>,
    /// How a value is written for the scale and the hover.
    pub unit: Unit,
    /// Each sample's time, oldest first, for the hover.
    pub times: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// 0 to 1, written as a percentage.
    Share,
    /// Bytes a second, in decimal units as the Mac writes them.
    BytesPerSecond,
    /// Operations or packets a second.
    PerSecond,
    Watts,
}

impl Unit {
    pub fn format(self, v: f32) -> String {
        match self {
            Unit::Share => format!("{:.0}%", v * 100.0),
            Unit::BytesPerSecond => {
                let (n, unit) = [(1e9, "GB/s"), (1e6, "MB/s"), (1e3, "KB/s")]
                    .into_iter()
                    .find(|(d, _)| v >= *d)
                    .map_or((v, "bytes/s"), |(d, u)| (v / d, u));
                if n >= 10.0 || unit == "bytes/s" { format!("{n:.0} {unit}") } else { format!("{n:.1} {unit}") }
            }
            Unit::PerSecond => format!("{v:.0}/s"),
            Unit::Watts => format!("{v:.0} W"),
        }
    }
}

/// The least 1, 2 or 5 step at or above `v`: 3 is 5, 70 is 100, 0.3 is 0.5.
pub fn nice_max(v: f32) -> f32 {
    if v <= 0.0 || !v.is_finite() {
        return 1.0;
    }
    let decade = 10f32.powf(v.log10().floor());
    [1.0, 2.0, 5.0, 10.0].into_iter().map(|m| m * decade).find(|m| *m >= v * 0.999_9).unwrap_or(10.0 * decade)
}

/// The top the series are drawn to.
pub fn top(data: &GraphData) -> f32 {
    if let Some(max) = data.max {
        return max;
    }
    let len = data.series.iter().map(|s| s.values.len()).max().unwrap_or(0);
    let largest = (0..len)
        .map(|i| {
            let at = |s: &Series| s.values.get(i).copied().flatten().unwrap_or(0.0);
            match data.shape {
                Shape::Stacked => data.series.iter().map(at).sum::<f32>(),
                _ => data.series.iter().map(at).fold(0.0, f32::max),
            }
        })
        .fold(0.0, f32::max);
    nice_max(largest)
}

/// Each series' runs of points between gaps, as `(x, upper y, lower y)`: the newest sample
/// at `width`, one every `step` points leftward.
pub fn runs(data: &GraphData, width: f32, height: f32, step: f32) -> Vec<Vec<Vec<(f32, f32, f32)>>> {
    let max = top(data).max(f32::EPSILON);
    let len = data.series.iter().map(|s| s.values.len()).max().unwrap_or(0);
    let mut base = vec![0.0f32; len];
    let mid = height / 2.0;
    data.series
        .iter()
        .enumerate()
        .map(|(n, series)| {
            let offset = len - series.values.len();
            let mut out: Vec<Vec<(f32, f32, f32)>> = Vec::new();
            let mut run: Vec<(f32, f32, f32)> = Vec::new();
            for (i, value) in series.values.iter().enumerate() {
                let at = i + offset;
                let x = width - (len - 1 - at) as f32 * step;
                let Some(v) = value else {
                    if !run.is_empty() {
                        out.push(std::mem::take(&mut run));
                    }
                    continue;
                };
                let share = (v / max).clamp(0.0, 1.0);
                let (upper, lower) = match data.shape {
                    Shape::Lines => (height - share * height, height),
                    Shape::Stacked => {
                        let below = base[at];
                        base[at] = (below + share).min(1.0);
                        (height - base[at] * height, height - below * height)
                    }
                    Shape::Mirrored if n == 0 => (mid - share * mid, mid),
                    Shape::Mirrored => (mid + share * mid, mid),
                };
                if x >= 0.0 {
                    run.push((x, upper, lower));
                }
            }
            if !run.is_empty() {
                out.push(run);
            }
            out
        })
        .collect()
}

fn css(c: Colour, alpha: f32) -> String {
    format!("rgba({},{},{},{})", c.r, c.g, c.b, c.a * alpha)
}

/// The well's drawing: grid hairlines, then each series' areas and lines.
fn svg(data: &GraphData, width: f32, height: f32, step: f32, theme: &isle_tokens::Theme) -> String {
    let t = &theme.tokens;
    let mut out = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}">"#);
    let hairline = css(t.colour_hairline, 1.0);
    for q in [0.25, 0.5, 0.75] {
        let y = height * q;
        let stroke = if data.shape == Shape::Mirrored && q == 0.5 { css(t.colour_hairline_strong, 1.0) } else { hairline.clone() };
        out += &format!(r#"<line x1="0" y1="{y}" x2="{width}" y2="{y}" stroke="{stroke}" stroke-width="1"/>"#);
    }
    for (series, runs) in data.series.iter().zip(runs(data, width, height, step)) {
        for run in runs {
            let upper: Vec<String> = run.iter().map(|(x, y, _)| format!("{x:.1},{y:.1}")).collect();
            let lower: Vec<String> = run.iter().rev().map(|(x, _, y)| format!("{x:.1},{y:.1}")).collect();
            out += &format!(
                r#"<polygon points="{} {}" fill="{}"/><polyline points="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round" stroke-linecap="round"/>"#,
                upper.join(" "),
                lower.join(" "),
                css(series.colour, t.series_area_opacity),
                upper.join(" "),
                css(series.colour, 1.0),
                t.stroke_graph,
            );
        }
    }
    out + "</svg>"
}

#[derive(PartialEq)]
pub struct Graph {
    pub data: Rc<GraphData>,
    pub width: f32,
    /// The accessible name: "CPU load".
    pub name: String,
    /// What it says read as a whole: "CPU load, last 2 minutes, 12 to 46%, now 31%".
    pub summary: String,
}

impl Component for Graph {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let a11y = use_a11y();
        let mut hover = use_state(|| None::<usize>);
        let (width, height, step) = (self.width, t.size_graph, t.size_graph_step);
        let len = self.data.series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        // Each sample moves the trace one step toward the leading edge and then it stands
        // still; under Reduce Motion it jumps.
        let reduced = theme.appearance.reduce_motion;
        let mut glide = use_animation(move |_| AnimNum::new(step, 0.0).time(240).function(Function::Cubic));
        use_side_effect_with_deps(&len, move |_| {
            if !reduced {
                glide.start();
            }
        });
        let dx = if reduced { 0.0 } else { glide.get().value() };
        let drawing = svg(&self.data, width, height, step, &theme);
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            drawing.hash(&mut h);
            h.finish()
        };
        // The sample nearest a point `x` from the well's leading edge.
        let nearest = move |x: f32| -> Option<usize> {
            let back = ((width - x) / step).round().max(0.0) as usize;
            (back < len).then(|| len - 1 - back)
        };
        let scale = self.data.max.is_none().then(|| self.data.unit.format(top(&self.data)));
        let reading = (*hover.read()).map(|i| {
            let values: Vec<String> = self
                .data
                .series
                .iter()
                .filter_map(|s| s.values.get(i + s.values.len() - len).copied().flatten())
                .map(|v| self.data.unit.format(v))
                .collect();
            let time = self.data.times.get(i).cloned().unwrap_or_default();
            (i, format!("{}  {time}", values.join(", ")))
        });
        let keys = move |e: Event<KeyboardEventData>| {
            let now = *hover.peek();
            let to = match e.key {
                Key::Named(NamedKey::ArrowLeft) => Some(now.map_or(len.saturating_sub(1), |i| i.saturating_sub(1))),
                Key::Named(NamedKey::ArrowRight) => Some(now.map_or(len.saturating_sub(1), |i| (i + 1).min(len.saturating_sub(1)))),
                Key::Named(NamedKey::Escape) => None,
                _ => return,
            };
            e.stop_propagation();
            hover.set(to.filter(|_| len > 0));
        };
        let line = reading.as_ref().map(|(i, _)| {
            let x = width - (len - 1 - i) as f32 * step;
            rect()
                .position(Position::new_absolute().left(x).top(0.))
                .width(Size::px(1.))
                .height(Size::px(height))
                .background(colour(t.colour_hairline_strong))
                .into_element()
        });
        let tip = reading.as_ref().map(|(_, words)| {
            rect()
                .position(Position::new_absolute().left(0.).top(-t.size_control - t.space_xs))
                .padding((t.space_xxs, t.space_s))
                .corner_radius(t.radius_control)
                .background(colour(t.colour_window_raised))
                .child(text::line(&theme, text::Style::Callout, colour(t.colour_text), words.clone()))
                .into_element()
        });
        rect()
            .width(Size::px(width))
            .height(Size::px(height))
            .corner_radius(t.radius_control)
            .overflow(Overflow::Clip)
            .background(colour(t.colour_window_sunken))
            .border(
                Border::new()
                    .fill(colour(if theme.appearance.increase_contrast { t.colour_hairline_strong } else { t.colour_hairline }))
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
            .a11y_id(a11y)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Figure)
            .a11y_alt(match &reading {
                Some((_, words)) => format!("{}, {words}", self.name),
                None => self.summary.clone(),
            })
            .on_key_down(keys)
            .on_pointer_move(move |e: Event<PointerEventData>| hover.set_if_modified(nearest(e.element_location().x as f32)))
            .on_pointer_leave(move |_| hover.set_if_modified(None))
            .child(
                rect().margin((0., 0., 0., dx)).child(
                    SvgViewer::new((key, Bytes::from(drawing.into_bytes())))
                        .show_loader(false)
                        .width(Size::px(width))
                        .height(Size::px(height)),
                ),
            )
            .maybe_child(scale.map(|s| {
                rect()
                    .position(Position::new_absolute().left(t.space_xs).top(t.space_xs))
                    .child(text::line(&theme, text::Style::Footnote, colour(t.colour_text_2), s))
                    .into_element()
            }))
            .maybe_child(line)
            .maybe_child(tip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(shape: Shape, series: Vec<Vec<Option<f32>>>, max: Option<f32>) -> GraphData {
        GraphData {
            series: series.into_iter().map(|values| Series { values, colour: Colour::parse("#000000").unwrap() }).collect(),
            shape,
            max,
            unit: Unit::Share,
            times: Vec::new(),
        }
    }

    #[test]
    fn units_write_as_the_mac_does() {
        assert_eq!(Unit::BytesPerSecond.format(52_300_000.0), "52 MB/s");
        assert_eq!(Unit::BytesPerSecond.format(1_500.0), "1.5 KB/s");
        assert_eq!(Unit::BytesPerSecond.format(300.0), "300 bytes/s");
        assert_eq!(Unit::Share.format(0.31), "31%");
        assert_eq!(Unit::Watts.format(84.4), "84 W");
    }

    #[test]
    fn the_scale_rounds_up_to_a_one_two_or_five_step() {
        assert_eq!(nice_max(3.0), 5.0);
        assert_eq!(nice_max(70.0), 100.0);
        assert_eq!(nice_max(0.3), 0.5);
        assert_eq!(nice_max(200.0), 200.0);
        assert_eq!(nice_max(0.0), 1.0);
        assert_eq!(top(&data(Shape::Stacked, vec![vec![Some(30.0)], vec![Some(40.0)]], None)), 100.0, "stacked sums");
        assert_eq!(top(&data(Shape::Lines, vec![vec![Some(30.0)], vec![Some(40.0)]], None)), 50.0);
    }

    #[test]
    fn a_gap_splits_the_trace_and_the_newest_is_at_the_trailing_edge() {
        let d = data(Shape::Lines, vec![vec![Some(0.5), Some(1.0), None, Some(0.0)]], Some(1.0));
        let r = runs(&d, 100.0, 80.0, 4.0);
        assert_eq!(r[0].len(), 2, "two runs either side of the gap");
        assert_eq!(r[0][1], vec![(100.0, 80.0, 80.0)]);
        assert_eq!(r[0][0][1], (92.0, 0.0, 80.0), "full height two steps back");
    }

    #[test]
    fn stacked_rides_on_the_series_below_and_mirrored_splits_about_the_middle() {
        let s = data(Shape::Stacked, vec![vec![Some(0.25)], vec![Some(0.25)]], Some(1.0));
        let r = runs(&s, 10.0, 100.0, 4.0);
        assert_eq!(r[0][0][0], (10.0, 75.0, 100.0));
        assert_eq!(r[1][0][0], (10.0, 50.0, 75.0));
        let m = data(Shape::Mirrored, vec![vec![Some(1.0)], vec![Some(0.5)]], Some(1.0));
        let r = runs(&m, 10.0, 100.0, 4.0);
        assert_eq!(r[0][0][0], (10.0, 0.0, 50.0), "in above");
        assert_eq!(r[1][0][0], (10.0, 75.0, 50.0), "out below");
    }
}
