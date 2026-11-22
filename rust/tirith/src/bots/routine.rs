//! A routine made, changed or removed from a bot's card or its own commands.
use super::list::key_for;
use super::schedule::days;
use super::store::{find, with};
use crate::app::{now_iso, Shared};
use crate::server::s;
use chrono::NaiveTime;
use serde_json::{json, Value};
/// A timed routine's days and its time, written HH:MM.
fn timed(r: &mut Value, b: &Value) -> Result<bool, String> {
    r["days"] = days(&b["days"]).map_or(r["days"].clone(), |d| json!(d));
    let at = NaiveTime::parse_from_str(s(r, "at").trim(), "%H:%M")
        .map_err(|_| "the time must be HH:MM, as 08:00")?;
    r["at"] = json!(at.format("%H:%M").to_string());
    Ok(r["days"].as_array().is_some_and(|d| !d.is_empty()))
}
/// The fields `b` carries laid over the routine `r`: its `trigger` is `schedule` (a time on some days) or `webhook` (an event from outside whose `when` holds: `<source:kind>`, a pattern of it with `*`, or `*`); it must keep a title, what to do, and a day or a `when`.
pub(super) fn set(r: &mut Value, b: &Value) -> Result<(), String> {
    ["title", "prompt", "at", "on", "trigger", "when"]
        .into_iter()
        .filter(|k| !b[*k].is_null())
        .for_each(|k| r[k] = b[k].clone());
    let hook = s(r, "trigger") == "webhook";
    let ok = if hook {
        r["when"] = json!(s(r, "when").trim());
        !s(r, "when").is_empty()
    } else {
        r["trigger"] = json!("schedule");
        timed(r, b)?
    };
    let ok = ok && !s(r, "title").trim().is_empty() && !s(r, "prompt").trim().is_empty();
    ok.then_some(()).ok_or(
        "a routine needs a title, what to do, and at least one day or the event it waits for",
    )?;
    Ok(())
}
/// The change to the bot's routines: (what the conversation says, the id of a changed routine).
fn edit(
    list: &mut Vec<Value>,
    b: &Value,
    rid: &str,
    title: &str,
) -> Result<(String, String), String> {
    let i = list.iter().position(|r| {
        (!rid.is_empty() && r["id"] == rid)
            || (!title.is_empty() && s(r, "title").eq_ignore_ascii_case(title))
    });
    let Some(()) = (b["delete"] != true).then_some(()) else {
        return Ok((
            format!(
                "Removed routine · {}",
                s(&list.remove(i.ok_or("no such routine")?), "title")
            ),
            String::new(),
        ));
    };
    list.extend(
        i.is_none()
            .then(|| json!({"id": format!("r{:06x}", rand::random::<u32>() >> 8), "on": true})),
    );
    let last = list.len() - 1;
    let r = &mut list[i.unwrap_or(last)];
    set(r, b)?;
    Ok((
        if i.is_none() {
            format!("Created routine · {}", s(r, "title"))
        } else {
            String::new()
        },
        s(r, "id"),
    ))
}
/// A new routine, or a change to one (found by `id` or by its title): `title`, `prompt`, `trigger` (`schedule` or `webhook`), `days` and `at` ("HH:MM") or `when`, `on`; with `delete` it goes. The conversation says when one is made or removed.
pub(super) fn routine(app: &Shared, id: String, b: Value) -> Result<Value, String> {
    let (rid, title) = (s(&b, "id"), s(&b, "title"));
    let (said, changed) = with(app, |bots| {
        let list = find(bots, &id)?["routines"]
            .as_array_mut()
            .ok_or(format!("bot '{id}' has no routines"))?;
        edit(list, &b, &rid, &title)
    })?;
    // A changed routine does not run for a time already past today.
    let _ = (!changed.is_empty())
        .then(|| app.set_kv(&format!("routine:{id}:{changed}"), Some(&now_iso())));
    let _ = (!said.is_empty())
        .then(|| app.chat("routine", &said, &key_for(app, &id).unwrap_or_default()));
    Ok(json!({"ok": true}))
}
