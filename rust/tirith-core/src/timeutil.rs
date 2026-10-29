//! ISO-8601 helpers (`now_iso` / `parse_iso` of `home.py`).

use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};

/// UTC time as ISO-8601 with a `Z` suffix and whole seconds.
pub fn iso(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// The current time as [`iso`].
pub fn now_iso() -> String {
    iso(Utc::now())
}

/// Parse an ISO-8601 timestamp; no offset means UTC. `None` when it does not parse.
pub fn parse_iso(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let value = value.replacen(' ', "T", 1);
    if let Ok(dt) = DateTime::parse_from_rfc3339(&value) {
        return Some(dt.with_timezone(&Utc));
    }
    ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M"]
        .iter()
        .find_map(|fmt| NaiveDateTime::parse_from_str(&value, fmt).ok())
        .map(|naive| naive.and_utc())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let at = parse_iso("2026-11-04T21:00:00-05:00").unwrap();
        assert_eq!(iso(at), "2026-11-05T02:00:00Z");
        assert_eq!(parse_iso("2026-11-05T02:00:00Z"), Some(at));
    }

    #[test]
    fn naive_is_utc_and_junk_is_none() {
        assert_eq!(iso(parse_iso("2026-01-02T03:04:05").unwrap()), "2026-01-02T03:04:05Z");
        assert_eq!(parse_iso(""), None);
        assert_eq!(parse_iso("not a date"), None);
    }
}
