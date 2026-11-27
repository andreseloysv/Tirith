//! What runs add up to: their tokens, their cost where a price is known, and how many had none.
use serde_json::{json, Value};

/// One run of a lane, as `lane_runs` keeps it.
pub struct Run {
    pub lane: String,
    pub project: String,
    pub step: String,
    pub item: String,
    pub issue: String,
    pub attempt: i64,
    pub agent: String,
    pub model: String,
    pub started_at: String,
    pub ended_at: String,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub cost: Option<f64>,
    pub source: String,
    /// What ran: `lane`, `bot` (a bot's turn) or `judge` (a judge's turn, of the bot's conversation `bot`).
    pub kind: String,
    pub bot: String,
}

#[derive(Default, Clone)]
pub struct Sum {
    pub runs: i64,
    /// Runs whose cost is known.
    pub priced: i64,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub cost: f64,
    /// The cost of each kind of run: lanes, bots' turns, judges' turns.
    pub by_kind: [f64; 3],
}

impl Sum {
    pub fn add(&mut self, r: &Run) {
        self.runs += 1;
        self.priced += r.cost.is_some() as i64;
        self.input += r.input;
        self.output += r.output;
        self.cache_read += r.cache_read;
        self.cache_write += r.cache_write;
        self.cost += r.cost.unwrap_or(0.0);
        let at = ["lane", "bot", "judge"].iter().position(|k| *k == r.kind);
        at.into_iter()
            .for_each(|i| self.by_kind[i] += r.cost.unwrap_or(0.0));
    }

    fn kind_cost(&self, i: usize) -> Option<f64> {
        (self.priced > 0).then_some((self.by_kind[i] * 1e6).round() / 1e6)
    }

    pub fn json(&self) -> Value {
        let cost = (self.priced > 0).then_some((self.cost * 1e6).round() / 1e6);
        json!({"runs": self.runs, "priced_runs": self.priced, "tokens_in": self.input, "tokens_out": self.output,
            "tokens_cache_read": self.cache_read, "tokens_cache_write": self.cache_write, "cost_usd": cost,
            "lane_cost_usd": self.kind_cost(0), "bot_cost_usd": self.kind_cost(1), "judge_cost_usd": self.kind_cost(2)})
    }
}
