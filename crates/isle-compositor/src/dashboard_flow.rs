//! Where the dashboard's widgets are drawn while they move to their places on `motion-move`
//! (`surfaces/widgets.md` Motion): the soft push's neighbours, whose pieces of the surfaces the
//! compositor moves, and the ghost and a dropped widget landing, which the front surface draws.

use std::collections::HashMap;
use std::time::Instant;

use isle_shell::{
    dashboard::surface::{Frame, Lifted},
    transition::Motion,
};
use isle_tokens::Spring;

struct Moving {
    x: Motion,
    y: Motion,
    /// Its size in cells: a resize puts it at once.
    size: (i32, i32),
}

#[derive(Default)]
pub struct Flow {
    widgets: HashMap<String, Moving>,
    ghost: Option<[Motion; 4]>,
    /// The widget let go of, drawn lifted until it lands, and how lifted it still is.
    landing: Option<(String, Motion)>,
    lifted: Option<String>,
    last: Option<Instant>,
}

impl Flow {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn moving(&self) -> bool {
        self.widgets
            .values()
            .any(|m| !m.x.settled() || !m.y.settled())
            || self
                .ghost
                .as_ref()
                .is_some_and(|g| g.iter().any(|m| !m.settled()))
            || self.landing.is_some()
    }

    /// The ghost or a landing widget moves, which the front surface draws.
    pub fn redraws(&self) -> bool {
        self.ghost
            .as_ref()
            .is_some_and(|g| g.iter().any(|m| !m.settled()))
            || self.landing.is_some()
    }

    /// Each widget still on its way, by how far it is drawn from its place.
    pub fn offsets(&self) -> Vec<(String, f64, f64)> {
        self.widgets
            .iter()
            .filter(|(id, m)| {
                (!m.x.settled() || !m.y.settled())
                    && self.landing.as_ref().is_none_or(|(l, _)| l != *id)
            })
            .map(|(id, m)| {
                (
                    id.clone(),
                    m.x.value() - m.x.target(),
                    m.y.value() - m.y.target(),
                )
            })
            .collect()
    }

    pub fn step(&mut self, now: Instant) {
        let dt = self
            .last
            .map_or(0.0, |l| now.duration_since(l).as_secs_f64().min(0.1));
        self.last = Some(now);
        for m in self.widgets.values_mut() {
            m.x.step(now, dt);
            m.y.step(now, dt);
        }
        for m in self.ghost.iter_mut().flatten() {
            m.step(now, dt);
        }
        if let Some((_, lift)) = &mut self.landing {
            lift.step(now, dt);
        }
    }

