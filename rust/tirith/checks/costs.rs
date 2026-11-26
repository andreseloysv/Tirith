// What lanes spent: the issue key, a run's row from each agent's log, a resumed session's own share, the export.
use super::{
    export::{csv, report},
    issue::issue_key,
    record::{counted, fresh, run_ended},
};
use crate::app::{Lane, Shared};
use serde_json::json;

fn app(name: &str) -> Shared {
    let home = std::env::temp_dir().join(format!("tirith-costs-{name}-{}", std::process::id()));
    std::fs::create_dir_all(home.join("logs/lanes")).unwrap();
    let config = "[network]\nsandbox = false\n[prices.model-a]\ninput = 0.10\ncached = 0.01\noutput = 0.50\n";
    std::fs::write(home.join("config.toml"), config).unwrap();
    crate::app::App::open_at(home)
}

fn lane(id: &str, item: &str, workspace: &str) -> Lane {
    Lane {
        id: id.into(),
        project: "p".into(),
        step: "1".into(),
        item: item.into(),
        agent: "copilot".into(),
        model: Some("model-a".into()),
        workspace: workspace.into(),
        ..Lane::default()
    }
}

#[test]
fn an_issue_key_is_what_the_text_starts_with() {
    for (text, key) in [
        ("ABC-123: do it", "ABC-123"),
        ("[AB2-7] do it", "AB2-7"),
        ("ABC-123 do it", "ABC-123"),
        ("ABC-12x: no", ""),
        ("fix ABC-123", ""),
        ("A-1: one letter", ""),
        ("abc-1: lower", ""),
        ("", ""),
    ] {
        assert_eq!(issue_key(text), key, "{text}");
    }
}

#[test]
fn copilot_is_told_where_to_write_its_usage() {
    let ws = std::env::temp_dir().join(format!("tirith-costs-ws-{}/slot-1", std::process::id()));
    let argv = |alias| counted(alias, vec!["copilot".into()], &ws, 0);
    assert!(argv("copilot").contains(&"--usage-output-file".to_string()));
    assert_eq!(argv("codex").len(), 1, "the others print their usage");
}

#[test]
fn every_run_of_a_lane_is_a_row_and_a_resume_counts_its_own() {
    let app = app("runs");
    let ws = app.home.join("lanes/p/slot-1");
    let log = |lines: &[serde_json::Value]| {
        let mut text = format!("{}now\n", crate::lanes::RUN_START);
        lines.iter().for_each(|l| text += &format!("{l}\n"));
        std::fs::write(app.home.join("logs/lanes/l1.log"), text).unwrap();
    };
    let mut l = lane("l1", "ABC-9: first item", &ws.display().to_string());
    fresh(&app, "l1");
    // Muse prints one `run.usage`, cached tokens apart.
    let run = |input, output| json!({"payload_type": "run.usage", "payload": {"models": [{"model": "model-a", "input_tokens": input, "output_tokens": output, "cached_tokens": 0}]}});
    log(&[run(1_000_000, 100_000)]);
    l.agent = "muse".into();
    run_ended(&app, &l);
    l.attempts = 1;
    log(&[run(2_000_000, 0)]);
    run_ended(&app, &l);
    let all = report(&app, "", "", "item");
    let row = &all["rows"][0];
    assert_eq!(
        (row["runs"].as_i64(), row["issue"].as_str()),
        (Some(2), Some("ABC-9"))
    );
    assert_eq!(row["tokens_in"], 3_000_000);
    let cost = row["cost_usd"].as_f64().unwrap();
    assert!((cost - (0.1 * 3.0 + 0.5 * 0.1)).abs() < 1e-9, "{cost}");
    let by_run = report(&app, "p", "", "run");
    assert_eq!(by_run["rows"].as_array().unwrap().len(), 2);
    let text = csv(&report(&app, "", "", "issue"));
    assert!(
        text.starts_with("project,issue,runs,priced_runs,tokens_in"),
        "{text}"
    );
    assert!(text.contains("p,ABC-9,2,2,3000000,"), "{text}");
}

#[test]
fn copilots_resumed_session_counts_only_the_new_run() {
    let app = app("resume");
    let ws = app.home.join("lanes/p/slot-1");
    let mut l = lane("l2", "no key here", &ws.display().to_string());
    l.session = "s1".into();
    fresh(&app, "l2");
    std::fs::write(
        app.home.join("logs/lanes/l2.log"),
        format!("{}now\n", crate::lanes::RUN_START),
    )
    .unwrap();
    let file = |attempt: u32, input: i64, output: i64| {
        let usage = json!({"inputTokens": input, "outputTokens": output, "cacheReadTokens": 0, "cacheWriteTokens": 0});
        let data = json!({"modelMetrics": {"model-a": {"usage": usage}}});
        let path = counted("copilot", vec![], &ws, attempt)[1].clone();
        std::fs::write(path, data.to_string()).unwrap();
    };
    file(0, 1_000_000, 100_000);
    run_ended(&app, &l);
    l.attempts = 1;
    file(1, 3_000_000, 100_000); // the session's totals so far: the second run used 2M in, 0 out
    run_ended(&app, &l);
    let runs = report(&app, "p", "", "run");
    let rows = runs["rows"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .map(|r| r["tokens_in"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        [1_000_000, 2_000_000]
    );
    assert_eq!(rows[1]["tokens_out"], 0);
    let total = report(&app, "p", "", "item")["total"].clone();
    assert!(
        (total["cost_usd"].as_f64().unwrap() - 0.35).abs() < 1e-9,
        "{total}"
    );
}

#[test]
fn a_copilot_runs_cost_counts_its_fresh_input_once() {
    let app = app("fresh");
    let ws = app.home.join("lanes/p/slot-1");
    let l = lane("l3", "no key", &ws.display().to_string());
    fresh(&app, "l3");
    std::fs::write(
        app.home.join("logs/lanes/l3.log"),
        format!("{}now\n", crate::lanes::RUN_START),
    )
    .unwrap();
    // Copilot's input holds its cache: 14 fresh, 124810 read, 27404 written.
    let usage = json!({"inputTokens": 152228, "outputTokens": 2154, "cacheReadTokens": 124810, "cacheWriteTokens": 27404});
    let path = counted("copilot", vec![], &ws, 0)[1].clone();
    std::fs::write(
        path,
        json!({"modelMetrics": {"model-a": {"usage": usage}}}).to_string(),
    )
    .unwrap();
    run_ended(&app, &l);
    let row = &report(&app, "p", "", "run")["rows"][0];
    assert_eq!(row["tokens_in"], 14);
    let dollars = (14.0 * 0.10 + 27404.0 * 0.10 + 124810.0 * 0.01 + 2154.0 * 0.50) / 1e6;
    assert!(
        (row["cost_usd"].as_f64().unwrap() - dollars).abs() < 1e-6,
        "{row}"
    );
}
