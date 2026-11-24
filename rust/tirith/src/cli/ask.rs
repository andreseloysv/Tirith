//! Questions and answers: a lane asks the manager, the manager answers a lane or a card, the manager asks the owner.
use crate::client::{call, print, Flag};
use serde_json::json;

pub(super) fn ask(question: &str) -> Result<(), String> {
    let lane = std::env::var("TIRITH_LANE").map_err(|_| "neobot ask works inside a lane (TIRITH_LANE); the manager asks the owner with neobot ask-owner".to_string());
    lane.and_then(|l| {
        call(
            "POST",
            &format!("/api/lanes/{l}/ask"),
            Some(json!({"question": question})),
        )
    })
    .map(|_| println!("Asked the manager. End your turn now; you will be resumed with the answer."))
}

pub(super) fn answer(id: &str, answer: &str) -> Result<(), String> {
    // A card (o12) the owner answered in the chat is closed with their answer; anything else is a lane.
    let card = id
        .strip_prefix('o')
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    let path = if card {
        format!("/api/owner/{id}")
    } else {
        format!("/api/lanes/{id}/answer")
    };
    print(call(
        "POST",
        &path,
        Some(json!({"answer": answer, "by": "manager"})),
    ))
}

pub(super) fn ask_owner(question: &str, project: &str, flag: Flag) -> Result<(), String> {
    if std::env::var("TIRITH_LANE").is_ok_and(|l| !l.is_empty()) {
        Err(
            "a lane does not ask the owner: ask the manager with neobot ask \"<question>\""
                .to_string(),
        )
    } else {
        let body = json!({"kind": flag("--kind").unwrap_or_default(), "question": question, "project": project, "lane": flag("--lane").unwrap_or_default()});
        call("POST", "/api/owner", Some(body)).map(|b| println!("{b}"))
    }
}
