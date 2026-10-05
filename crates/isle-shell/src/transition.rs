//! Open and close drawn by the compositor on a surface's texture (B7; shell.md rule 5).

use std::time::{Duration, Instant};

use isle_tokens::{Spring, Theme};

/// A value moving toward its target on a spring, or as a fade of fixed length.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    value: f64,
    velocity: f64,
    target: f64,
    drive: Drive,
}

#[derive(Clone, Copy, Debug)]
enum Drive {
    Spring(Spring),
    /// From `from` at `start`, reaching the target after `length`.
    Fade {
        from: f64,
        start: Instant,
        length: Duration,
    },
}

impl Motion {
    pub fn at(value: f64) -> Self {
        Self {
            value,
            velocity: 0.0,
            target: value,
            drive: Drive::Spring(Spring {
                stiffness: 1.0,
                damping: 1.0,
            }),
        }
    }

    /// Moves toward `target` on `spring` from where it is, keeping its velocity.
    pub fn spring_to(&mut self, target: f64, spring: Spring) {
        self.target = target;
        self.drive = Drive::Spring(spring);
    }

    /// Moves toward `target` in a straight line over `length`.
    pub fn fade_to(&mut self, target: f64, length: Duration) {
        self.target = target;
        self.drive = Drive::Fade {
            from: self.value,
            start: Instant::now(),
            length,
        };
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn target(&self) -> f64 {
        self.target
    }

    pub fn settled(&self) -> bool {
        (self.value - self.target).abs() < 1e-3 && self.velocity.abs() < 1e-2
    }

    /// Advances by `dt` seconds, ending at `now` for a fade.
    pub fn step(&mut self, now: Instant, dt: f64) {
        match self.drive {
            Drive::Spring(spring) => {
                // Semi-implicit Euler in small steps stays stable at these stiffnesses.
                let steps = (dt / 0.001).ceil().max(1.0) as usize;
                let h = dt / steps as f64;
                for _ in 0..steps {
                    let force = -(spring.stiffness as f64) * (self.value - self.target)
                        - spring.damping as f64 * self.velocity;
                    self.velocity += force * h;
                    self.value += self.velocity * h;
                }
            }
            Drive::Fade {
                from,
                start,
                length,
            } => {
                let t = (now.duration_since(start).as_secs_f64() / length.as_secs_f64().max(1e-6))
                    .min(1.0);
                self.value = from + (self.target - from) * t;
                self.velocity = if t >= 1.0 { 0.0 } else { 1.0 };
            }
        }
        if self.settled() {
            self.value = self.target;
            self.velocity = 0.0;
        }
    }
}

/// Where a surface is in its open or close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Opening,
    Open,
    Closing,
    Closed,
}

#[derive(Clone, Copy, Debug)]
pub struct Transition {
    pub phase: Phase,
    opacity: Motion,
    scale: Motion,
    last: Instant,
}

impl Default for Transition {
    fn default() -> Self {
        Self {
            phase: Phase::Closed,
            opacity: Motion::at(0.0),
            scale: Motion::at(1.0),
            last: Instant::now(),
        }
    }
}

/// How shell surfaces move: the theme's springs, a fade under Reduce Motion, or nothing.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    open: Spring,
    exit: Spring,
    from_scale: f64,
    fade: Option<Duration>,
    animated: bool,
}

impl Style {
    pub fn new(theme: &Theme, animated: bool) -> Self {
        Self {
            open: theme.spring_open(),
            exit: theme.spring_exit(),
            from_scale: theme.tokens.motion_panel_scale as f64,
            fade: theme
                .appearance
                .reduce_motion
                .then(|| Theme::millis(theme.tokens.motion_reduced_fade_ms)),
            animated,
        }
    }
}

impl Transition {
    pub fn open(&mut self, style: &Style) {
        if matches!(self.phase, Phase::Closed) {
            self.opacity = Motion::at(0.0);
            self.scale = Motion::at(style.from_scale);
        }
        self.phase = Phase::Opening;
        self.aim(style, 1.0, 1.0, style.open);
    }

