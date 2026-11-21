//! `tirith status`: the lanes working now and the last ones that ended.
use crate::client;
use serde_json::Value;

pub(super) fn status(project: &str) -> Result<(), String> {
    let s = client::call_json("GET", "/api/state", None)?;
    let lanes: Vec<Value> = s["lanes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| project.is_empty() || l["project"] == project)
        .cloned()
        .collect();
    print_working(&lanes);
    print_ended(&lanes);
    Ok(())
}

fn print_working(lanes: &[Value]) {
    let working: Vec<&Value> = lanes
        .iter()
        .filter(|l| {
            matches!(
                l["status"].as_str(),
                Some("queued" | "running" | "waiting" | "merge-queued" | "merging")
            )
        })
        .collect();
    working.iter().for_each(|l| {
        println!(
            "{:<14} {:<40} {}:{} {}",
            l["status"].as_str().unwrap_or(""),
            l["id"].as_str().unwrap_or(""),
            l["project"].as_str().unwrap_or(""),
            l["step"].as_str().unwrap_or(""),
            l["item"].as_str().unwrap_or("")
        )
    });
    let _ = working.is_empty().then(|| println!("no lane is working"));
}

/// The 8 lanes that ended last, newest first.
fn print_ended(lanes: &[Value]) {
    let mut ended: Vec<&Value> = lanes
        .iter()
        .filter(|l| {
            l["ended_at"].is_string()
                && !matches!(l["status"].as_str(), Some("merge-queued" | "merging"))
        })
        .collect();
    ended.sort_by_key(|l| l["ended_at"].as_str().unwrap_or("").to_string());
    ended.iter().rev().take(8).for_each(|l| {
        println!(
            "{:<14} {:<40} {}",
            l["status"].as_str().unwrap_or(""),
            l["id"].as_str().unwrap_or(""),
            l["note"].as_str().unwrap_or("")
        )
    });
}
