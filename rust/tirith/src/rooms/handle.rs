//! The rules for one message of a room, in order: only genuine events, who wrote it, what it says, pairing, history, and who it is for.
use super::agents::from_agent;
use super::history::{blocked, record};
use super::hooked::hooked;
use super::words::{not_for_people, words};
use super::{pair::pairing, poll::Out, sealed, Peer, Room, PHONE};
use serde_json::Value;
/// What every message of one poll is read with.
pub(super) struct Seen<'a> {
    pub(super) me: &'a sealed::Me,
    pub(super) platform: &'a str,
    pub(super) backlog: bool,
    pub(super) owners: &'a [String],
    pub(super) host: &'a str,
    /// The key of the agent that posts this room's webhook (`webhook:<bot>`), or empty.
    pub(super) hook_from: &'a str,
}
/// Who wrote one message.
pub(super) struct Who {
    pub(super) author: String,
    pub(super) id: String,
    pub(super) from: String,
    pub(super) at: String,
    pub(super) mine: bool,
}
impl Who {
    fn of(m: &Value, me: &sealed::Me) -> Who {
        let author = m["event"]["author"].as_str().unwrap_or("").to_string();
        Who {
            id: sealed::key_id(&author),
            from: sealed::clean(m["from"].as_str().unwrap_or("")),
            at: m["at"].as_str().unwrap_or("").to_string(),
            mine: author == me.public,
            author,
        }
    }
}
/// Applies the rules to one message (not saved here: the caller's `update` saves).
pub(super) fn handle(r: &mut Room, seen: &Seen, m: &Value, out: &mut Out) {
    let e = &m["event"];
    let Some(()) = sealed::genuine(e, seen.platform, &r.id).then_some(()) else {
        return;
    };
    let who = Who::of(m, seen.me);
    note_manager(r, m, &who);
    let Some(text) = words(r, seen.me, e) else {
        return;
    };
    let Some(()) = (!hooked(seen, &who, &text, out)).then_some(()) else {
        return;
    };
    let Some(()) = (!not_for_people(r, &who, &text, out)).then_some(()) else {
        return;
    };
    let Some(()) = (!pairing(r, seen, &who, &text, out)).then_some(()) else {
        return;
    };
    let Some(()) = (!blocked(r, &who.id)).then_some(()) else {
        return;
    };
    let device = record(r, m, &who, &text);
    let Some(()) = (!who.mine && !seen.backlog && !text.trim().is_empty()).then_some(()) else {
        return;
    };
    match device {
        Some(i) => phone_line(r, i, who, text, out),
        None => from_agent(r, m, seen.owners, who, text, out),
    }
}
/// Another agent that signs as itself in a private room is one of its managers (not accepted until it joins with a code).
fn note_manager(r: &mut Room, m: &Value, who: &Who) {
    if r.private()
        && !who.mine
        && m["signed_by"] == "self"
        && !r.devices.iter().any(|d| d.key_id == who.id)
    {
        match r.managers.iter_mut().find(|p| p.key_id == who.id) {
            Some(p) => (p.name, p.last_seen) = (who.from.clone(), who.at.clone()),
            None => r.managers.push(Peer {
                name: who.from.clone(),
                key_id: who.id.clone(),
                public: who.author.clone(),
                since: who.at.clone(),
                last_seen: who.at.clone(),
                ..Default::default()
            }),
        }
    }
}
/// The owner wrote from a paired phone: it releases the brake and goes to the manager as the owner's.
fn phone_line(r: &mut Room, i: usize, who: Who, text: String, out: &mut Out) {
    r.devices[i].last_seen = who.at;
    (r.brake.streak, r.brake.braked) = (0, false);
    out.deliver.push((format!("{} {PHONE}", who.from), text));
}
