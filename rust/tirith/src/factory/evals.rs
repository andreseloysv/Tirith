//! Reading a factory's evals, and the hash of the setting a role runs with (what an eval's verdict is tied to).
use super::loaded::Factory;
use super::rowrefs::sample;
use super::stamp::short_hash;
use super::values::{rows, stem, str_of};

/// One `[[evals]]` entry read for one event: (id, judge role, rubric file stem, sample share).
pub type Eval = (String, String, String, f64);

impl Factory {
    /// The evals that judge `on`: (id, judge role, rubric file stem, sample share) of every `[[evals]]` on it.
    pub fn evals_on(&self, on: &str) -> Vec<Eval> {
        rows(&self.raw, "evals")
            .filter(|e| str_of(e, "on") == on)
            .map(|e| {
                (
                    str_of(e, "id").into(),
                    str_of(e, "judge").into(),
                    stem(str_of(e, "rubric")),
                    sample(e),
                )
            })
            .collect()
    }

    /// Those of them that judge event `event`: a share of the events is judged, by their ids.
    pub fn evals_for(&self, on: &str, event: i64) -> Vec<Eval> {
        let in_share = |e: &Eval| event.rem_euclid(100) < (e.3 * 100.0) as i64;
        self.evals_on(on).into_iter().filter(in_share).collect()
    }

    /// True when some eval's judge is `role`.
    pub fn is_judge(&self, role: &str) -> bool {
        rows(&self.raw, "evals").any(|e| str_of(e, "judge") == role)
    }

    /// A short hash of the setting `role` runs with: its resolved table (agent, model, effort, ...), its agent's table, its prompt texts and `extra` (the page's model override for it, when the caller has one). An unknown role has none (an empty stamp).
    pub fn role_hash(&self, role: &str, extra: &str) -> String {
        let Some(r) = self.raw.get("roles").and_then(|s| s.get(role)) else {
            return String::new();
        };
        let json = |v: Option<&toml::Value>| serde_json::to_string(&v).unwrap_or_default();
        short_hash(&format!(
            "{}|{}|{}|{extra}",
            json(Some(r)),
            json(
                self.raw
                    .get("agents")
                    .and_then(|s| s.get(str_of(r, "agent")))
            ),
            self.prompts_of(role)
        ))
    }
}
