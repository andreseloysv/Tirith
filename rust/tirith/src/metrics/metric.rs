//! One health measure: how it is built from the factory's description of its id, and the line a CEO pass reads.
use crate::app::Lane;
pub(super) struct Metric {
    pub(super) id: &'static str,
    pub(super) label: String,
    pub(super) value: f64,
    /// `None` for metrics shown without a target; `at_least` says which side of the target is green.
    pub(super) target: Option<f64>,
    pub(super) at_least: bool,
    pub(super) red: bool,
    pub(super) facts: String,
}
impl Metric {
    /// A measure with the factory's label, target and side for its id (`described`); red when it counts now (`live`) and is on the wrong side of its target.
    pub(super) fn new(
        described: (String, Option<f64>, bool),
        id: &'static str,
        value: f64,
        live: bool,
        facts: String,
    ) -> Metric {
        let (label, target, at_least) = described;
        let red = live && target.is_some_and(|t| if at_least { value < t } else { value > t });
        Metric {
            id,
            label,
            value: (value * 10.0).round() / 10.0,
            target,
            at_least,
            red,
            facts,
        }
    }
}
pub(super) fn describe(m: &Metric) -> String {
    let target = m
        .target
        .map(|t| {
            format!(
                " (target: {} {t})",
                if m.at_least { "at least" } else { "at most" }
            )
        })
        .unwrap_or_default();
    let facts = if m.facts.is_empty() {
        String::new()
    } else {
        format!(". Facts: {}", m.facts)
    };
    format!("{} [{}] is {}{}{}", m.label, m.id, m.value, target, facts)
}
/// Seconds since the epoch of an RFC 3339 time.
pub(super) fn secs(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.timestamp())
}
/// An epoch second as the database writes times.
pub(super) fn iso(s: i64) -> String {
    chrono::DateTime::from_timestamp(s, 0)
        .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_default()
}
/// The lanes' ids, comma-separated.
pub(super) fn ids(v: &[&Lane]) -> String {
    v.iter()
        .map(|l| l.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}