    pub fn close(&mut self, style: &Style) {
        self.phase = Phase::Closing;
        self.aim(style, 0.0, style.from_scale, style.exit);
    }

    fn aim(&mut self, style: &Style, opacity: f64, scale: f64, spring: Spring) {
        let now = Instant::now();
        self.last = now;
        self.opacity.target = opacity;
        self.scale.target = scale;
        match (style.animated, style.fade) {
            (false, _) => {
                self.opacity = Motion::at(opacity);
                self.scale = Motion::at(scale);
            }
            // Reduce Motion: a fade in place, no scale.
            (true, Some(length)) => {
                self.opacity.drive = Drive::Fade {
                    from: self.opacity.value,
                    start: now,
                    length,
                };
                self.scale = Motion::at(1.0);
            }
            (true, None) => {
                self.opacity.drive = Drive::Spring(spring);
                self.scale.drive = Drive::Spring(spring);
            }
        }
        self.settle_phase();
    }

    /// Advances to `now`; true while it still moves.
    pub fn step(&mut self, now: Instant) -> bool {
        let dt = now.duration_since(self.last).as_secs_f64().min(0.1);
        self.last = now;
        if matches!(self.phase, Phase::Opening | Phase::Closing) {
            self.opacity.step(now, dt);
            self.scale.step(now, dt);
            self.settle_phase();
        }
        self.moving()
    }

    fn settle_phase(&mut self) {
        if self.opacity.settled() && self.scale.settled() {
            self.phase = match self.phase {
                Phase::Opening => Phase::Open,
                Phase::Closing => Phase::Closed,
                other => other,
            };
        }
    }

    pub fn moving(&self) -> bool {
        matches!(self.phase, Phase::Opening | Phase::Closing)
    }

    pub fn opacity(&self) -> f32 {
        self.opacity.value.clamp(0.0, 1.0) as f32
    }

    pub fn scale(&self) -> f64 {
        self.scale.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme(reduce_motion: bool) -> Theme {
        isle_tokens::Source::shipped()
            .theme(&isle_tokens::Appearance {
                reduce_motion,
                ..Default::default()
            })
            .unwrap()
    }

    fn run(t: &mut Transition) -> Duration {
        let start = Instant::now();
        let mut now = start;
        while t.moving() {
            now += Duration::from_millis(7);
            t.step(now);
            assert!(now - start < Duration::from_secs(2), "never settled");
        }
        now - start
    }

    #[test]
    fn open_and_close_settle_quickly_and_exits_are_quicker() {
        let style = Style::new(&theme(false), true);
        let mut t = Transition::default();
        t.open(&style);
        assert!(t.opacity() < 0.01 && (t.scale() - 0.96).abs() < 1e-6);
        let opening = run(&mut t);
        assert_eq!(t.phase, Phase::Open);
        assert_eq!((t.opacity(), t.scale()), (1.0, 1.0));
        t.close(&style);
        let closing = run(&mut t);
        assert_eq!(t.phase, Phase::Closed);
        assert!(closing < opening, "{closing:?} < {opening:?}");
        assert!(opening < Duration::from_millis(600), "{opening:?}");
    }

    #[test]
    fn reduce_motion_fades_in_place_and_no_transitions_snaps() {
        let style = Style::new(&theme(true), true);
        let mut t = Transition::default();
        t.open(&style);
        assert_eq!(t.scale(), 1.0);
        let took = run(&mut t);
        assert!(took >= Duration::from_millis(120) && took < Duration::from_millis(140));

        let style = Style::new(&theme(false), false);
        let mut t = Transition::default();
        t.open(&style);
        assert!(!t.moving());
        assert_eq!((t.opacity(), t.scale(), t.phase), (1.0, 1.0, Phase::Open));
    }
}
