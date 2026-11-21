//! This manager joins another manager's room with the one-use code that manager gave (`sealed-<room>:<key id>:<code>`).
use super::settings::{base, my_name};
use super::{sealed, store::save, watch::watch, Peer, Room};
use crate::app::{now_iso, Shared};
use serde_json::json;
const NOT_A_CODE: &str = "that is not a room code (sealed-<room>:<key id>:<code>)";
pub(super) async fn join(app: &Shared, code: &str, project: &str) -> Result<Room, String> {
    let (id, key, secret) = parse(code).ok_or(NOT_A_CODE)?;
    let base = base(app);
    let me = sealed::identity(&app.home, &my_name(app))?;
    sealed::post(&base, &me, "announce", id, json!("joined")).await?;
    let (public, name) = sealed::keys(&base, id, &me.name)
        .await?
        .into_iter()
        .find(|(p, _)| sealed::key_id(p) == key)
        .ok_or("that manager is not in the room")?;
    sealed::post(
        &base,
        &me,
        "message",
        id,
        sealed::seal(
            &me,
            &format!("tirith-join {secret}"),
            &format!("#{id}"),
            std::slice::from_ref(&public),
        ),
    )
    .await?;
    let count = sealed::page(&base, id, None, 0, &me.name).await?["count"].as_u64();
    let host = Peer {
        name,
        key_id: key.to_string(),
        public,
        since: now_iso(),
        accepted: true,
        ..Default::default()
    };
    let r = Room {
        id: id.to_string(),
        project: app.conv(project).into(),
        kind: "peer".into(),
        me: me.name.clone(),
        me_id: me.id.clone(),
        cursor: count,
        managers: vec![host],
        ..Default::default()
    };
    save(app, &r);
    watch(app, id);
    Ok(r)
}
/// The code's room, key id and secret, when it is a well-formed code.
fn parse(code: &str) -> Option<(&str, &str, &str)> {
    let parts: Vec<&str> = code.trim().split(':').collect();
    let [id, key, secret] = parts[..] else {
        return None;
    };
    let valid = id.starts_with("sealed-")
        && key.len() == 64
        && secret.len() == 32
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    valid.then_some((id, key, secret))
}
