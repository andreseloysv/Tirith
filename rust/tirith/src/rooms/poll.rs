//! One long poll of a room: read, handle each new message once (the cursor moves past it), answer.
use super::handle::{handle, Seen};
use super::settings::{base, host, owners};
use super::{deliver::act, roster::roster, sealed, store::update, Room};
use crate::app::Shared;
use serde_json::Value;
/// What one poll decided: sealed or plain messages to send (text, readers), and turns for the manager.
#[derive(Default)]
pub(super) struct Out {
    pub(super) controls: Vec<(String, Value)>,
    pub(super) send: Vec<(String, Vec<String>)>,
    pub(super) deliver: Vec<(String, String)>,
    pub(super) mentions: Vec<Value>,
    /// What the room's webhook brought (`tirith-hook` JSON), for its bot.
    pub(super) hooks: Vec<String>,
}
pub(super) async fn poll(app: &Shared, r: &Room) -> Result<(), String> {
    let base = base(app);
    let me = sealed::identity(&app.home, &r.me)?;
    let platform = sealed::platform(&base).await?;
    let after = r.cursor.or(if r.private() { Some(0) } else { None });
    let got = sealed::page(
        &base,
        &r.id,
        after,
        if r.cursor.is_some() { 25 } else { 0 },
        &me.name,
    )
    .await?;
    let list = got["messages"].as_array().cloned().unwrap_or_default();
    let (owners, host) = (owners(app), host());
    let hook_from = crate::bots::hook_from(app, &r.project);
    let seen = Seen {
        me: &me,
        platform: &platform,
        backlog: r.cursor.is_none() && !r.private(),
        owners: &owners,
        host: &host,
        hook_from: &hook_from,
    };
    let mut out = Out::default();
    let here = here(&got);
    let Some(()) = update(app, &r.id, |r| take_in(r, &list, here, &seen, &mut out)) else {
        return Ok(());
    };
    act(app, r, out);
    Ok(())
}
/// Who the platform says is in the room now.
fn here(got: &Value) -> Vec<String> {
    got["here"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|h| h["name"].as_str().map(String::from))
        .collect()
}
/// The new messages go into the record (not saved here: the caller's `update` saves).
fn take_in(r: &mut Room, list: &[Value], here: Vec<String>, seen: &Seen, out: &mut Out) {
    r.here = here;
    for m in list {
        let n = m["n"].as_u64().unwrap_or(0);
        let Some(()) = (!r.cursor.is_some_and(|c| n <= c)).then_some(()) else {
            continue;
        };
        r.cursor = Some(n);
        handle(r, seen, m, out);
    }
    roster(r, out);
    // Public rooms are only watched: nothing said there reaches the manager (it writes there with `neobot room say`).
    out.deliver.retain(|_| r.private());
    r.owes.owed.extend(out.deliver.iter().cloned()); // saved with the cursor: a restart in between loses nothing
}
