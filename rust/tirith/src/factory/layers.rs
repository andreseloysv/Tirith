//! The layers of one factory version: the embedded text, the folder's file, config.toml's sections and routes, and the `default` agent under every other.
use super::embedded::TEXT;
use super::lists::SECTIONS;
use super::values::{merge, rows, str_of};

/// The embedded factory with the folder's file laid over it.
pub(super) fn files(text: &str) -> Result<toml::Value, String> {
    let mut table = toml::Value::Table(TEXT.parse().map_err(|e| format!("factory.toml: {e}"))?);
    merge(
        &mut table,
        &toml::Value::Table(text.parse().map_err(|e| format!("factory.toml: {e}"))?),
    );
    Ok(table)
}

/// config.toml's factory sections laid over the table, but its routes (see `routes`).
pub(super) fn sections(table: &mut toml::Value, overlay: &toml::Table) {
    SECTIONS
        .into_iter()
        .filter(|sec| *sec != "routes")
        .filter_map(|sec| overlay.get(sec).map(|o| (sec, o)))
        .for_each(|(sec, o)| merge(&mut table[sec], o));
}

/// Routes config.toml writes to a role replace the factory's to that role, as a set (so the page can also take a factory route away from a bot); schedule routes (a bot's routines, a timed pass) stand next to them.
pub(super) fn routes(table: &mut toml::Value, overlay: &toml::Table) {
    let laid: Vec<toml::Value> = rows(overlay, "routes").cloned().collect();
    let replaced: Vec<&str> = laid
        .iter()
        .filter(|r| str_of(r, "on") != "schedule")
        .map(|r| str_of(r, "to"))
        .collect();
    table["routes"].as_array_mut().into_iter().for_each(|dst| {
        dst.retain(|r| str_of(r, "on") == "schedule" || !replaced.contains(&str_of(r, "to")));
        dst.extend(laid.iter().cloned());
    });
}

/// The `default` agent under every other (never its chat args for an exec agent), and the codex command needs no config line, as before.
pub(super) fn agents(table: &mut toml::Value) {
    let default = table["agents"]["default"].clone();
    for a in table["agents"]
        .as_table_mut()
        .into_iter()
        .flat_map(|t| t.iter_mut())
        .filter(|(n, _)| *n != "default")
        .map(|(_, a)| a)
    {
        let mut under = default.clone();
        let _ = (a.get("protocol").and_then(|p| p.as_str()) == Some("exec"))
            .then(|| under.as_table_mut().map(|t| t.remove("chat_args")));
        merge(&mut under, a);
        *a = under;
    }
    let _ = table["agents"]
        .as_table_mut()
        .and_then(|a| a.get_mut("codex"))
        .and_then(|c| c.as_table_mut())
        .map(|c| c.entry("command").or_insert_with(|| "codex".into()));
}
