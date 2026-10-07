//! Notification Centre: the island's second panel, opened from the clock, its chord, the
//! banners' overflow and the summary; its notifications stacked by app (`p8-services.md`
//! choice 22).

use isle_shell::island::centre::{CentreCard, CentreFrame, Day, Group, Month};
use jiff::{Zoned, civil::Weekday};

use crate::{
    notifications::Entry,
    state::{Isle, State},
};

/// When a notification arrived, as the centre reads it.
fn when(at: u64, now: &Zoned) -> String {
    let Ok(stamp) = jiff::Timestamp::from_second(at as i64) else {
        return String::new();
    };
    let then = stamp.to_zoned(now.time_zone().clone());
    let minutes = (now.timestamp().as_second() - at as i64) / 60;
    if minutes < 1 {
        "Now".into()
    } else if minutes < 60 {
        format!("{minutes}m ago")
    } else if then.date() == now.date() {
        then.strftime("%H:%M").to_string()
    } else if then.date() == now.date().yesterday().unwrap_or(now.date()) {
        then.strftime("Yesterday %H:%M").to_string()
    } else {
        then.strftime("%-d %b").to_string()
    }
}

fn month(now: &Zoned) -> Month {
    let today = now.date();
    let first = today.first_of_month();
    let back = first.weekday().to_monday_zero_offset() as i64;
    let start = first
        .checked_sub(jiff::Span::new().days(back))
        .unwrap_or(first);
    let days = (0..42)
        .map(|i| {
            let d = start
                .checked_add(jiff::Span::new().days(i))
                .unwrap_or(start);
            Day {
                number: d.day() as u8,
                this_month: d.month() == today.month(),
                today: d == today,
            }
        })
        .collect();
    let weekdays = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ]
    .iter()
    .map(|w| format!("{w:?}")[..1].to_owned())
    .collect();
    Month {
        title: now.strftime("%B %Y").to_string(),
        weekdays,
        days,
    }
}

impl Isle {
    pub fn centre_open(&self) -> bool {
        self.island.panel.as_ref().is_some_and(|p| p.centre)
    }

    fn groups(&self, entries: &[&Entry], now: &Zoned) -> Vec<Group> {
        let mut groups: Vec<Group> = Vec::new();
        // Newest first, so each app's group is in the order its newest arrived.
        for entry in entries.iter().rev() {
            let app = self.notification_app(&entry.n);
            let card = CentreCard {
                id: entry.n.id,
                summary: entry.n.summary.clone(),
                body: crate::banners::strip_markup(&entry.n.body),
                time: when(entry.n.at, now),
                image: entry.n.image.clone(),
                live: entry.live,
            };
            match groups.iter_mut().find(|g| g.app == app) {
                Some(group) => group.cards.push(card),
                None => {
                    let name = self.app_name(&app);
                    groups.push(Group {
                        app_name: if name.is_empty() {
                            entry.n.app_name.clone()
                        } else {
                            name
                        },
                        icon: self.app_icon(&app, 32),
                        open: self.notifications.expanded.contains(&app),
                        app,
                        cards: vec![card],
                    });
                }
            }
        }
        groups
    }

    pub fn centre_frame(&self) -> CentreFrame {
        let now = Zoned::now();
        let entries = &self.notifications.entries;
        let live: Vec<&Entry> = entries
            .iter()
            .filter(|e| e.live && e.fate != crate::notifications::Fate::Transient)
            .collect();
        let records: Vec<&Entry> = entries.iter().filter(|e| !e.live).collect();
        CentreFrame {
            date: now.strftime("%A %-d %B").to_string(),
            month: month(&now),
            groups: self.groups(&live, &now),
            earlier: self.groups(&records, &now),
            clear_armed: self.notifications.clear_armed,
        }
    }
}

impl State {
    /// Opens the centre on the main screen, or closes it if it is open there.
    pub fn open_notification_centre(&mut self, _focused: Option<u32>) {
        let Some(output) = self.isle.main_output() else {
            return;
        };
        self.open_centre_on(&output);
    }

    pub fn open_centre_on(&mut self, output: &smithay::output::Output) {
        if self
            .isle
            .island
            .panel
            .as_ref()
            .is_some_and(|p| p.centre && p.output == output.name())
        {
            self.close_control_centre();
            return;
        }
        self.open_control_centre(output, true, None);
        if let Some(panel) = self.isle.island.panel.as_mut() {
            panel.centre = true;
            panel.page = None;
        }
        // While the centre is open no banner shows anywhere: arrivals go into it.
        self.isle.banners.model.clear();
        self.isle.notifications.clear_armed = false;
        self.isle.refresh_island();
    }

    pub fn centre_action(&mut self, row: &str) {
        if row != "clear-all" {
            self.isle.notifications.clear_armed = false;
        }
        if let Some(id) = row.strip_prefix("open:").and_then(|i| i.parse().ok()) {
            self.close_control_centre();
            self.notification_invoke(id, "default");
        } else if let Some(id) = row.strip_prefix("close:").and_then(|i| i.parse().ok()) {
            self.notification_close(id);
        } else if let Some(app) = row.strip_prefix("expand:") {
            if !self.isle.notifications.expanded.remove(app) {
                self.isle.notifications.expanded.insert(app.to_owned());
            }
        } else if let Some(app) = row.strip_prefix("clear-app:") {
            let ids: Vec<u32> = self
                .isle
                .notifications
                .entries
                .iter()
                .filter(|e| self.isle.notification_app(&e.n) == app)
                .map(|e| e.n.id)
                .collect();
            for id in ids {
                self.notification_close(id);
            }
        } else if row == "clear-all" {
            // Two steps: the first asks, the second clears.
            if self.isle.notifications.clear_armed {
                self.isle.notifications.clear_armed = false;
                self.notifications_clear();
            } else {
                self.isle.notifications.clear_armed = true;
            }
        }
        self.isle.refresh_island();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_month_is_six_weeks_from_the_monday_before_the_first() {
        let now: Zoned = "2026-10-07T12:00[Europe/London]".parse().unwrap();
        let m = month(&now);
        assert_eq!(m.days.len(), 42);
        // 1 October 2026 is a Thursday: the grid starts on Monday 28 September.
        assert_eq!((m.days[0].number, m.days[0].this_month), (28, false));
        assert_eq!(m.days[3].number, 1);
        assert!(
            m.days
                .iter()
                .find(|d| d.today)
                .is_some_and(|d| d.number == 7)
        );
        assert_eq!(m.title, "October 2026");
        assert_eq!(m.weekdays[0], "M");
    }

    #[test]
    fn times_read_as_the_centre_says() {
        let now: Zoned = "2026-10-07T12:00[Europe/London]".parse().unwrap();
        let at = |s: &str| s.parse::<Zoned>().unwrap().timestamp().as_second() as u64;
        assert_eq!(when(at("2026-10-07T11:59:40[Europe/London]"), &now), "Now");
        assert_eq!(when(at("2026-10-07T11:45[Europe/London]"), &now), "15m ago");
        assert_eq!(
            when(at("2026-10-06T18:04[Europe/London]"), &now),
            "Yesterday 18:04"
        );
        assert_eq!(when(at("2026-10-03T09:00[Europe/London]"), &now), "3 Oct");
    }
}
