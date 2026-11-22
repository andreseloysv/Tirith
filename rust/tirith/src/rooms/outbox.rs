//! What Tirith says in rooms by itself, sent in order by one task.
use super::{sealed, settings::base, store::room, store::update, Room};
use crate::app::Shared;
use serde_json::json;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::mpsc;
/// One line to send: (room, text, the keys it is sealed for).
pub(super) type Line = (String, String, Vec<String>);
pub(super) static OUT: OnceLock<mpsc::UnboundedSender<Line>> = OnceLock::new();
/// Opens the outbox and starts its one sender, so a room reads things in order.
pub(super) fn open(app: &Shared) {
    let (tx, rx) = mpsc::unbounded_channel();
    let (_, sender) = (OUT.set(tx), app.clone());
    tokio::spawn(send_all(sender, rx));
}
async fn send_all(app: Shared, mut rx: mpsc::UnboundedReceiver<Line>) {
    while let Some((id, text, readers)) = rx.recv().await {
        send_until_done(&app, &id, &text, &readers).await;
        super::unsent::done(&app, &id, &text);
    }
}
/// While the platform is away all wait (some six hours at most); what it refuses three times is dropped and said.
async fn send_until_done(app: &Shared, id: &str, text: &str, readers: &[String]) {
    let mut tries = 0;
    while let Some(r) = room(id) {
        let Err(e) = send(app, &r, text, readers).await else {
            break;
        };
        tries += 1;
        if tries > 2000
            || (tries >= 3 && !e.starts_with("cannot reach") && !e.starts_with("HTTP 5"))
        {
            app.event("", &r.project, "error", "rooms", &format!("{id}: {e}"), "");
            update(app, id, |r| (r.told.listed.clear(), r.told.roster.clear()));
            break;
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
/// Posts `text` in the room as this Tirith: sealed for `readers` in a private room, plain in a public one.
pub(super) async fn send(
    app: &Shared,
    r: &Room,
    text: &str,
    readers: &[String],
) -> Result<(), String> {
    let (base, me) = (base(app), sealed::identity(&app.home, &r.me)?);
    let readers: Vec<String> = readers
        .iter()
        .filter(|p| !r.managers.iter().any(|m| m.blocked && m.public == **p))
        .cloned()
        .collect(); // forgotten while this waited
    match (r.private(), readers.is_empty()) {
        (false, _) => sealed::post(&base, &me, "message", &r.id, json!(text)).await,
        (true, true) => Err("nobody in this room can read it yet: pair a phone first".into()),
        (true, false) => {
            sealed::post(
                &base,
                &me,
                "message",
                &r.id,
                sealed::seal(&me, text, &format!("#{}", r.id), &readers),
            )
            .await
        }
    }
}
/// Queues `text` for a room, sealed for `readers`; nothing when there is nobody to read it or nothing to say.
pub(super) fn queue(id: &str, text: &str, readers: Vec<String>) {
    let _ = (!text.trim().is_empty() && !readers.is_empty()).then(|| {
        OUT.get()
            .map(|tx| tx.send((id.to_string(), text.to_string(), readers)))
    });
}
