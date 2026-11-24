//! What a package will do and need, shown before anything is made: the same checks as an import, and no change.
use super::check::checked;
use super::sign::{creator, hash};
use crate::server::s;
use serde_json::{json, Value};

/// What NeoBot itself holds a bot to, and what a package only declares.
fn enforced(p: &Value) -> Value {
    json!([
        {"what": "Connectors", "enforced": true, "note": "a bot reaches only the connectors and tools you grant it, and you grant them yourself"},
        {"what": "Files", "enforced": true, "note": "a bot writes only in its own folder and the shared skills"},
        {"what": "Browser", "enforced": false, "declared": p["permissions"]["browser"], "note": if p["permissions"]["browser"] == true { "the package uses NeoBot's browser; it is not limited per bot yet" } else { "the package says it does not use the browser; it is not limited per bot yet" }},
        {"what": "Network", "enforced": false, "note": "not limited per bot yet"},
    ])
}

/// The preview of `p`, or why it cannot be imported.
pub(in crate::bots) fn preview(p: &Value) -> Result<Value, String> {
    let c = checked(p)?;
    let who = creator(p)
        .ok_or("the signature does not hold: this package was changed or is not signed")?;
    Ok(json!({
        "hash": hash(p), "creator": who, "created": p["created"],
        "name": c.name, "shape": c.shape, "color": c.color,
        "description": s(&p["bot"], "description"), "instructions": s(&p["bot"], "instructions"),
        "routines": c.routines.iter().map(|r| json!({"title": r["title"], "trigger": r["trigger"], "days": r["days"], "at": r["at"], "when": r["when"], "prompt": r["prompt"]})).collect::<Vec<_>>(),
        "skills": c.skills.iter().map(|(n, t)| json!({"name": n, "text": t})).collect::<Vec<_>>(),
        "integrations": p["integrations"], "permissions": enforced(p),
        "grants_nothing": "importing grants no connector: the bot starts with none and you give it each one you accept",
    }))
}
