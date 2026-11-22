use rusqlite::params;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

const KEY: &str = "bot-mail01";
const TASK: &str = "Routine «gmail-safari» is due now. Check the inbox and list the new offers";

/// A fake stream-json agent. As the bot, it asks Tirith's hook (over HTTP, as an agent's hooks do) for two reads of the web, a search, a local read, a write in its folder and a `tirith say`, keeps what the hook answered in `<home>/heard-<conversation>.txt` and what it was handed in `told-<conversation>.txt`, and answers. As anyone else (the judge) it says «tie».
const AGENT: &str = r#"#!/bin/sh
read -r line
if [ "$TIRITH_ROLE" != "KEY" ]; then
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"tie — both list the same offers"}]}}'
  echo '{"type":"result","subtype":"success","is_error":false,"usage":{"input_tokens":3,"output_tokens":2}}'
  exit 0
fi
here=$(pwd)
out="$TIRITH_HOME/heard-$TIRITH_PROJECT.txt"
hook() {
  curl -s --noproxy '*' -X POST "http://127.0.0.1:PORT/hook/PreToolUse" -H 'Host: 127.0.0.1' -H "X-Tirith-Role: $TIRITH_ROLE" -H 'Content-Type: application/json' --data-binary "{\"cwd\":\"$here\",\"session_id\":\"r\",\"tool_name\":\"$1\",\"tool_input\":$2}" >> "$out"
  echo >> "$out"
}
hook WebFetch '{"url":"https://mail.example/inbox"}'
hook WebFetch '{"url":"https://mail.example/inbox?page=2"}'
hook WebSearch '{"query":"new offers"}'
hook Bash '{"command":"cat memory.md"}'
hook Write "{\"file_path\":\"$here/notes.md\",\"content\":\"seen\"}"
hook Bash '{"command":"tirith say --project KEY hola"}'
printf '%s\n' "$line" > "$TIRITH_HOME/told-$TIRITH_PROJECT.txt"
echo '{"type":"assistant","message":{"content":[{"type":"text","text":"3 offers listed"}]}}'
echo '{"type":"result","subtype":"success","is_error":false,"usage":{"input_tokens":10,"output_tokens":5}}'
"#;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=Coach", "-c", "user.email=coach@localhost"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A Tirith home with the bot «Correo» and its routine `r1`, on the fake agent (the judge too) that calls the hook on `port`.
fn app_with_a_bot(name: &str, port: u16) -> (crate::app::Shared, PathBuf) {
    app_with_agent(name, port, AGENT)
}

