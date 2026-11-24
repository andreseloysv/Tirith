//! An HTTP hook of NeoBot's as it is installed, and one of an older install brought up to it when the daemon starts: without the token a lane's hook could only refuse.
use crate::app::Shared;
use serde_json::{json, Value};

/// The hook of `event` on the daemon at `port`.
pub(super) fn http(port: u16, event: &str) -> Value {
    json!({"type": "http", "url": format!("http://127.0.0.1:{port}/hook/{event}"),
        "headers": {"X-Tirith-Token": "$TIRITH_TOKEN"},
        "allowedEnvVars": ["TIRITH_TOKEN"], "timeout": 5})
}

/// Gives this daemon's own installed hooks the token's header; whether any changed. Nothing else of `settings` is touched.
fn upgrade(settings: &mut Value, port: u16) -> bool {
    let ours = format!("http://127.0.0.1:{port}/hook/");
    let mut changed = false;
    let events = settings["hooks"].as_object_mut().into_iter().flatten();
    for (event, groups) in events {
        let groups = groups.as_array_mut().into_iter().flatten();
        let hooks = groups
            .filter_map(|g| g.get_mut("hooks")?.as_array_mut())
            .flatten();
        for hook in hooks.filter(|h| h["url"].as_str().is_some_and(|u| u.starts_with(&ours))) {
            let new = http(port, event);
            for key in ["headers", "allowedEnvVars"] {
                if hook[key] != new[key] {
                    hook[key] = new[key].clone();
                    changed = true;
                }
            }
        }
    }
    changed
}

/// Run by the daemon as it starts; the file as it was stays beside it.
pub(super) fn at_start(app: &Shared) {
    let cfg = app.cfg();
    let Some(set) = cfg.factory.agent::<String>("anthropic", "settings") else {
        return;
    };
    let path = crate::config::expand(&set);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(mut settings) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    if !settings["hooks"].is_object() || !upgrade(&mut settings, cfg.port) {
        return;
    }
    let before = path.with_extension("json.before-tirith-token");
    let done = serde_json::to_string_pretty(&settings)
        .map_err(std::io::Error::other)
        .and_then(|out| {
            std::fs::write(before, &text)?;
            std::fs::write(&path, out + "\n")
        });
    if let Err(e) = done {
        eprintln!(
            "neobot: {}: its hooks still lack the token ({e}): run neobot hooks install",
            path.display()
        );
    }
}
