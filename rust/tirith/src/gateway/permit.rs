//! One-use permits for gateway calls that ask the owner first. A permit is keyed by the sha256 of the call (connector id, tool name, canonical arguments) in the bot's conversation, and bound to the approval card that asked for it: only the owner's yes on that card grants it (`manager::owner_answer`, never an agent's relayed answer), a no or a closing refuses it, and the one identical call it lets run uses it up.
use super::canon::sha;
use crate::app::{now_iso, Shared};
use serde_json::{json, Value};

/// The sha256 of one call: its connector, its tool and its canonical arguments.
pub fn call_sha(connector: &str, tool: &str, args: &Value) -> String {
    sha(&json!([connector, tool, args]))
}

/// Uses up a granted permit of conversation `conv` for call `call`: its card, once; None when there is none.
pub fn consume(app: &Shared, conv: &str, call: &str) -> Option<String> {
    let db = app.db.lock().unwrap(); // one lock: two identical calls never both take it
    let sql = "SELECT card FROM connector_permits WHERE conv = ?1 AND call_sha = ?2 AND status = 'granted' ORDER BY at LIMIT 1";
    let card: String = db.query_row(sql, [conv, call], |r| r.get(0)).ok()?;
    let sql = "UPDATE connector_permits SET status = 'used' WHERE card = ?1 AND status = 'granted'";
    let used = db.execute(sql, [&card]).unwrap_or(0);
    (used == 1).then_some(card)
}

/// The card still open for call `call` of `conv`, so asking again opens no second one.
fn asking(app: &Shared, conv: &str, call: &str) -> Option<String> {
    let sql = "SELECT p.card FROM connector_permits p JOIN owner_items o ON o.id = p.card WHERE p.conv = ?1 AND p.call_sha = ?2 AND p.status = 'asked' AND o.status = 'open' LIMIT 1";
    app.query(sql, [conv, call], |r| r.get::<_, String>(0))
        .into_iter()
        .next()
}

/// The approval card for call `call` of `conv`: the one already open, else a new one asking `question`.
pub fn ask(app: &Shared, conv: &str, call: &str, question: &str) -> Result<String, String> {
    if let Some(card) = asking(app, conv, call) {
        return Ok(card);
    }
    let row = crate::manager::ask_owner(app, conv, "approval", question, "")?;
    let card = row["id"].as_str().unwrap_or_default().to_string();
    let sql = "INSERT INTO connector_permits (card, at, conv, call_sha, status) VALUES (?1, ?2, ?3, ?4, 'asked')";
    app.insert(sql, rusqlite::params![card, now_iso(), conv, call]);
    Ok(card)
}

/// Whether card `card` asks for a gateway call.
pub fn is_gateway_card(app: &Shared, card: &str) -> bool {
    let sql = "SELECT COUNT(*) FROM connector_permits WHERE card = ?1";
    app.query(sql, [card], |r| r.get::<_, i64>(0))
        .first()
        .is_some_and(|n| *n > 0)
}

/// Whether `answer` is a yes: its first word is one of the factory's `[cards] yes`.
fn yes(app: &Shared, answer: &str) -> bool {
    let first = answer
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .find(|w| !w.is_empty())
        .unwrap_or("")
        .to_lowercase();
    let words = app.cfg().factory.words("cards", "yes");
    !first.is_empty() && words.iter().any(|w| w.to_lowercase() == first)
}

/// Card `card` ended (`status` answered or closed, with `answer`): its asked permit is granted by a yes, else refused.
pub fn settled(app: &Shared, card: &str, status: &str, answer: &str) {
    let granted = status == "answered" && yes(app, answer);
    let to = if granted { "granted" } else { "refused" };
    let sql = "UPDATE connector_permits SET status = ?1 WHERE card = ?2 AND status = 'asked'";
    app.insert(sql, rusqlite::params![to, card]);
}