/// `app_with_a_bot` with another fake agent.
fn app_with_agent(name: &str, port: u16, agent_script: &str) -> (crate::app::Shared, PathBuf) {
    let home = std::env::temp_dir().join(format!("tirith-replay-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let agent = home.join("fake-agent.sh");
    let script = agent_script
        .replace("PORT", &port.to_string())
        .replace("KEY", KEY);
    std::fs::write(&agent, script).unwrap();
    std::fs::set_permissions(&agent, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!("[agents.fake]\ncommand = \"{}\"\nchat_args = []\n[network]\nsandbox = false\n\n[roles.{KEY}]\nextends = \"manager\"\nscope = \"own\"\nsession = \"chat\"\nname = \"Correo\"\n\n[[routes]]\non = \"schedule\"\nto = \"{KEY}\"\nmode = \"turn\"\nid = \"r1\"\ntitle = \"gmail-safari\"\ntask = \"Check the inbox and list the new offers\"\ndays = [1]\nat = \"08:00\"\n", agent.display()),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    for role in [KEY, "judge"] {
        app.set_kv(&format!("models:{role}"), Some(r#"{"agent":"fake"}"#));
    }
    app.set_kv("test:network-closed", Some("1")); // the fake agent runs with no sandbox here
    let dir = home.join("manager").join(KEY);
    std::fs::create_dir_all(dir.join(".agents/skills/inbox")).unwrap();
    std::fs::write(dir.join("memory.md"), "# Memory\n- open every mail\n").unwrap();
    std::fs::write(
        dir.join(".agents/skills/inbox/SKILL.md"),
        "open each mail\n",
    )
    .unwrap();
    crate::bots::versioned(&app, KEY, "library");
    (app, home)
}

/// One run of routine `routine` as its agent saw it — the task, a web read with `result`, a Muse shell call — kept with its capture.
fn captured_run(app: &crate::app::Shared, routine: &str, result: &str) -> i64 {
    crate::turns::told(KEY, TASK);
    let mut t = crate::turns::Turn::new(app, KEY, KEY, ("schedule".into(), routine.into()));
    t.line(&json!({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": "tu1", "name": "WebFetch", "input": {"url": "https://mail.example/inbox"}}]}}));
    t.line(&json!({"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": "tu1", "content": [{"type": "text", "text": result}]}]}}));
    t.line(&json!({"payload_type": "tool.result", "payload": {"correlation_facts": {"tool_name": "bash"}, "text": "{\"command\":\"curl -s https://api.example/offers\",\"output\":\"2 offers\"}"}}));
    let id = app.insert("INSERT INTO turns (conv, role, \"trigger\", routine, started_at, status) VALUES (?1, ?1, 'schedule', ?2, '2026-11-21T08:00:00Z', 'ok')", params![KEY, routine]);
    crate::turns::keep_capture(app, id, &t);
    id
}

/// What the Coach does: a branch with one commit on the bot's main.
fn branch(home: &Path, name: &str, memory: &str) {
    let dir = home.join("manager").join(KEY);
    let draft = home.join("lab").join(KEY).join("draft");
    git(
        &dir,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            name,
            &draft.display().to_string(),
        ],
    );
    std::fs::write(draft.join("memory.md"), memory).unwrap();
    git(&draft, &["add", "-A"]);
    git(
        &draft,
        &[
            "commit",
            "-q",
            "-m",
            &format!("{name}: search with a filter"),
        ],
    );
    git(&dir, &["worktree", "remove", &draft.display().to_string()]);
}

#[test]
fn captures_keep_the_last_ten() {
    let (app, home) = app_with_a_bot("keep", 0);
    let ids: Vec<i64> = (0..12)
        .map(|i| captured_run(&app, "r1", &format!("{i} new offers")))
        .collect();
    let kept = || {
        let mut v: Vec<i64> = std::fs::read_dir(home.join("captures"))
            .unwrap()
            .flatten()
            .filter_map(|e| e.path().file_stem()?.to_str()?.parse().ok())
            .collect();
        v.sort();
        v
    };
    assert_eq!(kept(), ids[2..].to_vec(), "the last 10 runs of the routine");
    let c = crate::turns::capture_of(&app, ids[11]).unwrap();
    assert_eq!(c.start, TASK, "what started it");
    assert_eq!(
        (
            &c.calls[0]["tool"],
            &c.calls[0]["input"]["url"],
            &c.calls[0]["result"]
        ),
        (
            &json!("WebFetch"),
            &json!("https://mail.example/inbox"),
            &json!("11 new offers")
        ),
        "a call with its input and its result as the agent saw them"
    );
    assert_eq!(c.calls[1]["tool"], "bash");
    assert_eq!(
        c.calls[1]["input"]["command"],
        "curl -s https://api.example/offers"
    );
    assert!(
        c.calls[1]["result"].as_str().unwrap().contains("2 offers"),
        "Muse's tool.result"
    );
    assert_eq!(
        crate::turns::captured(&app, KEY, "r1", 8),
        ids[4..].to_vec(),
        "a replay's runs, oldest first"
    );

    // A turn outside the routine keeps its own, without evicting the routine's; and a capture stops at 256 KB.
    let big = captured_run(&app, "", &"é".repeat(300_000));
    assert_eq!(kept().len(), 11);
    let size = std::fs::metadata(home.join(format!("captures/{big}.jsonl")))
        .unwrap()
        .len();
    assert!(size <= 256 * 1024 + 1024, "{size} bytes");
    let c = crate::turns::capture_of(&app, big).unwrap();
    let cut = c.calls[0]["result"].as_str().unwrap();
    assert!(
        cut.len() < 300_000 && cut.chars().all(|ch| ch == 'é'),
        "cut on a character"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

/// A bot with two captured runs, a branch, a server for its hooks and a replay of it run to its end: the app, its home and the experiment.
async fn replayed(name: &str) -> (crate::app::Shared, PathBuf, i64, Vec<i64>) {
    replayed_with(name, AGENT).await
}

/// `replayed`, the bot's fake agent being `script`.
async fn replayed_with(name: &str, script: &str) -> (crate::app::Shared, PathBuf, i64, Vec<i64>) {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let (app, home) = app_with_agent(name, port, script);
    let app2 = app.clone();
    tokio::spawn(
        async move { drop(axum::serve(listener, crate::server::router(app2, port)).await) },
    );
    let b = json!({"bot": KEY, "branch": "v3", "routine": "r1", "mode": "replay", "hypothesis": "a filter: −40 % tokens"});
    branch(
        &home,
        "v3",
        "# Memory\n- search with a filter, never open each mail\n",
    );
    let none = super::start(&app, &b).unwrap_err();
    assert!(none.contains("no captured run"), "{none}");
    let runs = vec![
        captured_run(&app, "r1", "3 new offers: Ana, Luis, Marta"),
        captured_run(&app, "r1", "1 new offer: Pablo"),
    ];
    let v = super::start(&app, &b).unwrap();
    assert_eq!(
        (v["mode"].as_str(), v["replays"].as_i64()),
        (Some("replay"), Some(2)),
        "{v}"
    );
    let e = v["id"].as_i64().unwrap();
    assert!(super::start(&app, &b)
        .unwrap_err()
        .contains("one at a time"));
    for _ in 0..600 {
        if super::list(&app, KEY)[0]["status"] != "running" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    (app, home, e, runs)
}

#[tokio::test(flavor = "multi_thread")]
async fn replay_answers_from_the_capture() {
    let (app, home, e, runs) = replayed("answers").await;
    for (turn, result) in runs
        .iter()
        .zip(["3 new offers: Ana, Luis, Marta", "1 new offer: Pablo"])
    {
        for side in ["A", "B"] {
            let conv = format!("replay-{e}-{turn}-{side}");
            let heard = std::fs::read_to_string(home.join(format!("heard-{conv}.txt"))).unwrap();
            let said: Vec<serde_json::Value> = heard
                .lines()
                .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{conv}: {e}: {heard}")))
                .collect();
            let why = |i: usize| {
                said[i]["hookSpecificOutput"]["permissionDecisionReason"]
                    .as_str()
                    .unwrap_or("")
                    .to_string()
            };
            let denied = |i: usize| said[i]["hookSpecificOutput"]["permissionDecision"] == "deny";
            assert_eq!(
                why(0),
                format!("Recorded result (replay): {result}"),
                "{conv}: the web read gets the recorded result"
            );
            assert!(
                denied(1) && why(1).contains("not available in a replay"),
                "{conv}: a second read the capture does not have: {}",
                why(1)
            );
            assert!(
                denied(2) && why(2).contains("not available in a replay"),
                "{conv}: a search the capture does not have"
            );
            assert!(
                !denied(3) && !denied(4),
                "{conv}: its folder reads and writes as always: {} / {}",
                said[3],
                said[4]
            );
            assert!(
                denied(5) && why(5).contains("not available in a replay"),
                "{conv}: tirith say"
            );
            let told = std::fs::read_to_string(home.join(format!("told-{conv}.txt"))).unwrap();
            assert!(
                told.contains("Check the inbox and list the new offers"),
                "{conv}: the captured task"
            );
            let memory = if side == "B" {
                "search with a filter"
            } else {
                "open every mail"
            };
            assert!(
                told.contains(memory),
                "{conv}: its own worktree's memory: {told}"
            );
        }
    }
    let rows: Vec<(String, String, String)> = app.query(
        "SELECT variant, \"trigger\", routine FROM turns WHERE experiment = ?1 ORDER BY id",
        [e],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    );
    let want = |v: &str| (v.to_string(), "replay".to_string(), "r1".to_string());
    assert_eq!(rows, vec![want("A"), want("B"), want("A"), want("B")]);
    let pairs: Vec<i64> = app.query(
        "SELECT b_turn - a_turn FROM lab_comparisons WHERE experiment = ?1",
        [e],
        |r| r.get(0),
    );
    assert_eq!(pairs, vec![1, 1], "each B compared with its A");
    let row = &super::list(&app, KEY)[0];
    assert!(
        matches!(row["status"].as_str(), Some("adopted" | "discarded")),
        "decided by the rule: {row}"
    );
    assert_eq!(
        (row["mode"].as_str(), row["compared"].as_i64()),
        (Some("replay"), Some(2)),
        "{row}"
    );
    let lab = home.join("lab").join(KEY).join(e.to_string());
    assert!(!lab.exists(), "the throwaway worktrees go");
    // The network is closed to a replay's turn: its processes reach the proxy as `replay:<experiment>`, refused everything.
    assert_eq!(crate::lab::proxy_caller(KEY), KEY);
    let r = super::replay::Replay {
        experiment: e,
        routine: "r1".into(),
        dir: home.clone(),
    };
    let inside = super::replay::REPLAY
        .scope(r, async { crate::lab::proxy_caller(KEY) })
        .await;
    assert!(
        crate::lab::closed(&inside) && !crate::lab::closed(KEY),
        "{inside}"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test(flavor = "multi_thread")]
async fn replay_reaches_nobody() {
    let (app, home, e, _) = replayed("nobody").await;
    let chat: Vec<(String, String)> = app.query("SELECT project, text FROM chat", [], |r| {
        Ok((r.get(0)?, r.get(1)?))
    });
    assert!(
        chat.iter()
            .all(|(p, t)| p == KEY && (t.starts_with("Adopté") || t.starts_with("Descarté"))),
        "only the lab's decision line, nothing of the replay's turns: {chat:?}"
    );
    let cards: i64 = app
        .query("SELECT COUNT(*) FROM owner_items", [], |r| r.get(0))
        .remove(0);
    assert_eq!(cards, 0, "no card");
    let conv = format!("replay-{e}-1-A");
    assert!(
        crate::manager::say(&app, &conv, "hola", "").is_err(),
        "a replay's conversation is no one's"
    );
    assert!(crate::manager::ask_owner(&app, &conv, "decision", "¿sigo?", "").is_err());
    let turns: Vec<String> = app.query("SELECT conv FROM turns WHERE experiment = ?1", [e], |r| {
        r.get(0)
    });
    assert!(
        turns.iter().all(|c| c.starts_with("replay-")),
        "its turns are in conversations of their own: {turns:?}"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

/// An agent that asks only for what the captures hold: the inbox, with a different prompt and a slash at the end of the address.
const FAIR: &str = r#"#!/bin/sh
read -r line
if [ "$TIRITH_ROLE" != "KEY" ]; then
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"tie — both list the same offers"}]}}'
  echo '{"type":"result","subtype":"success","is_error":false,"usage":{"input_tokens":3,"output_tokens":2}}'
  exit 0
fi
here=$(pwd)
out="$TIRITH_HOME/heard-$TIRITH_PROJECT.txt"
curl -s --noproxy '*' -X POST "http://127.0.0.1:PORT/hook/PreToolUse" -H 'Host: 127.0.0.1' -H "X-Tirith-Role: $TIRITH_ROLE" -H 'Content-Type: application/json' --data-binary "{\"cwd\":\"$here\",\"session_id\":\"r\",\"tool_name\":\"WebFetch\",\"tool_input\":{\"url\":\"https://mail.example/inbox/\",\"prompt\":\"list the offers\"}}" >> "$out"
echo '{"type":"assistant","message":{"content":[{"type":"text","text":"3 offers listed"}]}}'
echo '{"type":"result","subtype":"success","is_error":false,"usage":{"input_tokens":10,"output_tokens":5}}'
"#;

#[tokio::test(flavor = "multi_thread")]
async fn replay_compares_only_what_asked_for_the_same() {
    // Both sides ask for the captured inbox (another prompt, a slash more): it is answered, and the pairs are compared and judged.
    let (app, home, e, _) = replayed_with("fair", FAIR).await;
    let tally = |v: &str| -> i64 {
        let sql = "SELECT COUNT(*) FROM lab_comparisons WHERE experiment = ?1 AND verdict = ?2";
        app.query(sql, params![e, v], |r| r.get(0)).remove(0)
    };
    assert_eq!((tally("tie"), tally("void")), (2, 0));
    let heard = std::fs::read_to_string(home.join(format!("heard-replay-{e}-1-A.txt"))).unwrap();
    assert!(
        heard.contains("Recorded result (replay): 3 new offers"),
        "{heard}"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test(flavor = "multi_thread")]
async fn replay_pairs_that_asked_for_more_prove_nothing() {
    // The default fake agent also asks for a second page and a search the captures lack: every pair proves nothing.
    let (app, home, e, _) = replayed("void").await;
    let verdicts: Vec<Option<String>> = app.query(
        "SELECT verdict FROM lab_comparisons WHERE experiment = ?1",
        [e],
        |r| r.get(0),
    );
    assert_eq!(
        verdicts,
        vec![Some("void".to_string()); 2],
        "never a tie or a win for B"
    );
    let row = &super::list(&app, KEY)[0];
    let line = row["numbers"]["line"].as_str().unwrap();
    assert_eq!(row["status"], "discarded", "{row}");
    assert!(
        line.contains("sin comparaciones") && line.contains("2 par(es) sin comparar"),
        "{line}"
    );
    assert_eq!(row["numbers"]["tally"]["voids"], 2, "{row}");
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_call_is_the_same_by_what_it_asks_for() {
    let key = |t: &str, i: serde_json::Value| super::call_key::key(t, &i);
    assert_eq!(
        key(
            "WebFetch",
            json!({"url": "https://a.example/x/", "prompt": "one"})
        ),
        key(
            "WebFetch",
            json!({"url": "https://a.example/x", "prompt": "two"})
        )
    );
    assert_ne!(
        key("WebFetch", json!({"url": "https://a.example/x"})),
        key("WebFetch", json!({"url": "https://a.example/y"}))
    );
    assert_eq!(
        key("WebSearch", json!({"query": " new  offers "})),
        key("WebSearch", json!({"query": "new offers"}))
    );
    assert_eq!(
        key(
            "Bash",
            json!({"command": "curl  -s x\n", "description": "fetch"})
        ),
        key(
            "Bash",
            json!({"command": "curl -s x", "description": "other words"})
        )
    );
    assert_ne!(
        key("Bash", json!({"command": "curl -s x"})),
        key("Bash", json!({"command": "curl -s y"}))
    );
    assert_eq!(
        key("mcp__x__go", json!({"a": 1, "reason": "why"})),
        key("mcp__x__go", json!({"a": 1}))
    );
}

#[test]
fn a_replay_needs_the_network_closed() {
    let (app, home) = app_with_a_bot("closed", 0);
    app.set_kv("test:network-closed", None);
    branch(&home, "v3", "# Memory\n- a filter\n");
    captured_run(&app, "r1", "3 new offers");
    let b =
        json!({"bot": KEY, "branch": "v3", "routine": "r1", "mode": "replay", "hypothesis": "x"});
    let refused = super::start(&app, &b).unwrap_err();
    assert!(
        refused.contains("network closed") && refused.contains("[network] sandbox = false"),
        "{refused}"
    );
    assert!(super::list(&app, KEY).is_empty(), "no experiment was made");
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn a_replays_branch_may_change_the_routines_effort() {
    let (app, home) = app_with_a_bot("tuned", 0);
    app.set_kv(
        &format!("models:{KEY}"),
        Some(r#"{"agent":"fake","effort":"high"}"#),
    );
    // B's worktree carries the branch's `routines.toml`: its turn runs with that effort, and no other turn does.
    let tree = home.join("tree-b");
    std::fs::create_dir_all(&tree).unwrap();
    std::fs::write(tree.join("routines.toml"), "[r1]\neffort = \"medium\"\n").unwrap();
    let r = super::replay::Replay {
        experiment: 1,
        routine: "r1".into(),
        dir: tree,
    };
    let inside = super::replay::REPLAY
        .scope(r, async { crate::models::role(&app, KEY).effort })
        .await;
    assert_eq!(inside.as_deref(), Some("medium"));
    assert_eq!(
        crate::models::role(&app, KEY).effort.as_deref(),
        Some("high")
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
