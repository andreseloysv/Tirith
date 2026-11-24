//! A bot's sealed webhook on the platform: the bot's own private room (made now, even with no phone paired), and the signed requests that make or delete the webhook of that room.
use super::settings::{base, my_name};
use super::store::{room as room_of_id, ROOMS};
use super::{create::create, listing::label, sealed, Room};
use crate::app::Shared;
use serde_json::{json, Value};
/// The conversation's own private room: the one its phones use, or a new one.
async fn room_of(app: &Shared, key: &str) -> Result<Room, String> {
    let had = ROOMS
        .lock()
        .unwrap()
        .values()
        .find(|r| r.kind == "manager" && app.conv(&r.project) == key)
        .cloned();
    match had {
        Some(r) => Ok(r),
        None => create(app, &label(app, key), key, String::new()).await,
    }
}
/// Makes the webhook of `key`'s room, sealed once for this NeoBot's key alone: the platform's `{id, url, from}`, with the room.
pub async fn hook_create(app: &Shared, key: &str) -> Result<Value, String> {
    let r = room_of(app, key).await?;
    let me = sealed::identity(&app.home, &r.me)?;
    let body = json!({"hook": "create", "readers": [me.public], "seal": "once"}); // a platform that knows it cannot open what it stored
    let mut got = sealed::request(&base(app), &me, &r.id, "/hooks", &body).await?;
    let ok = ["id", "url", "from"]
        .iter()
        .all(|k| got[k].as_str().is_some_and(|v| !v.is_empty()));
    ok.then_some(())
        .ok_or("the platform did not give a webhook")?;
    got["room"] = json!(r.id);
    Ok(got)
}
/// Deletes the webhook `id` of `room`; a webhook the platform no longer knows is gone already.
pub async fn hook_delete(app: &Shared, room: &str, id: &str) -> Result<(), String> {
    let name = room_of_id(room).map_or_else(|| my_name(app), |r| r.me);
    let me = sealed::identity(&app.home, &name)?;
    let body = json!({"hook": "delete", "id": id});
    let path = format!("/hooks/{id}/delete");
    match sealed::request(&base(app), &me, room, &path, &body).await {
        Err(e) if e.contains("no such webhook") => Ok(()),
        r => r.map(|_| ()),
    }
}
