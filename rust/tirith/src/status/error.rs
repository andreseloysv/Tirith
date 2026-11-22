//! The last thing that went wrong: an error event or a failed turn.
use super::span::span;
use crate::app::{redact_secrets, Shared};
use chrono::{DateTime, Local};
const LAST: &str = "SELECT at, project, CASE kind WHEN 'error' THEN tool ELSE kind END, summary FROM events
 WHERE kind IN ('error', 'lane.failed', 'lane.not_started', 'deploy.failed', 'main.red', 'factory.invalid')
 UNION ALL
 SELECT ended_at, conv, CASE role WHEN conv THEN '' ELSE role || ' ' END || 'turn failed', COALESCE(answer, '') FROM turns
 WHERE status = 'failed' AND conv NOT LIKE 'shadow-%' AND conv NOT LIKE 'replay-%'
 ORDER BY 1 DESC LIMIT 1";
/// Its age, whose it is, what it was and why, on two lines.
pub(super) fn last_error(app: &Shared, now: DateTime<Local>) -> String {
    let found: Vec<[String; 4]> = app.query(LAST, [], |r| {
        Ok([r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?])
    });
    let Some([at, project, what, why]) = found.first() else {
        return "Last error: none".into();
    };
    let age = DateTime::parse_from_rfc3339(at).map_or(0, |t| now.timestamp() - t.timestamp());
    let whose = crate::bots::bot(app, project)
        .map(|bot| crate::server::s(&bot, "name"))
        .or_else(|| Some(project.clone()).filter(|p| !p.is_empty()))
        .unwrap_or_else(|| "no project".into());
    format!(
        "Last error ({} ago) · {whose} · {what}\n{}",
        span(age.max(0) as u64),
        short(&redact_secrets(why))
    )
    .trim_end()
    .into()
}
/// One line with both its ends: a lane's failure names the lane first and says why last.
fn short(text: &str) -> String {
    let line: Vec<char> = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .collect();
    if line.len() <= 240 {
        return line.into_iter().collect();
    }
    let part = |chars: &[char]| chars.iter().collect::<String>();
    format!(
        "{} … {}",
        part(&line[..100]),
        part(&line[line.len() - 137..])
    )
}
