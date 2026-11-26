//! A lane's run ended: its row in `lane_runs`, made whole as a turn's is (its own share of a session's totals, its model, its price).
use super::{issue::issue_key, read};
use crate::app::{now_iso, Lane, Shared};
use crate::turns::{fresh_input, price_of, share, Usage};
use rusqlite::params;
use std::path::PathBuf;

fn conv(lane: &str) -> String {
    format!("lane:{lane}")
}

/// A lane starts from its brief: its agent's session starts too, and has spent nothing.
pub fn fresh(app: &Shared, lane: &str) {
    crate::turns::fresh(app, &conv(lane));
}

/// Where Copilot writes what the lane's run (in `ws`) used: beside the worktree, gone with it.
fn usage_file(ws: &std::path::Path, attempt: u32) -> PathBuf {
    crate::lanes::temp_of(ws).join(format!("usage-{attempt}.json"))
}

/// The lane's argv, with Copilot told where to write what it used (the other agents print it in their stream).
pub fn counted(
    alias: &str,
    mut argv: Vec<String>,
    ws: &std::path::Path,
    attempt: u32,
) -> Vec<String> {
    let file = usage_file(ws, attempt);
    let told = argv.iter().any(|a| a == "--usage-output-file");
    if alias == "copilot" && !told && std::fs::create_dir_all(crate::lanes::temp_of(ws)).is_ok() {
        let _ = std::fs::remove_file(&file);
        argv.extend(["--usage-output-file".into(), file.display().to_string()]);
    }
    argv
}

/// The row of the run that just ended: nothing known is a row with no tokens and no cost, so the run still counts.
pub fn run_ended(app: &Shared, lane: &Lane) {
    let log = std::fs::read(app.home.join(format!("logs/lanes/{}.log", lane.id)))
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let mut u: Usage = read::from_log(&log);
    let file = usage_file(std::path::Path::new(&lane.workspace), lane.attempts);
    let mut source = if u.seen { "log" } else { "none" };
    if !u.seen && file.exists() {
        read::from_file(&mut u, &file);
        let _ = std::fs::remove_file(&file);
        source = "usage-file";
    }
    let key = conv(&lane.id);
    let _ = app
        .kv(&format!("spent:{key}"))
        .is_none()
        .then(|| fresh(app, &lane.id));
    share(app, (&key, &lane.session), false, &mut u);
    if u.input_has_cache {
        u.input = fresh_input(&u);
    }
    let model = Some(std::mem::take(&mut u.model))
        .filter(|m| !m.is_empty())
        .or_else(|| lane.model.clone())
        .or_else(|| role_model(app, lane))
        .unwrap_or_default();
    let cost = u.cost.or_else(|| price_of(&app.cfg().raw, &model, &u));
    let issue = issue_key(&lane.item);
    app.insert(
        "INSERT INTO lane_runs (lane, project, step, item, issue, attempt, agent, model, started_at, ended_at, tokens_in, tokens_out, tokens_cache_read, tokens_cache_write, cost_usd, source) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![lane.id, lane.project, lane.step, lane.item, issue, lane.attempts, lane.agent, model, lane.started_at, now_iso(), u.input, u.output, u.cache_read, u.cache_write, cost, source],
    );
}

/// The model its lane role names, when the lane runs that role's agent.
fn role_model(app: &Shared, lane: &Lane) -> Option<String> {
    let role = crate::models::role(app, &crate::models::lane_role(&app.cfg()));
    role.model
        .filter(|m| m != "auto" && lane.agent == role.agent)
}
