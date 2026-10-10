//! Sunrise and sunset for where the clock says the machine is: the time zone's own
//! coordinates from tzdata, no location service.

use std::path::Path;

const ZONE_TABLES: &[&str] = &["/usr/share/zoneinfo/zone1970.tab", "/usr/share/zoneinfo/zone.tab"];

/// The latitude and longitude, in degrees north and east, tzdata gives `zone`.
pub fn coordinates(zone: &str) -> Option<(f64, f64)> {
    ZONE_TABLES
        .iter()
        .find_map(|table| coordinates_in(Path::new(table), zone))
}

fn coordinates_in(table: &Path, zone: &str) -> Option<(f64, f64)> {
    let text = std::fs::read_to_string(table).ok()?;
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| {
            let mut fields = line.split('\t');
            let (_, place, name) = (fields.next()?, fields.next()?, fields.next()?);
            (name == zone).then(|| iso6709(place)).flatten()
        })
}

/// `+DDMM[SS]+DDDMM[SS]`, latitude then longitude.
fn iso6709(text: &str) -> Option<(f64, f64)> {
    let split = text[1..].find(['+', '-'])? + 1;
    let (lat, lon) = text.split_at(split);
    Some((degrees(lat, 2)?, degrees(lon, 3)?))
}

fn degrees(text: &str, whole: usize) -> Option<f64> {
    let sign = if text.starts_with('-') { -1.0 } else { 1.0 };
    let digits = &text[1..];
    let part = |from: usize, to: usize| digits.get(from..to).and_then(|d| d.parse::<f64>().ok());
    let d = part(0, whole)?;
    let m = part(whole, whole + 2)?;
    let s = part(whole + 2, whole + 4).unwrap_or(0.0);
    Some(sign * (d + m / 60.0 + s / 3600.0))
}

/// The sun on one day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sun {
    /// It rises and sets, at these Unix times.
    RisesAndSets { rise: i64, set: i64 },
    /// It never sets (polar day).
    Up,
    /// It never rises (polar night).
    Down,
}

/// The sunrise and sunset nearest `noon`, a Unix time near the middle of the day, at
/// latitude `lat` and longitude `lon` (degrees, north and east positive). The sunrise
/// equation as NOAA gives it, to within a minute or two at mid latitudes.
pub fn sun(noon: i64, lat: f64, lon: f64) -> Sun {
    let julian = noon as f64 / 86_400.0 + 2_440_587.5;
    let n = (julian - 2_451_545.0 + 0.0008).round();
    let mean_noon = n - lon / 360.0;
    let anomaly = (357.5291 + 0.985_600_28 * mean_noon).rem_euclid(360.0);
    let m = anomaly.to_radians();
    let centre = 1.9148 * m.sin() + 0.02 * (2.0 * m).sin() + 0.0003 * (3.0 * m).sin();
    let ecliptic = (anomaly + centre + 180.0 + 102.9372).rem_euclid(360.0).to_radians();
    let transit = 2_451_545.0 + mean_noon + 0.0053 * m.sin() - 0.0069 * (2.0 * ecliptic).sin();
    let declination = (ecliptic.sin() * 23.4397_f64.to_radians().sin()).asin();
    let phi = lat.to_radians();
    let cos_hour = ((-0.833_f64).to_radians().sin() - phi.sin() * declination.sin())
        / (phi.cos() * declination.cos());
    if cos_hour < -1.0 {
        return Sun::Up;
    }
    if cos_hour > 1.0 {
        return Sun::Down;
    }
    let hour = cos_hour.acos().to_degrees() / 360.0;
    let unix = |j: f64| ((j - 2_440_587.5) * 86_400.0).round() as i64;
    Sun::RisesAndSets {
        rise: unix(transit - hour),
        set: unix(transit + hour),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-06-21 12:00 UTC.
    const MIDSUMMER: i64 = 1_782_043_200;
    /// 2026-12-21 17:00 UTC, noon in New York.
    const MIDWINTER_NY: i64 = 1_797_872_400;

    fn near(got: i64, want: i64) {
        assert!((got - want).abs() <= 180, "{got} is {} s from {want}", got - want);
    }

    #[test]
    fn london_at_midsummer() {
        let Sun::RisesAndSets { rise, set } = sun(MIDSUMMER, 51.5083, -0.1253) else {
            panic!("the sun rises in London");
        };
        // 03:43 and 20:21 UTC.
        near(rise, MIDSUMMER - (8 * 3600 + 17 * 60));
        near(set, MIDSUMMER + 8 * 3600 + 21 * 60);
    }

    #[test]
    fn new_york_at_midwinter() {
        let Sun::RisesAndSets { rise, set } = sun(MIDWINTER_NY, 40.7142, -74.0064) else {
            panic!("the sun rises in New York");
        };
        // 12:16 and 21:32 UTC.
        near(rise, MIDWINTER_NY - (4 * 3600 + 44 * 60));
        near(set, MIDWINTER_NY + 4 * 3600 + 32 * 60);
    }

    #[test]
    fn polar_day_and_night() {
        assert_eq!(sun(MIDSUMMER, 78.2, 15.6), Sun::Up);
        assert_eq!(sun(MIDWINTER_NY, 78.2, 15.6), Sun::Down);
    }

    #[test]
    fn coordinates_parse_both_precisions() {
        let (lat, lon) = iso6709("+513030-0000731").unwrap();
        assert!((lat - 51.5083).abs() < 0.001 && (lon + 0.1253).abs() < 0.001);
        let (lat, lon) = iso6709("+4734-05243").unwrap();
        assert!((lat - 47.5667).abs() < 0.001 && (lon + 52.7167).abs() < 0.001);
    }

    #[test]
    fn a_zone_from_tzdata() {
        if Path::new(ZONE_TABLES[0]).exists() {
            let (lat, _) = coordinates("Europe/London").unwrap();
            assert!((lat - 51.5).abs() < 0.1);
            assert_eq!(coordinates("Nowhere/Atlantis"), None);
        }
    }
}
