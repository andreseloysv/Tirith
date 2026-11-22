//! What goes out of config.toml before the bots are written again: their roles and routes, then the sections left empty.
use super::written::has;
use serde_json::Value;
use toml_edit::DocumentMut;
/// The owner's bot roles go, and every route to one of them, and the schedule and webhook routes with a task — its routines — (and other routes, if changed) of the system bots in `bots`. A timed route with no task is a system bot's pass: nothing would write it again, so it stays.
pub(super) fn drop_bots(doc: &mut DocumentMut, bots: &[Value]) {
    doc.get_mut("roles")
        .and_then(|i| i.as_table_mut())
        .into_iter()
        .for_each(|roles| {
            roles.retain(|k, v| {
                !(k.starts_with("bot-")
                    && v.as_table()
                        .and_then(|t| t.get("scope"))
                        .and_then(|i| i.as_str())
                        == Some("own"))
            })
        });
    doc.get_mut("routes")
        .and_then(|i| i.as_array_of_tables_mut())
        .into_iter()
        .for_each(|routes| {
            routes.retain(|r| {
                let said = |k: &str| r.get(k).and_then(|i| i.as_str()).unwrap_or("");
                let (to, timed) = (said("to"), matches!(said("on"), "schedule" | "webhook"));
                let pass = timed && said("task").is_empty();
                !(to.starts_with("bot-") || (!pass && has(bots, to, !timed)))
            })
        });
}
/// An emptied `roles` table or `routes` array leaves no header behind.
pub(super) fn drop_empty(doc: &mut DocumentMut) {
    ["roles", "routes"].into_iter().for_each(|k| {
        drop(
            doc.get(k)
                .and_then(|i| {
                    i.as_table()
                        .map(|t| t.is_empty())
                        .or_else(|| i.as_array_of_tables().map(|t| t.is_empty()))
                })
                .is_some_and(|e| e)
                .then(|| doc.remove(k)),
        )
    });
}
