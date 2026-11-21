//! Pairing: a phone scans the one-use QR (`tirith-pair <code>`), another manager joins with its one-use code (`tirith-join <code>`).
use super::handle::{Seen, Who};
use super::{keeper::WAKE, poll::Out, sealed, Peer, Room, CODE_SECS};
use crate::app::now_s;
/// True when the message was a pairing or a join, which go no further.
pub(super) fn pairing(r: &mut Room, seen: &Seen, who: &Who, text: &str, out: &mut Out) -> bool {
    let word = text.split_whitespace().next().unwrap_or("");
    if r.private() && !who.mine && word == "tirith-pair" {
        pair(r, seen, who, text, out);
        return true;
    }
    let join = r.private() && !who.mine && word == "tirith-join";
    let _ = (join && r.kind == "manager" && burn(&mut r.codes.peer_code, text, "tirith-join"))
        .then(|| {
            r.managers
                .iter_mut()
                .find(|p| p.key_id == who.id)
                .into_iter()
                .for_each(|p| p.accepted = true)
        });
    join
}
/// A phone that is here already may scan again: its code is used up and it is told so once more.
fn pair(r: &mut Room, seen: &Seen, who: &Who, text: &str, out: &mut Out) {
    if r.kind == "manager"
        && now_s().saturating_sub(r.codes.invite_at) < CODE_SECS
        && burn(&mut r.codes.invite, text, "tirith-pair")
    {
        r.managers.retain(|p| p.key_id != who.id);
        let _ = (!r.devices.iter().any(|d| d.key_id == who.id))
            .then(|| {
                r.devices.push(Peer {
                    name: who.from.clone(),
                    key_id: who.id.clone(),
                    public: who.author.clone(),
                    since: who.at.clone(),
                    last_seen: who.at.clone(),
                    accepted: true,
                    blocked: false,
                })
            })
            .map(|_| WAKE.notify_one()); // it gets every project's room
        (r.brake.streak, r.brake.braked) = (0, false);
        out.send.push((
            format!("Connected to {} on {}", seen.me.name, seen.host),
            vec![who.author.clone()],
        ));
    }
}
/// True, and the code is used up, when `text` is `<prefix> <code>` with the room's current code (the last `:` part of `code`).
pub(super) fn burn(code: &mut String, text: &str, prefix: &str) -> bool {
    let (want, parts) = (
        code.rsplit(':').next().unwrap_or("").to_string(),
        text.split_whitespace().collect::<Vec<_>>(),
    );
    let ok = parts.len() == 2
        && parts[0] == prefix
        && want.len() == 32
        && parts[1].len() == 32
        && want
            .bytes()
            .zip(parts[1].bytes())
            .fold(0, |acc, (x, y)| acc | (x ^ y))
            == 0;
    ok.then(|| code.clear()).is_some()
}
pub(super) fn new_code() -> String {
    sealed::hex(&rand::random::<[u8; 16]>())
}
