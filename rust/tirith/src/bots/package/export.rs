//! A bot as a package: its card, routines, own skills and the connectors it is granted, signed by this OpenBot's key.
use crate::app::Shared;
use crate::server::s;
use serde_json::{json, Value};

/// What the skills say they use of the browser: the declared permission is read from the text, so it cannot be left out by mistake.
fn browser(skills: &[(String, String)]) -> bool {
    skills
        .iter()
        .any(|(_, t)| t.contains("tirith_browser") || t.contains("browser_"))
}

fn routine(r: &Value) -> Value {
    let mut out = json!({"title": r["title"], "prompt": r["prompt"], "trigger": r["trigger"], "catch_up": r["catch_up"]});
    if s(r, "trigger") == "webhook" {
        out["when"] = r["when"].clone();
    } else {
        out["days"] = r["days"].clone();
        out["at"] = r["at"].clone();
    }
    out
}

/// Bot `id` of the owner's as a signed package; `about` and `instructions` are the creator's words for the listing.
pub(in crate::bots) fn export(app: &Shared, id: &str, about: &Value) -> Result<Value, String> {
    let key = super::super::key_for(app, id)
        .filter(|k| k.starts_with("bot-"))
        .ok_or(format!("no bot '{id}' of the owner's"))?;
    let bot = super::super::bot(app, &key).ok_or(format!("no bot '{id}'"))?;
    let own = super::super::folder::folder(app, &key).join(super::super::SKILLS);
    let skills = super::super::skills(&own, false);
    let grants: Vec<Value> = super::super::connect::granted(app, id)?
        .as_array()
        .into_iter()
        .flatten()
        .filter(|g| g["granted"] == true)
        .map(|g| json!({"connector": g["id"], "tools": g["tools"], "deny": g["deny"], "approval": g["approval"]}))
        .collect();
    let permissions = json!({"browser": browser(&skills), "files": "its own folder", "network": "not limited by OpenBot"});
    let p = json!({
        "format": super::FORMAT, "v": super::VERSION,
        "bot": {"name": bot["name"], "description": s(about, "description"), "instructions": s(about, "instructions"), "face": {"shape": bot["shape"], "color": bot["color"]}},
        "routines": bot["routines"].as_array().into_iter().flatten().map(routine).collect::<Vec<_>>(),
        "skills": skills.iter().map(|(n, t)| json!({"name": n, "text": t})).collect::<Vec<_>>(),
        "integrations": grants, "permissions": permissions, "created": crate::app::now_iso(),
    });
    Ok(super::sign::signed(p, &crate::rooms::me(app)?))
}
