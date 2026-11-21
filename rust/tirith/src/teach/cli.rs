//! `tirith teach rehearse|do|stop`: a bot rehearses a draft in the owner's Edge, from its session (`TIRITH_PROJECT` is its conversation).
use crate::client::{call_with, Flag};
use serde_json::{json, Value};
use std::time::Duration;
const USE: &str = "use: tirith teach rehearse \"<skill>\" | tirith teach do open \"<url>\" | do click \"<selector or text>\" | do type \"<field>\" \"<value>\" | tirith teach stop";
/// The request a `tirith teach` command makes: its path under the bot's teach API and its body.
pub(super) fn request(pos: &[&str]) -> Result<(&'static str, Value), String> {
    Ok(match pos {
        ["rehearse", skill] => ("rehearse", json!({"skill": skill})),
        ["stop"] => ("rehearse/stop", json!({})),
        ["do", kind @ ("open" | "click"), target] => {
            ("do", json!({"kind": kind, "target": target}))
        }
        ["do", "type", field, value] => (
            "do",
            json!({"kind": "type", "target": field, "value": value}),
        ),
        _ => return Err(USE.into()),
    })
}
/// What the bot reads back: the step's result and where its screenshot is.
fn said(v: &Value) -> Result<(), String> {
    let shot = v["shot"].as_str().filter(|p| !p.is_empty());
    let line = match (v["n"].as_u64(), shot) {
        (Some(n), Some(p)) => format!(
            "step {n}: {}\nscreenshot: {p}",
            v["result"].as_str().unwrap_or("")
        ),
        (Some(n), None) => format!("step {n}: {}", v["result"].as_str().unwrap_or("")),
        _ => "ok".into(),
    };
    if v["ok"] == false && v["n"].is_u64() {
        return Err(format!("failed · {line}"));
    }
    println!("{line}");
    Ok(())
}
pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    let ["teach", rest @ ..] = pos else {
        return None;
    };
    let conv = flag("--project").or_else(|| std::env::var("TIRITH_PROJECT").ok());
    let conv = match conv.filter(|c| !c.is_empty()) {
        Some(c) => c,
        None => {
            return Some(Err(
                "no conversation: run it in a bot's session (TIRITH_PROJECT)".into(),
            ))
        }
    };
    let id = conv.strip_prefix("bot-").unwrap_or(&conv).to_string();
    let sent = request(rest).and_then(|(path, body)| {
        let who = [(
            "X-Tirith-Role",
            std::env::var("TIRITH_ROLE").unwrap_or_default(),
        )];
        let path = format!("/api/teach/{id}/{path}");
        call_with(
            "POST",
            &path,
            body.to_string(),
            &who,
            Duration::from_secs(55),
        )
    });
    let read = |b: String| serde_json::from_str::<Value>(&b).unwrap_or_default();
    Some(sent.map(read).and_then(|v| said(&v)).map_err(|e| {
        let v: Value = serde_json::from_str(&e).unwrap_or_default();
        v["error"].as_str().map_or(e, str::to_string)
    }))
}
