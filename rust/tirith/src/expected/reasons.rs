//! Why a turn failed, in a few words for the owner: the kind of failure and the agent's own last line.
use super::words::short;
use regex::Regex;
use std::sync::LazyLock;

static NET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)enotfound|econnrefused|econnreset|etimedout|eai_again|(cannot|could ?n.t|could not|failed to) (resolve|connect)|unable to connect|connection (error|refused|reset|closed)|network (error|is unreachable|unreachable)|fetch failed|error sending request|unreachable|tirith'?s? (proxy|network)|blocked by tirith").unwrap()
});
static SESSION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)not logged in|please (run /)?log ?in|invalid (api key|bearer|authentication)|authentication_error|oauth token (has )?(expired|revoked)|unauthorized|\b401\b").unwrap()
});
static TIME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)timed out|no answer within|timeout").unwrap());

/// The kind of failure in `said`, then its last line, with no secret in it.
pub(super) fn failure(said: &str) -> String {
    let last = said.lines().rev().find(|l| !l.trim().is_empty());
    let Some(last) = last else {
        return "its agent ended without answering".into();
    };
    let kind = if crate::models::out_of_credits(said) {
        "out of credits"
    } else if NET.is_match(said) {
        "no network"
    } else if SESSION.is_match(said) {
        "not logged in"
    } else if TIME.is_match(said) {
        "took too long"
    } else {
        "its agent failed"
    };
    format!("{kind} ({})", short(&crate::app::redact_secrets(last), 140))
}
