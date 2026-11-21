//! Reading a factory's measures and its word lists (plan words, card kinds).
use super::loaded::Factory;
use crate::config::get;

impl Factory {
    /// A measure's (label, target, at_least).
    pub fn metric(&self, id: &str) -> (String, Option<f64>, bool) {
        (
            get(&self.raw, &["metrics", id, "label"]).unwrap_or_default(),
            get(&self.raw, &["metrics", id, "target"]),
            get(&self.raw, &["metrics", id, "at_least"]).unwrap_or(false),
        )
    }

    /// One word list of a section, empty when missing.
    pub fn words(&self, sec: &str, key: &str) -> Vec<String> {
        get(&self.raw, &[sec, key]).unwrap_or_default()
    }
}
