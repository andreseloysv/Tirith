//! `tirith guard …` commands.
use crate::client::call;
use serde_json::Value;

/// `tirith guard …` commands.
pub fn cli(pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    Some(match pos {
        ["guard", "check"] => check(),
        ["guard", "install", rest @ ..] if rest.len() <= 1 => install(rest.first().copied()),
        _ => return None,
    })
}

/// Every project's guard, as the daemon sees it.
fn all() -> Result<Vec<Value>, String> {
    serde_json::from_value(crate::client::call_json("GET", "/api/guard", None)?)
        .map_err(|e| e.to_string())
}

/// `tirith guard check`: each project's state; fails when one needs attention.
fn check() -> Result<(), String> {
    all().and_then(|ps| {
        ps.iter().for_each(|p| {
            println!(
                "{:<20} {}",
                p["project"].as_str().unwrap_or(""),
                p["reason"].as_str().unwrap_or("")
            )
        });
        ps.iter()
            .all(|p| p["ok"] == true)
            .then_some(())
            .ok_or_else(|| "some project's guard needs attention".into())
    })
}

/// `tirith guard install [project]`: that project, or every project whose guard is not ok.
fn install(project: Option<&str>) -> Result<(), String> {
    let ids: Vec<String> = project.map_or_else(
        || {
            all()
                .map(|ps| {
                    ps.iter()
                        .filter(|p| p["ok"] != true)
                        .filter_map(|p| p["project"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        },
        |id| vec![id.to_string()],
    );
    let mut failed = false;
    for id in &ids {
        match call("POST", &format!("/api/guard/{id}/install"), None) {
            Ok(_) => println!("{id}: installed"),
            Err(e) => {
                eprintln!("{id}: {e}");
                failed = true;
            }
        }
    }
    (!failed)
        .then_some(())
        .ok_or_else(|| "some guards were not installed".into())
}
