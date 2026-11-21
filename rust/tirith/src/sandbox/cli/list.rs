//! `tirith net list`: the destinations seen, then the rules blocking.
use serde_json::Value;

pub fn list() -> Result<(), String> {
    crate::client::call_json("GET", "/api/net/destinations", None).and_then(|d| {
        for x in d.as_array().into_iter().flatten().take(40) {
            print_destination(x);
        }
        print_blocks()
    })
}

/// One destination: host, connections and refusals, last seen, callers and the rule blocking it.
fn print_destination(x: &Value) {
    let lanes: Vec<&str> = x["lanes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let rule = x["rule"]
        .as_str()
        .map(|r| format!("  BLOCKED by {r}"))
        .unwrap_or_default();
    let n = format!(
        "{}{}",
        x["count"],
        if x["refused"] == 0 {
            String::new()
        } else {
            format!("+{} refused", x["refused"])
        }
    );
    println!(
        "{:<40} {n:>12}  {}  {}{rule}",
        x["host"].as_str().unwrap_or(""),
        x["last"].as_str().unwrap_or(""),
        lanes.join(",")
    );
}

fn print_blocks() -> Result<(), String> {
    let blocks = crate::client::call_json("GET", "/api/net/blocks", None)?;
    println!(
        "blocked: {}",
        blocks
            .as_array()
            .map(|b| b
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" "))
            .unwrap_or_default()
    );
    Ok(())
}
