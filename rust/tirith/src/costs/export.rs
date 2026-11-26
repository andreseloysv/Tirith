//! The rows as JSON or CSV, for a BI tool: one flat table, a header line, dollars with no sign and no thousands.
use super::group::{done_items, group, Row};
use super::rows::runs;
use crate::app::Shared;
use serde_json::{json, Value};

const NUMBERS: [&str; 7] = [
    "runs",
    "priced_runs",
    "tokens_in",
    "tokens_out",
    "tokens_cache_read",
    "tokens_cache_write",
    "cost_usd",
];

fn object(row: &Row) -> Value {
    let mut o = serde_json::Map::new();
    row.keys
        .iter()
        .for_each(|(k, v)| drop(o.insert(k.to_string(), json!(v))));
    if let Value::Object(sum) = row.sum.json() {
        o.extend(sum);
    }
    Value::Object(o)
}

/// The summary of every finished item (a plan item ticked done) among `rows` grouped by item: how many, what they cost, and what one cost on average.
fn finished(rows: &[Row]) -> Value {
    let done: Vec<&Row> = rows
        .iter()
        .filter(|r| r.keys.iter().any(|(k, v)| *k == "done" && v == "true"))
        .collect();
    let cost: f64 = done.iter().map(|r| r.sum.cost).sum();
    let priced = done.iter().filter(|r| r.sum.priced > 0).count();
    json!({"finished_items": done.len(), "finished_items_priced": priced, "finished_cost_usd": (cost * 1e6).round() / 1e6,
        "cost_per_finished_item_usd": (priced > 0).then(|| (cost / priced as f64 * 1e6).round() / 1e6)})
}

/// `{project, since, by, rows, total, finished}`: the rows grouped `by`, all the runs added up, and the cost per finished item.
pub fn report(app: &Shared, project: &str, since: &str, by: &str) -> Value {
    let all = runs(app, project, since);
    let done = done_items(app, &all);
    let rows = group(&all, by, &done);
    let mut total = super::sum::Sum::default();
    all.iter().for_each(|r| total.add(r));
    let items = group(&all, "item", &done);
    let mut columns: Vec<String> = rows
        .first()
        .map(|r| r.keys.iter().map(|(k, _)| k.to_string()).collect())
        .unwrap_or_default();
    columns.extend(NUMBERS.map(String::from));
    json!({"project": project, "since": since, "by": by, "columns": columns, "rows": rows.iter().map(object).collect::<Vec<_>>(),
        "total": total.json(), "finished": finished(&items)})
}

fn cell(v: &Value) -> String {
    let s = match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

/// The report's rows as CSV: the grouping columns, then the numbers.
pub fn csv(report: &Value) -> String {
    let rows = report["rows"].as_array().cloned().unwrap_or_default();
    let cols: Vec<String> = serde_json::from_value(report["columns"].clone()).unwrap_or_default();
    let mut out = cols.join(",") + "\n";
    for row in &rows {
        out += &cols
            .iter()
            .map(|c| cell(&row[c]))
            .collect::<Vec<_>>()
            .join(",");
        out.push('\n');
    }
    out
}
