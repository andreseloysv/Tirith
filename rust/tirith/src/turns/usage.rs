//! What a turn used, read from what its agent prints: Anthropic's `result`, Muse's `run.usage` (or its `model_completed` records), Codex's `turn.completed`, Copilot's usage file.
use super::usage_keys::{n, Keys, ANTHROPIC, CODEX, COPILOT, MUSE_CALL, MUSE_RUN};
use serde_json::Value;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Usage {
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    /// What the agent said it cost, in dollars (Anthropic's `total_cost_usd`: its session's, so far).
    pub cost: Option<f64>,
    /// The agent said the turn failed (Anthropic's `is_error`).
    pub failed: bool,
    /// Some line carried usage.
    pub seen: bool,
    /// Codex, Copilot and Muse count their cached tokens, read and written, inside their input; Anthropic counts them apart.
    pub input_has_cache: bool,
    /// The tokens are the session's so far, not the turn's (Codex, Copilot).
    pub session_totals: bool,
    /// The model the usage names.
    pub model: String,
    /// A line gave the whole turn's sum: records that come after it are already in it.
    pub(super) summed: bool,
}

impl Usage {
    /// Takes one line of any agent's output; the shapes do not collide.
    pub fn take(&mut self, msg: &Value) {
        let kind = msg["payload_type"].as_str().or(msg["type"].as_str());
        let event = &msg["payload"]["event"];
        match kind.unwrap_or("") {
            "result" if msg["exitCode"].is_null() => {
                let u = &msg["usage"];
                let write = n(u, "cache_creation_input_tokens");
                self.add(u, ANTHROPIC, write);
                self.cost = msg["total_cost_usd"].as_f64().or(self.cost);
                self.failed = msg["is_error"] == true;
            }
            "turn.completed" => {
                let u = &msg["usage"];
                self.sum(true);
                self.add(u, CODEX, n(u, "cache_write_input_tokens"));
            }
            "runtime.session" if event["kind"] == "model_completed" && !self.summed => {
                let u = &event["usage"];
                self.input_has_cache = true;
                self.named(event["model"].as_str());
                self.add(u, MUSE_CALL, n(u, "cache_write_tokens"));
            }
            "run.usage" => {
                self.sum(false);
                for u in msg["payload"]["models"].as_array().into_iter().flatten() {
                    self.named(u["model"].as_str());
                    self.add(u, MUSE_RUN, n(u, "cache_write_tokens"));
                }
            }
            "copilot.usage" => {
                self.sum(true);
                for (model, m) in msg["data"]["modelMetrics"]
                    .as_object()
                    .into_iter()
                    .flatten()
                {
                    self.named(Some(model));
                    let u = &m["usage"];
                    self.add(u, COPILOT, n(u, "cacheWriteTokens"));
                }
            }
            _ => {}
        }
    }

    /// A line with the whole sum replaces what was counted record by record.
    fn sum(&mut self, session_totals: bool) {
        (self.input, self.output, self.cache_read, self.cache_write) = (0, 0, 0, 0);
        (self.input_has_cache, self.summed, self.session_totals) = (true, true, session_totals);
    }

    fn named(&mut self, model: Option<&str>) {
        if let Some(model) = model.filter(|_| self.model.is_empty()) {
            self.model = model.into();
        }
    }

    fn add(&mut self, u: &Value, (input, output, read): Keys, cache_write: i64) {
        self.input += n(u, input);
        self.output += n(u, output);
        self.cache_read += n(u, read);
        self.cache_write += cache_write;
        self.seen = true;
    }
}
