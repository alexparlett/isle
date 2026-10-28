//! Weather for the dashboard: Open-Meteo's forecast fetched by Isle on the worker
//! (`p17-widgets.md` choice 13; v1 D31).

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// What a Weather widget draws, kept between showings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Forecast {
    pub place: String,
    /// What the place setting was when this was fetched.
    pub asked: String,
    pub now: f64,
    pub code: u8,
    pub day: bool,
    pub high: f64,
    pub low: f64,
    /// The next six hours: its hour as shown, WMO code, temperature.
    pub hours: Vec<(String, u8, f64)>,
    /// The next five days: weekday, WMO code, low, high.
    pub days: Vec<(String, u8, f64, f64)>,
    pub fahrenheit: bool,
    /// Unix seconds it was fetched.
    pub at: u64,
}

/// A WMO weather code's glyph, by day or night, and its words.
pub fn condition(code: u8, day: bool) -> (&'static str, &'static str) {
    match code {
        0 => (if day { "sun" } else { "moon" }, "Clear"),
        1 => (if day { "sun" } else { "moon" }, "Mostly clear"),
        2 => (if day { "cloud-sun" } else { "cloud-moon" }, "Partly cloudy"),
        3 => ("cloud", "Cloudy"),
        45 | 48 => ("cloud-fog", "Fog"),
        51..=57 => ("cloud-rain", "Drizzle"),
        61..=67 | 80..=82 => ("cloud-rain", "Rain"),
        71..=77 | 85 | 86 => ("cloud-snow", "Snow"),
        95..=99 => ("cloud-lightning", "Thunderstorms"),
        _ => ("cloud", "Cloudy"),
    }
}

/// Whether the region measures in Fahrenheit: the United States and a few others.
pub fn fahrenheit() -> bool {
    let locale = ["LC_ALL", "LC_MEASUREMENT", "LANG"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();
    ["_US", "_LR", "_MM", "_BS", "_BZ", "_KY", "_PW"]
        .iter()
        .any(|r| locale.contains(r))
}

#[derive(Deserialize)]
struct Response {
    current: Current,
    hourly: Hourly,
    daily: Daily,
}

#[derive(Deserialize)]
struct Current {
    time: String,
    temperature_2m: f64,
    weather_code: u8,
    is_day: u8,
}

#[derive(Deserialize)]
struct Hourly {
    time: Vec<String>,
    temperature_2m: Vec<f64>,
    weather_code: Vec<u8>,
}

#[derive(Deserialize)]
struct Daily {
    time: Vec<String>,
    weather_code: Vec<u8>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
}

/// Open-Meteo's answer as a forecast; times are the place's own (`timezone=auto`).
pub fn parse(json: &str, place: &str, asked: &str, fahrenheit: bool, at: u64, hours_24: bool) -> Option<Forecast> {
    let r: Response = serde_json::from_str(json).ok()?;
    let now_hour = r.current.time.get(..13)?;
    let start = r.hourly.time.iter().position(|t| t.get(..13) == Some(now_hour))?;
    let hours = (start + 1..start + 7)
        .filter_map(|i| {
            let t = r.hourly.time.get(i)?;
            let h: u32 = t.get(11..13)?.parse().ok()?;
            let label = if hours_24 {
                format!("{h:02}")
            } else {
                let (h12, m) = match h {
                    0 => (12, "AM"),
                    12 => (12, "PM"),
                    h if h > 12 => (h - 12, "PM"),
                    h => (h, "AM"),
                };
                format!("{h12} {m}")
            };
            Some((label, *r.hourly.weather_code.get(i)?, *r.hourly.temperature_2m.get(i)?))
        })
        .collect();
    let days = (1..6)
        .filter_map(|i| {
            let date: jiff::civil::Date = r.daily.time.get(i)?.parse().ok()?;
            let weekday = date.strftime("%a").to_string();
            Some((
                weekday,
                *r.daily.weather_code.get(i)?,
                *r.daily.temperature_2m_min.get(i)?,
                *r.daily.temperature_2m_max.get(i)?,
            ))
        })
        .collect();
    Some(Forecast {
        place: place.to_owned(),
        asked: asked.to_owned(),
        now: r.current.temperature_2m,
        code: r.current.weather_code,
        day: r.current.is_day == 1,
        high: *r.daily.temperature_2m_max.first()?,
        low: *r.daily.temperature_2m_min.first()?,
        hours,
        days,
        fahrenheit,
        at,
    })
}

fn client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Isle")
        .build()
        .ok()
}