    /// Aims every motion at `frame`'s places and puts the ghost and a landing widget where
    /// they are; with `animated` false everything is where it lands.
    pub fn apply(&mut self, frame: &mut Frame, animated: bool, spring: Spring) {
        if !animated {
            self.clear();
            return;
        }
        // A motion starting from rest counts its time from now.
        if !self.moving() {
            self.last = Some(Instant::now());
        }
        let lifted = frame.lifted.as_ref().map(|l| l.id.clone());
        if let Some(was) = self.lifted.take()
            && lifted.as_ref() != Some(&was)
            && frame.widgets.iter().any(|w| w.id == was)
        {
            let mut lift = Motion::at(1.0);
            lift.spring_to(0.0, spring);
            self.landing = Some((was, lift));
        }
        self.lifted = lifted.clone();
        self.widgets
            .retain(|id, _| frame.widgets.iter().any(|w| &w.id == id));
        for w in &frame.widgets {
            let (x, y, _, _) = frame.extent.rect(w.at);
            let size = (w.at.w, w.at.h);
            if let Some(l) = frame.lifted.as_ref().filter(|l| l.id == w.id) {
                // Under the pointer it follows 1:1, and lands from there.
                self.widgets.insert(
                    w.id.clone(),
                    Moving {
                        x: Motion::at(l.x as f64),
                        y: Motion::at(l.y as f64),
                        size,
                    },
                );
                continue;
            }
            let m = self.widgets.entry(w.id.clone()).or_insert_with(|| Moving {
                x: Motion::at(x as f64),
                y: Motion::at(y as f64),
                size,
            });
            if m.size != size {
                *m = Moving {
                    x: Motion::at(x as f64),
                    y: Motion::at(y as f64),
                    size,
                };
            }
            if m.x.target() != x as f64 || m.y.target() != y as f64 {
                m.x.spring_to(x as f64, spring);
                m.y.spring_to(y as f64, spring);
            }
        }
        if let Some((id, lift)) = &self.landing {
            match self
                .widgets
                .get(id)
                .filter(|m| !m.x.settled() || !m.y.settled())
            {
                Some(m) if lifted.is_none() => {
                    let (x, y) = (m.x.value() as f32, m.y.value() as f32);
                    frame.lifted = Some(Lifted {
                        id: id.clone(),
                        x,
                        y,
                        lift: lift.value().clamp(0.0, 1.0) as f32,
                    });
                }
                _ => self.landing = None,
            }
        }
        match frame.ghost.as_mut() {
            Some(ghost) => {
                let (x, y, w, h) = frame.extent.rect(ghost.at);
                let target = [x, y, w, h].map(|v| v as f64);
                let motions = self.ghost.get_or_insert_with(|| target.map(Motion::at));
                for (m, t) in motions.iter_mut().zip(target) {
                    if m.target() != t {
                        m.spring_to(t, spring);
                    }
                }
                if motions.iter().any(|m| !m.settled()) {
                    let [x, y, w, h] = motions.each_ref().map(|m| m.value() as f32);
                    ghost.drawn = Some((x, y, w, h));
                }
            }
            None => self.ghost = None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use isle_shell::dashboard::{
        grid::{Cells, Extent},
        surface::{Body, Ghost, WidgetView},
    };

    use super::*;

    fn frame(b: Cells) -> Frame {
        let view = |id: &str, at: Cells| WidgetView {
            id: id.into(),
            name: id.into(),
            at,
            body: Body::Title,
        };
        Frame {
            extent: Extent {
                cols: 20,
                rows: 10,
                x: 0.0,
                y: 0.0,
                cell: 72.0,
                gap: 16.0,
            },
            widgets: vec![view("a", Cells::new(0, 0, 2, 2)), view("b", b)],
            ..Default::default()
        }
    }

    fn spring() -> Spring {
        isle_tokens::Source::shipped()
            .theme(&Default::default())
            .unwrap()
            .spring_move()
    }

    fn settle(flow: &mut Flow, b: Cells, ghost: Option<Ghost>) -> Vec<Frame> {
        let mut frames = Vec::new();
        let mut now = Instant::now();
        for i in 0..200 {
            now += Duration::from_millis(7);
            flow.step(now);
            if i == 2 && ghost.is_none() && flow.landing.is_none() {
                let offsets = flow.offsets();
                assert_eq!(offsets.len(), 1);
                assert!(offsets[0].1 < 0.0 && offsets[0].1 > -88.0, "{offsets:?}");
            }
            let mut f = frame(b);
            f.ghost = ghost;
            flow.apply(&mut f, true, spring());
            frames.push(f);
            if !flow.moving() {
                return frames;
            }
        }
        panic!("never settled");
    }

    #[test]
    fn a_pushed_neighbour_flows_and_a_dropped_widget_lands_lifted() {
        let mut flow = Flow::default();
        flow.apply(&mut frame(Cells::new(2, 0, 2, 2)), true, spring());
        assert!(!flow.moving());

        // Pushed a cell along: drawn between its places, then at its new one.
        let mut f = frame(Cells::new(3, 0, 2, 2));
        flow.apply(&mut f, true, spring());
        assert!(flow.moving());
        settle(&mut flow, Cells::new(3, 0, 2, 2), None);
        let x1 = 3.0 * 88.0;
        assert!(flow.offsets().is_empty());

        // Held at the pointer, then let go: it lands lifted, its shadow going.
        let mut held = frame(Cells::new(3, 0, 2, 2));
        held.lifted = Some(Lifted {
            id: "b".into(),
            x: 600.0,
            y: 400.0,
            lift: 1.0,
        });
        flow.apply(&mut held, true, spring());
        let frames = settle(&mut flow, Cells::new(3, 0, 2, 2), None);
        let first = frames[2].lifted.as_ref().expect("landing");
        assert!(
            first.x < 600.0 && first.x > x1 && first.lift < 1.0,
            "{first:?}"
        );
        assert!(frames.last().unwrap().lifted.is_none());

        // The ghost moves cell to cell.
        let ghost = |x| {
            Some(Ghost {
                at: Cells::new(x, 4, 2, 2),
                refused: false,
                drawn: None,
            })
        };
        let mut f = frame(Cells::new(3, 0, 2, 2));
        f.ghost = ghost(0);
        flow.apply(&mut f, true, spring());
        let frames = settle(&mut flow, Cells::new(3, 0, 2, 2), ghost(1));
        let gx = frames[2].ghost.unwrap().drawn.expect("moving").0;
        assert!(gx > 0.0 && gx < 88.0, "{gx}");

        // Without motion everything is where it lands.
        let mut f = frame(Cells::new(5, 0, 2, 2));
        flow.apply(&mut f, false, spring());
        assert!(flow.offsets().is_empty() && !flow.moving());
    }
}
