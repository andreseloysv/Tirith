//! What a turn used, read from what its agent already prints: Anthropic's `result` line, Muse's `model_completed` records (summed in the turn), Codex's `turn.completed`.
use serde_json::Value;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Usage {
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    /// What the agent said the turn cost, in dollars (Anthropic's `total_cost_usd`).
    pub cost: Option<f64>,
    /// The agent said the turn failed (Anthropic's `is_error`).
    pub failed: bool,
    /// Some line carried usage.
    pub seen: bool,
    /// Codex counts its cached tokens inside `input_tokens`; the others count them apart.
    pub input_has_cache: bool,
}

/// A count of `u`, 0 when missing.
fn n(u: &Value, key: &str) -> i64 {
    u[key].as_i64().unwrap_or(0)
}

impl Usage {
    /// Takes one line of any agent's output; the shapes do not collide.
    pub fn take(&mut self, msg: &Value) {
        let event = &msg["payload"]["event"];
        if msg["type"] == "result" {
            let u = &msg["usage"];
            let cache = n(u, "cache_creation_input_tokens");
            self.add(u, "input_tokens", "cache_read_input_tokens", cache);
            self.cost = msg["total_cost_usd"].as_f64().or(self.cost);
            self.failed = msg["is_error"] == true;
        } else if msg["type"] == "turn.completed" {
            self.add(&msg["usage"], "input_tokens", "cached_input_tokens", 0);
            self.input_has_cache = true;
        } else if msg["payload_type"] == "runtime.session" && event["kind"] == "model_completed" {
            let u = &event["usage"];
            let write = n(u, "cache_write_tokens");
            self.add(u, "input_tokens", "cache_read_tokens", write);
        }
    }

    fn add(&mut self, u: &Value, input: &str, cache_read: &str, cache_write: i64) {
        self.input += n(u, input);
        self.output += n(u, "output_tokens");
        self.cache_read += n(u, cache_read);
        self.cache_write += cache_write;
        self.seen = true;
    }
}

/// Muse's usage of a turn from its session's own log, when its output did not carry it: the `model_completed` records of `~/.local/share/muse/sessions/<year>/<month>/<day>/<id>/session.jsonl` recorded since `since_us` (microseconds).
pub fn from_muse_session(id: &str, since_us: i64) -> Usage {
    let mut usage = Usage::default();
    let root = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".local/share/muse/sessions");
    let Some(file) = find(&root, id, 3) else {
        return usage;
    };
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|m| m["recorded_at"].as_i64().unwrap_or(0) >= since_us)
        .for_each(|m| usage.take(&m));
    usage
}

/// The session.jsonl of session `id`, `depth` folders (year, month, day) below `dir`.
fn find(dir: &std::path::Path, id: &str, depth: u32) -> Option<std::path::PathBuf> {
    if depth == 0 {
        let file = dir.join(id).join("session.jsonl");
        return file.exists().then_some(file);
    }
    let mut subs: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();
    subs.sort_by_key(|e| std::cmp::Reverse(e.file_name())); // the newest days first
    subs.iter().find_map(|e| find(&e.path(), id, depth - 1))
}