/// A place's name and coordinates from Open-Meteo's geocoding.
pub async fn find(name: &str) -> Result<(String, f64, f64), String> {
    #[derive(Deserialize)]
    struct Found {
        results: Option<Vec<Place>>,
    }
    #[derive(Deserialize)]
    struct Place {
        name: String,
        latitude: f64,
        longitude: f64,
    }
    let client = client().ok_or("No network client")?;
    let found: Found = client
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&[("name", name), ("count", "1"), ("format", "json")])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let place = found
        .results
        .and_then(|r| r.into_iter().next())
        .ok_or_else(|| format!("No place called {name}"))?;
    Ok((place.name, place.latitude, place.longitude))
}

/// The forecast's JSON for a place.
pub async fn fetch(latitude: f64, longitude: f64, fahrenheit: bool) -> Result<String, String> {
    let client = client().ok_or("No network client")?;
    let (lat, lon) = (latitude.to_string(), longitude.to_string());
    let mut query = vec![
        ("latitude", lat.as_str()),
        ("longitude", lon.as_str()),
        ("current", "temperature_2m,weather_code,is_day"),
        ("hourly", "temperature_2m,weather_code"),
        ("daily", "weather_code,temperature_2m_max,temperature_2m_min"),
        ("timezone", "auto"),
        ("forecast_days", "7"),
    ];
    if fahrenheit {
        query.push(("temperature_unit", "fahrenheit"));
    }
    client
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&query)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANSWER: &str = r#"{
        "current": {"time": "2026-10-28T14:15", "temperature_2m": 12.4, "weather_code": 2, "is_day": 1},
        "hourly": {
            "time": ["2026-10-28T13:00","2026-10-28T14:00","2026-10-28T15:00","2026-10-28T16:00","2026-10-28T17:00","2026-10-28T18:00","2026-10-28T19:00","2026-10-28T20:00"],
            "temperature_2m": [12.0, 12.4, 12.1, 11.5, 10.2, 9.0, 8.1, 7.7],
            "weather_code": [2, 2, 3, 61, 61, 3, 0, 0]
        },
        "daily": {
            "time": ["2026-10-28","2026-10-29","2026-10-30","2026-10-31","2026-11-01","2026-11-02","2026-11-03"],
            "weather_code": [2, 61, 3, 0, 71, 95, 1],
            "temperature_2m_max": [13.0, 11.0, 10.5, 12.2, 4.0, 9.0, 10.0],
            "temperature_2m_min": [6.0, 5.5, 4.0, 3.3, -1.0, 2.0, 3.0]
        }
    }"#;

    /// Reaches Open-Meteo itself: `cargo test -- --ignored live_forecast`.
    #[tokio::test]
    #[ignore]
    async fn live_forecast() {
        let (name, lat, lon) = find("London").await.unwrap();
        let json = fetch(lat, lon, false).await.unwrap();
        assert!(parse(&json, &name, "", false, 1, true).is_some(), "{json}");
    }

    #[test]
    fn an_answer_becomes_now_the_next_hours_and_days() {
        let f = parse(ANSWER, "London", "", false, 1, true).unwrap();
        assert_eq!((f.now, f.code, f.day, f.high, f.low), (12.4, 2, true, 13.0, 6.0));
        assert_eq!(f.hours.len(), 6);
        assert_eq!(f.hours[0], ("15".to_owned(), 3, 12.1));
        assert_eq!(f.days.len(), 5);
        assert_eq!(f.days[0].0, "Thu");
        let f = parse(ANSWER, "London", "", false, 1, false).unwrap();
        assert_eq!(f.hours[0].0, "3 PM");
        assert_eq!(condition(f.code, true), ("cloud-sun", "Partly cloudy"));
        assert_eq!(condition(0, false).0, "moon");
        assert!(parse("{}", "x", "", false, 1, true).is_none());
    }
}
