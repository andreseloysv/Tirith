//! How each agent names its counts of tokens.
use serde_json::Value;

/// Its input, its output and its cached input.
pub(super) type Keys = (&'static str, &'static str, &'static str);

pub(super) const ANTHROPIC: Keys = ("input_tokens", "output_tokens", "cache_read_input_tokens");
pub(super) const CODEX: Keys = ("input_tokens", "output_tokens", "cached_input_tokens");
pub(super) const MUSE_CALL: Keys = ("input_tokens", "output_tokens", "cache_read_tokens");
pub(super) const MUSE_RUN: Keys = ("input_tokens", "output_tokens", "cached_tokens");
pub(super) const COPILOT: Keys = ("inputTokens", "outputTokens", "cacheReadTokens");

/// A count of `u`, 0 when missing.
pub(super) fn n(u: &Value, key: &str) -> i64 {
    u[key].as_i64().unwrap_or(0)
}
