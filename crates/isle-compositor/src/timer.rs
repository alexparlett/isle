//! Timers: the Timer widget's control, the island's Timer activity while one runs, and its
//! end as the island's timer-end event; they run on with the dashboard closed and across a
//! restart (`p17-widgets.md` choice 15).

use std::time::{Duration, Instant};

use isle_shell::island::model::{Activity, ActivityKind, Event, EventKind, Form};
use isle_state::domains::Timer;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer as Wake};

use crate::state::{Isle, State};

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// `m:ss`, or `h:mm:ss` from an hour.
pub fn clock(ms: u64) -> String {
    let secs = ms.div_ceil(1000);
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

impl Isle {
    /// The length a Timer widget is set to, in milliseconds.
    pub fn timer_length(&self, id: &str) -> u64 {
        let minutes = match self.settings.widget.get(id).and_then(|w| w.get("minutes")) {
            Some(isle_settings::Value::Int(m)) => (*m).clamp(1, 180) as u64,
            _ => 5,
        };
        minutes * 60_000
    }

    pub fn timer_label(&self, id: &str) -> String {
        match self.settings.widget.get(id).and_then(|w| w.get("label")) {
            Some(isle_settings::Value::Str(l)) if !l.trim().is_empty() => l.trim().to_owned(),
            _ => "Timer".to_owned(),
        }
    }

    /// What is left of `id`'s timer, and whether it runs; none while idle.
    pub fn timer_left(&self, id: &str) -> Option<(u64, bool)> {
        let t = self.dashboard.memory.timers.get(id)?;
        match (t.ends_at, t.left_ms) {
            (Some(end), _) => Some((end.saturating_sub(now_ms()), true)),
            (None, Some(left)) => Some((left, false)),
            _ => None,
        }
    }

    /// The island's Timer activity for the running timer that ends first.
    fn timer_activity(&mut self) {
        let now = now_ms();
        let first = self
            .dashboard
            .memory
            .timers
            .iter()
            .filter_map(|(id, t)| t.ends_at.map(|end| (end, id.clone(), t.label.clone())))
            .min();
        match first {
            Some((end, _, label)) => {
                let left = clock(end.saturating_sub(now));
                let started = self
                    .island
                    .model
                    .activities()
                    .find(|a| a.kind == ActivityKind::Timer)
                    .map_or_else(Instant::now, |a| a.started);
                self.island.model.set_activity(
                    Activity {
                        kind: ActivityKind::Timer,
                        face: left.clone(),
                        title: label,
                        detail: left,
                        actions: Vec::new(),
                        app: None,
                        art: None,
                        started,
                    },
                    Instant::now(),
                );
            }
            None => self.island.model.end_activity(ActivityKind::Timer),
        }
        self.refresh_island();
    }
}

impl State {
    pub fn timer_start(&mut self, id: &str) {
        let length = self.isle.timer_length(id);
        let label = self.isle.timer_label(id);
        let timers = &mut self.isle.dashboard.memory.timers;
        let left = timers.get(id).and_then(|t| t.left_ms).unwrap_or(length);
        timers.insert(
            id.to_owned(),
            Timer {
                label,
                duration_ms: length,
                ends_at: Some(now_ms() + left),
                left_ms: None,
            },
        );
        self.timers_changed();
    }

    pub fn timer_pause(&mut self, id: &str) {
        if let Some(t) = self.isle.dashboard.memory.timers.get_mut(id)
            && let Some(end) = t.ends_at.take()
        {
            t.left_ms = Some(end.saturating_sub(now_ms()));
        }
        self.timers_changed();
    }

    pub fn timer_reset(&mut self, id: &str) {
        self.isle.dashboard.memory.timers.remove(id);
        self.timers_changed();
    }

    /// A preset chosen: the length set, and the timer reset to it.
    pub fn timer_preset(&mut self, id: &str, minutes: i64) {
        self.isle
            .write_setting(&format!("widget[{id}].minutes"), isle_settings::Value::Int(minutes));
        self.isle.dashboard.memory.timers.remove(id);
        self.timers_changed();
    }

    fn timers_changed(&mut self) {
        self.isle.save_widgets();
        self.isle.timer_activity();
        self.arm_timer_tick();
        self.put_dashboard_frame();
    }

    /// Ends every timer whose time has come; at the start too, so one that ended while Isle
    /// was away says so.
    pub fn timers_due(&mut self) {
        let now = now_ms();
        let done: Vec<(String, String)> = self
            .isle
            .dashboard
            .memory
            .timers
            .iter()
            .filter(|(_, t)| t.ends_at.is_some_and(|end| end <= now))
            .map(|(id, t)| (id.clone(), t.label.clone()))
            .collect();
        for (id, label) in &done {
            self.isle.dashboard.memory.timers.remove(id);
            self.isle.island.model.post(
                Event {
                    kind: EventKind::TimerDone,
                    form: Form::Line {
                        symbol: "timer".into(),
                        fill: false,
                        text: format!("{label} done"),
                    },
                    output: None,
                    app: None,
                    urgent: true,
                },
                Instant::now(),
            );
        }
        if !done.is_empty() {
            self.isle.save_widgets();
        }
        self.isle.timer_activity();
        self.put_dashboard_frame();
    }

    /// Wakes each second while a timer runs, for the island's face and the widget.
    pub fn arm_timer_tick(&mut self) {
        let running = self.isle.dashboard.memory.timers.values().any(|t| t.ends_at.is_some());
        if !running || self.isle.dashboard.timer_tick.is_some() {
            return;
        }
        self.isle.dashboard.timer_tick = self
            .isle
            .loop_handle
            .insert_source(Wake::from_duration(Duration::from_millis(250)), |_, _, state| {
                state.timers_due();
                if state.isle.dashboard.memory.timers.values().any(|t| t.ends_at.is_some()) {
                    TimeoutAction::ToDuration(Duration::from_millis(250))
                } else {
                    state.isle.dashboard.timer_tick = None;
                    TimeoutAction::Drop
                }
            })
            .ok();
    }
}

#[cfg(test)]
mod tests {
    use super::clock;

    #[test]
    fn the_time_left_reads_as_a_clock() {
        assert_eq!(clock(300_000), "5:00");
        assert_eq!(clock(299_001), "5:00", "a countdown rounds up");
        assert_eq!(clock(298_999), "4:59");
        assert_eq!(clock(1), "0:01");
        assert_eq!(clock(3_725_000), "1:02:05");
    }
}
