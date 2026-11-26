//! What a lane or an item spent, as one short text: dollars (when a price is known) and tokens.
use serde_json::Value;

fn thousands(n: i64) -> String {
    match n {
        0..=999 => n.to_string(),
        1000..=999_999 => format!("{:.1}k", n as f64 / 1e3),
        _ => format!("{:.1}M", n as f64 / 1e6),
    }
}

/// `$0.0123 · 4.2k in, 310 out, 3 runs`, or nothing for a lane that has not run.
pub(super) fn spent(s: &Value) -> String {
    let runs = s["runs"].as_i64().unwrap_or(0);
    if runs == 0 {
        return String::new();
    }
    let n = |k: &str| s[k].as_i64().unwrap_or(0);
    let dollars = s["cost_usd"]
        .as_f64()
        .map_or("no price".into(), |c| format!("${c:.4}"));
    let tokens = n("tokens_in") + n("tokens_cache_read") + n("tokens_cache_write");
    let (word, unpriced) = (
        if runs == 1 { "run" } else { "runs" },
        runs - n("priced_runs"),
    );
    let part = if unpriced > 0 && unpriced < runs {
        format!(", {unpriced} unpriced")
    } else {
        String::new()
    };
    format!(
        "  {dollars} · {} in, {} out, {runs} {word}{part}",
        thousands(tokens),
        thousands(n("tokens_out"))
    )
}
