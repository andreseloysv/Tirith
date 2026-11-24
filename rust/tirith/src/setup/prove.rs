//! The connection check: one real, cheap turn of an agent by the same way its work runs (its login, the sandbox and proxy, a tool, the rules' hook).
use super::catalog;
use crate::app::{redact_secrets, Shared};
use serde_json::{json, Value};

/// The factory's hidden turn role the check runs as.
pub const ROLE: &str = "check";
const WORD: &str = "TIRITH-OK";
const ASK: &str = "This is NeoBot's connection check. Run this shell command exactly once: `neobot --version`. Then answer with the single word TIRITH-OK and nothing else.";
static ONE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// `{ok, answered, hooks, said, model}`: it answered, and its hook reached this daemon.
pub async fn prove(app: &Shared, alias: &str) -> Result<Value, String> {
    let _one = ONE.lock().await; // the check's role has one setting at a time
    app.reload_config(true);
    let cfg = app.cfg();
    cfg.factory
        .command(alias)
        .ok_or(format!("{alias} is not connected: neobot connect {alias}"))?;
    let known = catalog::known(alias).unwrap_or_default();
    let set = |s: &str| (!s.is_empty()).then(|| s.to_string());
    let setting = json!({"agent": alias, "model": set(&known.check_model), "effort": set(&known.check_effort), "fallback": []});
    app.set_kv(&format!("models:{ROLE}"), Some(&setting.to_string()));
    let before = app
        .query("SELECT COALESCE(MAX(id), 0) FROM events", [], |r| {
            r.get::<_, i64>(0)
        })
        .pop()
        .unwrap_or(0);
    let log = app.home.join(format!("logs/{ROLE}.log"));
    let logged = std::fs::metadata(&log).map_or(0, |m| m.len());
    let turn = crate::manager::run_turn(app, ROLE, ROLE, ASK).await;
    let model = app
        .query(
            "SELECT model FROM turns WHERE role = ?1 ORDER BY id DESC LIMIT 1",
            [ROLE],
            |r| r.get::<_, String>(0),
        )
        .pop()
        .unwrap_or_default();
    app.set_kv(&format!("models:{ROLE}"), None);
    let hooks = app
        .query(
            "SELECT COUNT(*) FROM events WHERE id > ?1 AND kind = 'PreToolUse' AND summary LIKE '%neobot --version%'",
            [before],
            |r| r.get::<_, i64>(0),
        )
        .first()
        .is_some_and(|n| *n > 0);
    let answered = turn.as_ref().is_ok_and(|t| t.contains(WORD));
    let said = turn.unwrap_or_else(|e| match super::why::since(&log, logged) {
        why if why.is_empty() => e,
        why => format!("{e}: {why}"),
    });
    let said: String = redact_secrets(&said).chars().take(600).collect();
    Ok(
        json!({"ok": answered && hooks, "answered": answered, "hooks": hooks, "said": said, "model": model}),
    )
}
