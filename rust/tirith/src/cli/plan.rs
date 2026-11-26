//! `neobot plan show` (each step with its open items and their lanes) and `neobot plan step`.
use super::spent::spent;
use crate::client;
use serde_json::{json, Value};

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
        let done = items
            .iter()
            .filter(|i| i["done"] == true && i["spent"].is_object());
        done.for_each(|it| {
            println!(
                "  - [x] {}{}",
                it["text"].as_str().unwrap_or(""),
                spent(&it["spent"])
            )
        });
    });
    Ok(())
}

pub(super) fn plan_step(project: &str, num: &str, title: &str, intro: &str) -> Result<(), String> {
    client::print(client::call(
        "POST",
        &format!("/api/plan/{project}/steps"),
        Some(json!({"step": num, "title": title, "intro": intro})),
    ))
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
            + &spent(&it["spent"])
    )
}
