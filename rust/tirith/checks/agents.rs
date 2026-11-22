// The agents' real output shapes, as measured on 2026-11-22 (Anthropic CLI 2.1.330, Codex 0.171.0, Copilot 1.0.114, Muse 1.4.25).
use super::{record::Turn, settle::settle, spent::fresh};
use serde_json::json;

fn app(name: &str) -> crate::app::Shared {
    let home = std::env::temp_dir().join(format!("tirith-agents-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("config.toml"), "[network]\nsandbox = false\n").unwrap();
    crate::app::App::open_at(home)
}

/// A turn of conversation "c" that read `lines`, settled: (fresh input, output, cache read, dollars, model).
fn turn(
    app: &crate::app::Shared,
    loose: bool,
    lines: &[serde_json::Value],
) -> (i64, i64, i64, Option<f64>, String) {
    let mut t = Turn::new(app, "c", "manager", ("owner".into(), String::new()));
    t.model.clear();
    t.agent = "fake".into();
    t.loose = loose;
    lines.iter().for_each(|l| t.line(l));
    settle(app, &mut t);
    (
        t.usage.input,
        t.usage.output,
        t.usage.cache_read,
        t.usage.cost,
        t.model.clone(),
    )
}

#[test]
fn anthropic_cost_is_the_turns_not_the_sessions() {
    let app = app("cost");
    fresh(&app, "c");
    let result = |cost: f64| {
        json!({"type": "result", "session_id": "s1", "is_error": false, "total_cost_usd": cost,
        "usage": {"input_tokens": 400, "output_tokens": 20, "cache_read_input_tokens": 600, "cache_creation_input_tokens": 0}})
    };
    let near = |got: Option<f64>, want: f64| (got.unwrap() - want).abs() < 1e-9;
    let one = turn(&app, false, &[result(0.000056)]);
    assert_eq!((one.0, one.1, one.2), (400, 20, 600));
    assert!(near(one.3, 0.000056));
    assert!(near(turn(&app, false, &[result(0.000168)]).3, 0.000112));
    // Resumed in a new process, it goes on counting.
    assert!(near(turn(&app, false, &[result(0.000224)]).3, 0.000056));
    // A turn of its own is its whole session, and a session whose spending before nobody knows gives no cost.
    assert!(near(turn(&app, true, &[result(0.0005)]).3, 0.0005));
    let mut other = result(0.9);
    other["session_id"] = json!("s2");
    assert_eq!(turn(&app, false, &[other]).3, None);
}

#[test]
fn codex_tokens_are_the_turns_with_the_cache_apart() {
    let app = app("codex");
    fresh(&app, "c");
    let done = |input: i64, cached: i64, out: i64| {
        [
            json!({"type": "thread.started", "thread_id": "th-1"}),
            json!({"type": "turn.completed", "usage": {"input_tokens": input, "cached_input_tokens": cached, "output_tokens": out}}),
        ]
    };
    let got = |t: (i64, i64, i64, Option<f64>, String)| (t.0, t.1, t.2);
    assert_eq!(got(turn(&app, false, &done(1000, 600, 20))), (400, 20, 600));
    assert_eq!(
        got(turn(&app, false, &done(3000, 1800, 60))),
        (800, 40, 1200)
    );
    assert_eq!(
        got(turn(&app, false, &done(4000, 2400, 80))),
        (400, 20, 600)
    );
}

#[test]
fn copilot_tokens_and_model_come_from_its_usage_file() {
    let app = app("copilot");
    fresh(&app, "c");
    let lines = |input: i64, cached: i64, out: i64| {
        [
            json!({"type": "session.start", "data": {"sessionId": "cp-1", "selectedModel": "auto"}}),
            json!({"type": "result", "sessionId": "cp-1", "exitCode": 0, "usage": {"premiumRequests": 1}}),
            json!({"type": "copilot.usage", "data": {"modelMetrics": {"mai-code-2-flash": {"usage":
            {"inputTokens": input, "outputTokens": out, "cacheReadTokens": cached, "cacheWriteTokens": 0}}}}}),
        ]
    };
    let one = turn(&app, false, &lines(33518, 25600, 36));
    assert_eq!(one, (7918, 36, 25600, None, "mai-code-2-flash".to_string()));
    let two = turn(&app, false, &lines(43518, 32600, 56));
    assert_eq!((two.0, two.1, two.2), (3000, 20, 7000));
}

#[test]
fn muse_run_usage_is_the_turn_and_never_counted_twice() {
    let app = app("muse");
    let call = json!({"payload_type": "runtime.session", "payload": {"event": {"kind": "model_completed", "model": "m",
        "usage": {"input_tokens": 33252, "output_tokens": 60, "cache_read_tokens": 16570}}}});
    let run = json!({"payload_type": "run.usage", "payload": {"models": [
        {"model": "m", "input_tokens": 66504, "cached_tokens": 33140, "output_tokens": 120, "calls": 2}]}});
    let got = turn(&app, true, &[call.clone(), call.clone(), run, call]);
    assert_eq!(got, (33364, 120, 33140, None, "m".to_string()));
}

#[test]
fn captures_name_the_calls_as_the_hooks_do() {
    let mut c = super::capture::Capture::default();
    for line in [
        json!({"type": "item.started", "item": {"id": "i1", "type": "command_execution", "command": "/bin/bash -lc 'curl -s https://a.example/x'"}}),
        json!({"type": "item.completed", "item": {"id": "i1", "type": "command_execution", "aggregated_output": "page"}}),
        json!({"type": "tool.execution_start", "data": {"toolCallId": "t1", "toolName": "create", "arguments": {"path": "a.txt", "file_text": "x"}}}),
        json!({"type": "tool.execution_complete", "data": {"toolCallId": "t1", "result": {"content": "Created"}}}),
    ] {
        c.take(&line);
    }
    assert_eq!(c.calls[0]["tool"], "Bash");
    assert_eq!(
        c.calls[0]["input"]["command"],
        "curl -s https://a.example/x"
    );
    assert_eq!(c.calls[0]["result"], "page");
    assert_eq!(
        (c.calls[1]["tool"].as_str(), c.calls[1]["result"].as_str()),
        (Some("Write"), Some("Created"))
    );
}

#[test]
fn codex_shell_wrappers_come_off() {
    for (shown, asked) in [
        ("/bin/bash -lc 'echo a b'", "echo a b"),
        ("/bin/zsh -lc true", "true"),
        (r#"bash -lc "echo \"hi\" \$HOME""#, r#"echo "hi" $HOME"#),
        (r#"sh -c 'echo it'"'"'s'"#, "echo it's"),
        ("git status", "git status"),
        ("sh -c 'a' b", "sh -c 'a' b"),
    ] {
        assert_eq!(super::shell::inner(shown), asked, "{shown}");
    }
}
