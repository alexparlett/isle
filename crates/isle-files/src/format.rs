//! How Files writes dates, sizes and counts (`p1/structure/files.md` § Copy;
//! `surfaces/files.md` § List view).

use jiff::{Timestamp, Zoned, civil::Date, tz::TimeZone};

/// "Today 14:02", "Yesterday 09:15", "2 Oct 2026 at 09:15".
pub fn date(seconds: i64, now: &Zoned) -> String {
    let Ok(at) = Timestamp::from_second(seconds) else {
        return "--".into();
    };
    let at = at.to_zoned(now.time_zone().clone());
    date_in(&at, now.date())
}

fn date_in(at: &Zoned, today: Date) -> String {
    let time = at.strftime("%H:%M");
    let day = at.date();
    if day == today {
        format!("Today {time}")
    } else if today.yesterday().is_ok_and(|y| y == day) {
        format!("Yesterday {time}")
    } else {
        format!("{} at {time}", at.strftime("%-d %b %Y"))
    }
}

/// The time zone dates are shown in.
pub fn now() -> Zoned {
    Zoned::now().with_time_zone(TimeZone::system())
}

/// Bytes in powers of a thousand, as Finder: "4 KB", "1.2 GB"; "Zero bytes" for none.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["KB", "MB", "GB", "TB", "PB"];
    if bytes == 0 {
        return "Zero bytes".into();
    }
    if bytes < 1000 {
        return format!("{bytes} bytes");
    }
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 0;
    while value >= 999.95 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    if value < 10.0 && unit > 0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{} {}", value.round() as u64, UNITS[unit])
    }
}

/// "1 item", "41 items".
pub fn items(n: usize) -> String {
    if n == 1 { "1 item".into() } else { format!("{n} items") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_in_thousands() {
        assert_eq!(size(0), "Zero bytes");
        assert_eq!(size(512), "512 bytes");
        assert_eq!(size(4_096), "4 KB");
        assert_eq!(size(1_200_000_000), "1.2 GB");
        assert_eq!(size(999_999), "1.0 MB");
        assert_eq!(size(250_000_000), "250 MB");
    }

    #[test]
    fn dates_name_today_and_yesterday() {
        let now: Zoned = "2026-10-21T15:00:00[UTC]".parse().unwrap();
        let at = |s: &str| s.parse::<Timestamp>().unwrap().as_second();
        assert_eq!(date(at("2026-10-21T14:02:00Z"), &now), "Today 14:02");
        assert_eq!(date(at("2026-10-20T09:15:00Z"), &now), "Yesterday 09:15");
        assert_eq!(date(at("2026-10-02T09:15:00Z"), &now), "2 Oct 2026 at 09:15");
        assert_eq!(items(1), "1 item");
    }
}
