//! `tirith plan show`: each step with its open items and their lanes.
use crate::client;
use serde_json::Value;

pub(super) fn plan_show(project: &str) -> Result<(), String> {
    let p = client::call_json("GET", &format!("/api/plan/{project}"), None)?;
    p["steps"].as_array().into_iter().flatten().for_each(|st| {
        let items = st["items"].as_array().cloned().unwrap_or_default();
        let open: Vec<&Value> = items.iter().filter(|i| i["done"] != true).collect();
        println!(
            "## {}. {}  ({} open, {} done)",
            st["num"].as_str().unwrap_or(""),
            st["title"].as_str().unwrap_or(""),
            open.len(),
            items.len() - open.len()
        );
        open.iter().for_each(|it| print_item(it));
    });
    Ok(())
}

fn print_item(it: &Value) {
    println!(
        "  - [ ] {}{}",
        it["text"].as_str().unwrap_or(""),
        it["lane"]
            .as_object()
            .map(|l| format!(
                "  [{} {}]",
                l["status"].as_str().unwrap_or(""),
                l["id"].as_str().unwrap_or("")
            ))
            .unwrap_or_default()
    )
}
