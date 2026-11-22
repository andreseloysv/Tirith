//! A bot's webhook from the page: its URL (made once on the platform, then the same), deleted, and «Probar» — a sample event (GitHub's or an e-mail) for one routine, with no platform in between.
use super::hook::kept;
use super::hook_event::{event_of, matches};
use super::hook_fire::fire;
use super::hook_sample::for_when;
use super::list::{bot, key_for};
use crate::app::{bad, Shared, R};
use crate::server::s;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
/// What the page shows of the webhook: its URL, or none.
fn shown(h: Option<Value>) -> Value {
    h.map_or(
        json!({"url": null}),
        |h| json!({"url": s(&h, "url"), "id": s(&h, "id")}),
    )
}
async fn read(State(app): State<Shared>, Path(id): Path<String>) -> R {
    Ok(Json(shown(kept(&app, &id))))
}
/// The bot's webhook URL: the kept one, or one made now in the bot's own room.
async fn make(State(app): State<Shared>, Path(id): Path<String>) -> R {
    let key = key_for(&app, &id).ok_or_else(|| bad(format!("no bot '{id}'")))?;
    if let Some(h) = kept(&app, &id) {
        return Ok(Json(shown(Some(h))));
    }
    let h = crate::rooms::hook_create(&app, &key).await.map_err(bad)?;
    app.set_kv(&format!("webhook:{id}"), Some(&h.to_string()));
    app.emit("bots", json!({}));
    Ok(Json(shown(Some(h))))
}
async fn delete(State(app): State<Shared>, Path(id): Path<String>) -> R {
    if let Some(h) = kept(&app, &id) {
        crate::rooms::hook_delete(&app, &s(&h, "room"), &s(&h, "id"))
            .await
            .map_err(bad)?;
    }
    app.set_kv(&format!("webhook:{id}"), None);
    app.emit("bots", json!({}));
    Ok(Json(json!({"ok": true})))
}
/// «Probar»: routine `rid` runs now with the sample that fits its `when` (an e-mail, or the GitHub event it names), whatever its `when`; `matches` says whether the `when` holds for it.
async fn test(State(app): State<Shared>, Path((id, rid)): Path<(String, String)>) -> R {
    let b = key_for(&app, &id)
        .and_then(|k| bot(&app, &k))
        .ok_or_else(|| bad(format!("no bot '{id}'")))?;
    let r = b["routines"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|r| r["id"] == rid.as_str())
        .cloned()
        .ok_or_else(|| bad("no such routine".into()))?;
    let e = event_of(&for_when(&s(&r, "when"))).ok_or_else(|| bad("no sample".into()))?;
    fire(&app, &b, &r, &e, "run-now");
    Ok(Json(
        json!({"ok": true, "event": e, "matches": matches(&s(&r, "when"), &e)}),
    ))
}
pub(super) fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/bots/{id}/webhook", get(read).post(make))
        .route("/api/bots/{id}/webhook/delete", post(delete))
        .route("/api/bots/{id}/webhook/test/{routine}", post(test))
}
