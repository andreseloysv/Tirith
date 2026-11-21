//! The owner's `/model` and `/models` chat commands.
use super::save::{line, save};
use super::{owners::agent_of, roles::role, updated::updated};
use crate::app::Shared;
use serde_json::{json, Value};
/// Deterministic owner commands, available even when no model can answer.
pub fn command(app: &Shared, text: &str) -> Option<Result<String, String>> {
    let words: Vec<&str> = text.split_whitespace().collect();
    matches!(
        words.first(),
        Some(&"/model" | &"/models" | &"/modelo" | &"/modelos")
    )
    .then_some(())?;
    let all_roles = app.cfg().factory.roles();
    let help = format!("Machine-wide models (all projects).\n/models — current settings and Codex models\n/model <{}|all> <agent>:<model> [effort]\n/model <role> reset — back to config.toml\nExample: /model manager codex (Codex's default model). Changes apply to the next turn or new lanes; running work finishes first.", all_roles.join("|"));
    Some(if words.len() == 1 || words.get(1) == Some(&"help") {
        Ok(overview(app, &all_roles, &help))
    } else {
        change(app, &words, &all_roles, help)
    })
}
/// Every role's setting, Codex's models with their efforts, and the help.
fn overview(app: &Shared, all_roles: &[String], help: &str) -> String {
    let roles = all_roles
        .iter()
        .map(|r| format!("{r}: {}", line(&role(app, r))))
        .collect::<Vec<_>>()
        .join("\n");
    let catalog: Vec<String> = crate::codex::catalog()
        .iter()
        .map(|m| {
            format!(
                "{} ({})",
                m["slug"].as_str().unwrap_or(""),
                m["efforts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    format!("{roles}\n\nCodex models:\n{}\n\n{help}", catalog.join("\n"))
}
/// `/model <role|all> <agent>:<model> [effort]` or `/model <role> reset`: every role is checked before any is saved.
fn change(
    app: &Shared,
    words: &[&str],
    all_roles: &[String],
    help: String,
) -> Result<String, String> {
    ((3..=4).contains(&words.len()))
        .then_some(())
        .ok_or_else(|| help.clone())?;
    let r = if words[1] == "todos" { "all" } else { words[1] };
    let roles: Vec<String> = match r {
        "all" => all_roles.to_vec(),
        _ if all_roles.iter().any(|x| x == r) => vec![r.to_string()],
        _ => return Err(help),
    };
    let reset = words[2] == "reset";
    (!reset || words.len() == 3)
        .then_some(())
        .ok_or_else(|| help.clone())?;
    let body = if reset { json!({}) } else { body(app, words) };
    if !reset {
        roles
            .iter()
            .try_for_each(|r| updated(app, r, &body).map(|_| ()))?; // validate every role before saving any of them
    }
    let lines: Vec<String> = roles
        .into_iter()
        .map(|r| save(app, &r, (!reset).then_some(&body)).map(|s| format!("{r}: {}", line(&s))))
        .collect::<Result<_, _>>()?;
    Ok(format!("Saved for all projects:\n{}\nApplies to the next turn or new lanes; running work finishes first.", lines.join("\n")))
}
/// The setting `<agent>:<model> [effort]` asks for; a model alone brings its own agent.
fn body(app: &Shared, words: &[&str]) -> Value {
    let (agent, model) = words[2]
        .split_once(':')
        .map(|(a, m)| (a.to_string(), m))
        .unwrap_or_else(|| {
            agent_of(&app.cfg(), words[2]).map_or((words[2].to_string(), ""), |a| (a, words[2]))
        });
    json!({"agent": agent, "model": model, "effort": words.get(3).copied().unwrap_or("")})
}
