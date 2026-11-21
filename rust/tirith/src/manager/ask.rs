//! `tirith ask-owner`: a new card for the owner.
use super::events::emit_event;
use crate::app::{now_iso, Shared};
use crate::rooms;
use serde_json::{json, Value};
/// `tirith ask-owner`: a card in its project's chat (so on the owner's phones too) and among the open cards of the owner's page, until answered. With `lane` it carries that waiting lane's question: the lane shows it, and the answer goes straight to the lane.
pub fn ask_owner(
    app: &Shared,
    project: &str,
    kind: &str,
    question: &str,
    lane: &str,
) -> Result<Value, String> {
    let project = ask_project(app, project, lane)?;
    let question = question.trim();
    check_kind(app, kind, question)?;
    let row = insert_card(app, &project, kind, question, lane)?;
    app.emit("owner", row.clone());
    let id = row["id"].as_str().unwrap_or("");
    emit_event(
        app,
        &project,
        "card.opened",
        &format!("{id} ({kind}): {question}"),
        None,
    );
    app.chat("card", &format!("{id} ({kind}): {question}"), &project);
    rooms::to_phones(app, &project, &format!("❓ **Needs you** (card {id}, {kind})\n\n{question}\n\nAnswer here with `{id}: <your answer>`"));
    Ok(row)
}
/// The conversation a card belongs to: its waiting lane's project, else the one it was asked in.
fn ask_project(app: &Shared, project: &str, lane: &str) -> Result<String, String> {
    let project = match app.lane(lane) {
        Some(l) if l.status == "waiting" => l.project,
        _ if lane.is_empty() => project.to_string(),
        _ => return Err(format!("lane {lane} is not waiting for an answer")),
    };
    (project.is_empty() || app.conv(&project) == project)
        .then_some(())
        .ok_or(format!("unknown project '{project}'"))?;
    Ok(app.conv(&project).to_string())
}
/// A card has one of the factory's kinds and a question.
fn check_kind(app: &Shared, kind: &str, question: &str) -> Result<(), String> {
    let kinds = app.cfg().factory.words("cards", "kinds");
    (kinds.iter().any(|k| k == kind) && !question.is_empty())
        .then_some(())
        .ok_or_else(|| {
            format!(
                "ask the owner with --kind {} and a question",
                kinds.join("|")
            )
        })
}
/// The card's row, kept in the table under the next free id.
fn insert_card(
    app: &Shared,
    project: &str,
    kind: &str,
    question: &str,
    lane: &str,
) -> Result<Value, String> {
    let (factory, role_hash) = app
        .lane(lane)
        .map(|l| (l.factory, l.role_hash))
        .unwrap_or_else(|| app.stamp_for(&app.chat_role(project))); // a lane's card carries its lane's stamp, else its conversation role's
    let db = app.db.lock().unwrap();
    let n: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(CAST(SUBSTR(id, 2) AS INTEGER)), 0) FROM owner_items",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let row = json!({"id": format!("o{}", n + 1), "at": now_iso(), "kind": kind, "project": project, "question": question, "status": "open", "answer": "", "lane": lane, "factory": factory, "role_hash": role_hash});
    let sql = "INSERT INTO owner_items (id, at, kind, project, question, status, answer, lane, factory, role_hash) VALUES (?1,?2,?3,?4,?5,'open','',?6,?7,?8)";
    db.execute(
        sql,
        rusqlite::params![
            row["id"].as_str(),
            row["at"].as_str(),
            kind,
            project,
            question,
            lane,
            row["factory"].as_str(),
            row["role_hash"].as_str()
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(row)
}
