//! The footers' figures over time, one point a sample, with an empty point for each sample
//! missed while Monitor was not shown, so the graphs show a gap rather than a line across it.

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use crate::sampler::Sample;

/// Enough points for the widest well: 520 points at a step of 4.
pub const KEEP: usize = 132;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Point {
    pub cpu_user: f32,
    pub cpu_system: f32,
    pub pressure: f32,
    pub cpu_power: Option<f32>,
    /// Each GPU's board power and busy share, in the sample's GPU order.
    pub gpu_power: Vec<Option<f32>>,
    pub gpu_busy: Vec<Option<f32>>,
    /// Every disk together, a second.
    pub reads: f32,
    pub writes: f32,
    pub read_bytes: f32,
    pub written_bytes: f32,
    /// Every interface but loopback, a second; none on the first sample after a gap.
    pub packets_in: Option<f32>,
    pub packets_out: Option<f32>,
    pub received: Option<f32>,
    pub sent: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
    pub points: VecDeque<Option<Point>>,
    pub times: VecDeque<String>,
    last: Option<(Instant, [u64; 4])>,
}

impl History {
    /// Adds `sample`, after an empty point for each interval of `every` missed since the last.
    pub fn push(&mut self, sample: &Sample, every: Duration) {
        let now = sample.at;
        let full = &sample.full;
        let gap = self.last.map(|(at, _)| now.duration_since(at));
        if let Some(gap) = gap {
            let missed = (gap.as_secs_f64() / every.as_secs_f64()).round() as usize;
            for _ in 1..missed.min(KEEP) {
                self.points.push_back(None);
                self.times.push_back(String::new());
            }
        }
        let fresh = gap.is_some_and(|g| g.as_secs_f64() < every.as_secs_f64() * 1.5);
        let rate = |i: usize| -> Option<f32> {
            let (at, last) = self.last.filter(|_| fresh)?;
            let secs = now.duration_since(at).as_secs_f64().max(1e-3);
            Some((full.interfaces[i].saturating_sub(last[i]) as f64 / secs) as f32)
        };
        let point = Point {
            cpu_user: full.cpu.user,
            cpu_system: full.cpu.system,
            pressure: full.memory.pressure,
            cpu_power: full.cpu_power.map(|(w, _)| w),
            gpu_power: full.gpus.iter().map(|g| g.power).collect(),
            gpu_busy: full.gpus.iter().map(|g| g.busy).collect(),
            reads: full.disks.iter().map(|d| d.reads).sum(),
            writes: full.disks.iter().map(|d| d.writes).sum(),
            read_bytes: full.disks.iter().map(|d| d.read_bytes as f32).sum(),
            written_bytes: full.disks.iter().map(|d| d.written_bytes as f32).sum(),
            packets_in: rate(1),
            packets_out: rate(3),
            received: rate(0),
            sent: rate(2),
        };
        self.points.push_back(Some(point));
        self.times.push_back(clock());
        while self.points.len() > KEEP {
            self.points.pop_front();
            self.times.pop_front();
        }
        self.last = Some((now, full.interfaces));
    }

    pub fn latest(&self) -> Option<&Point> {
        self.points.back().and_then(|p| p.as_ref())
    }

    /// One figure of every point, oldest first.
    pub fn series(&self, f: impl Fn(&Point) -> Option<f32>) -> Vec<Option<f32>> {
        self.points.iter().map(|p| p.as_ref().and_then(&f)).collect()
    }
}

/// The time now as the hover writes it.
fn clock() -> String {
    jiff::Zoned::now().strftime("%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn sample(at: Instant, received: u64) -> Sample {
        let mut full = isle_sysinfo::monitor::Full::default();
        full.interfaces = [received, 0, 0, 0];
        Sample { at, full: Arc::new(full), extras: Arc::default() }
    }

    #[test]
    fn missed_samples_leave_a_gap_and_rates_wait_for_a_fresh_pair() {
        let every = Duration::from_secs(2);
        let t0 = Instant::now();
        let mut h = History::default();
        h.push(&sample(t0, 1000), every);
        assert_eq!(h.latest().unwrap().received, None, "the first has nothing to compare with");
        h.push(&sample(t0 + every, 5000), every);
        assert_eq!(h.latest().unwrap().received, Some(2000.0));
        // Hidden for ten seconds: four samples missed.
        h.push(&sample(t0 + every * 6, 9000), every);
        assert_eq!(h.points.len(), 2 + 4 + 1);
        assert!(h.points.range(2..6).all(|p| p.is_none()));
        assert_eq!(h.latest().unwrap().received, None, "no rate across a gap");
    }
}
