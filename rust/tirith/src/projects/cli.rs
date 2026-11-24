//! `neobot project add`.
use serde_json::{json, Value};

/// `neobot project add <folder> [--name N] [--color #rrggbb]`.
pub fn cli(pos: &[&str], flag: crate::client::Flag) -> Option<Result<(), String>> {
    let ["project", "add", raw] = pos else {
        return None;
    };
    let Some(()) = (!std::env::var("TIRITH_LANE").is_ok_and(|l| !l.is_empty())).then_some(())
    else {
        return Some(Err(
            "a lane does not add projects: ask the manager with neobot ask \"<question>\"".into(),
        ));
    };
    let full = std::env::current_dir()
        .map(|d| d.join(raw).display().to_string())
        .unwrap_or(raw.to_string()); // an absolute path stays
    let body = json!({"folder": if raw.starts_with('~') { raw.to_string() } else { full }, "name": flag("--name").unwrap_or_default(), "color": flag("--color").unwrap_or_default()});
    Some(crate::client::call_json("POST", "/api/projects", Some(body)).map(|v| say(&v)))
}

/// What the daemon answered: the project, and the notes on what adding it did.
fn say(v: &Value) {
    let (id, dir) = (
        v["id"].as_str().unwrap_or(""),
        v["folder"].as_str().unwrap_or(""),
    );
    println!(
        "{}",
        if v["exists"] == true {
            format!("{dir} is already the project {id}")
        } else {
            format!("{id}: {dir}")
        }
    );
    v["notes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .for_each(|n| println!("  {n}"));
}
