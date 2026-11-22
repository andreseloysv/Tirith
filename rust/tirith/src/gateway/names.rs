//! The names the agent sees: `<connector>__<tool>`, so one gateway carries every connector of a bot.

const SEP: &str = "__";

/// The name tool `tool` of connector `connector` is listed under.
pub fn joined(connector: &str, tool: &str) -> String {
    format!("{connector}{SEP}{tool}")
}

/// The connector and the tool a listed name means: the longest of `ids` it starts with; else what comes before the first `__` (no connector of this bot), or nothing.
pub fn split<'a>(name: &'a str, ids: &[String]) -> (String, &'a str) {
    let known = ids
        .iter()
        .filter_map(|id| Some((id, name.strip_prefix(id.as_str())?.strip_prefix(SEP)?)))
        .max_by_key(|(id, _)| id.len());
    match known {
        Some((id, tool)) => (id.clone(), tool),
        None => name
            .split_once(SEP)
            .map_or((String::new(), name), |(c, t)| (c.to_string(), t)),
    }
}
