//! The page's view of a room (no secret but the current one-use QR, which the page shows), and its chat links.
use super::settings::{base, owners};
use super::{agents::mentions, Room, CODE_SECS};
use crate::app::{now_s, Shared};
use serde_json::{json, Value};
use std::collections::BTreeMap;
/// The chat page link: with `code` a one-use pairing QR, without it the permanent link of a phone in the room.
fn chat_url(base: &str, r: &Room, code: Option<&str>) -> String {
    let c = code.map(|c| format!("&c={c}")).unwrap_or_default();
    let t = if r.title.is_empty() || r.title == r.id {
        String::new()
    } else {
        format!("&t={}", encode(&r.title))
    };
    format!("{base}/chat#r={}{c}&m={}{t}", r.id, r.me_id)
}
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
/// The messages that name the owner and that neither the owner nor this manager answered.
fn needs_owner(app: &Shared, r: &Room) -> Vec<u64> {
    let speakers: Vec<String> = owners(app).into_iter().chain([r.me.clone()]).collect();
    r.messages
        .iter()
        .enumerate()
        .filter(|(i, m)| {
            !speakers.contains(&m.from)
                && owners(app).iter().any(|o| mentions(&m.text, o))
                && !r.messages[i + 1..]
                    .iter()
                    .any(|l| speakers.contains(&l.from))
        })
        .map(|(_, m)| m.n)
        .collect()
}
/// Who wrote in the room: how many messages, and when last.
fn participants(r: &Room) -> Vec<Value> {
    let mut people: BTreeMap<&str, (u32, &str)> = BTreeMap::new();
    r.messages.iter().for_each(|m| {
        *people.entry(&m.from).or_default() =
            (people.get(m.from.as_str()).map_or(0, |e| e.0) + 1, &m.at)
    });
    people
        .iter()
        .map(|(n, (c, at))| json!({"name": n, "messages": c, "last_seen": at}))
        .collect()
}
fn summary(r: &Room) -> String {
    r.messages
        .last()
        .map(|m| {
            format!(
                "Latest from {}: {}",
                m.from,
                m.text.chars().take(200).collect::<String>()
            )
        })
        .unwrap_or_default()
}
/// The view `v` for the caller of `h`: whoever holds the one-use pairing code becomes the owner, so no agent is shown it.
pub(super) fn seen(h: &axum::http::HeaderMap, mut v: Value) -> Value {
    let _ = crate::server::agent(h).then(|| v["invite"] = Value::Null);
    v
}
pub(super) fn view(app: &Shared, r: &Room) -> Value {
    let base = base(app);
    let (needs, people, summary) = (needs_owner(app, r), participants(r), summary(r));
    let manager = r.kind == "manager";
    let (invite, invite_at) = (&r.codes.invite, r.codes.invite_at);
    json!({
        "id": r.id, "title": r.title, "project": r.project, "kind": r.kind, "private": r.private(),
        "url": format!("{base}/r/{}", r.id), "me": {"name": r.me, "key_id": r.me_id},
        "link": if manager { json!(chat_url(&base, r, None)) } else { Value::Null },
        "invite": if manager && !invite.is_empty() && now_s().saturating_sub(invite_at) < CODE_SECS { json!(chat_url(&base, r, Some(invite))) } else { Value::Null },
        "peer_code": r.codes.peer_code, "braked": r.brake.braked, "error": r.error, "here": r.here, "summary": summary, "needs_owner": needs,
        "devices": r.devices.iter().map(|d| json!({"name": d.name, "key_id": d.key_id, "since": d.since, "last_seen": d.last_seen})).collect::<Vec<_>>(),
        "managers": r.managers.iter().map(|d| json!({"name": d.name, "key_id": d.key_id, "accepted": d.accepted, "blocked": d.blocked, "last_seen": d.last_seen})).collect::<Vec<_>>(),
        "participants": people,
        "messages": r.messages,
    })
}
