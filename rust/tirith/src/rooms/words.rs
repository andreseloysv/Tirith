//! What a message says: its words (unsealed, a picture saved as a file), and whether it is a control rather than a line for people.
use super::{files::save_image_at, handle::Who, poll::Out, sealed, Room};
use serde_json::Value;
/// The words of a message event: plain in a public room, sealed for me in any room; `None` for any other event.
pub(super) fn words(r: &Room, me: &sealed::Me, e: &Value) -> Option<String> {
    (e["kind"] == "message").then_some(())?;
    let text = match (&e["body"], r.private()) {
        (Value::String(s), false) => Some(s.clone()),
        (Value::Object(_), _) => sealed::unseal(me, e),
        _ => None,
    }?;
    Some(
        text.strip_prefix("tirith-image ")
            .map_or(text.clone(), |j| {
                save_image_at(&crate::config::tirith_home(), j, &r.id)
                    .unwrap_or_else(|| "[an image that could not be read]".into())
            }),
    )
}
/// True for the controls the phones and OpenBot exchange: none of them is a line for people. A paired phone's `tirith-models` request is answered.
pub(super) fn not_for_people(r: &Room, who: &Who, text: &str, out: &mut Out) -> bool {
    let models = text.strip_prefix("tirith-models ");
    models.into_iter().for_each(|body| {
        (r.kind == "manager" && r.devices.iter().any(|d| d.key_id == who.id))
            .then(|| serde_json::from_str(body).ok())
            .flatten()
            .into_iter()
            .for_each(|request| out.controls.push((who.author.clone(), request)))
    });
    let word = text.split_whitespace().next().unwrap_or("");
    models.is_some()
        || matches!(
            word,
            "tirith-roster" | "tirith-rooms" | "tirith-models-state" | "tirith-chat"
        )
        || (word == "tirith-join" && who.mine)
}
