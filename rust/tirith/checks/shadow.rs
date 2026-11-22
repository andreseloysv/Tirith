use rusqlite::params;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

const KEY: &str = "bot-mail01";
const TASK: &str = "Routine «gmail-safari» is due now. Check the inbox and list the new offers";

/// A fake stream-json agent. As the bot (a shadow B run), it keeps its folder and what it was handed, asks Tirith's hook (over HTTP, as an agent's hooks do) for a live read, a write in its folder, a `tirith safari read`, a browser look, and then for what reaches outside — `tirith say`, `ask-owner`, `safari open`, a POST, a click on «Send» — keeping each answer in `<home>/heard-<conversation>.txt`, and answers. As anyone else (the judge) it says «tie».
const AGENT: &str = r#"#!/bin/sh
read -r line
if [ "$TIRITH_ROLE" != "KEY" ]; then
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"tie — both list the same offers"}]}}'
  echo '{"type":"result","subtype":"success","is_error":false,"usage":{"input_tokens":3,"output_tokens":2}}'
  exit 0
fi
here=$(pwd)
printf '%s\n' "$here" > "$TIRITH_HOME/pwd-$TIRITH_PROJECT.txt"
printf '%s\n' "$line" > "$TIRITH_HOME/told-$TIRITH_PROJECT.txt"
out="$TIRITH_HOME/heard-$TIRITH_PROJECT.txt"
hook() {
  curl -s --noproxy '*' -X POST "http://127.0.0.1:PORT/hook/PreToolUse" -H 'Host: 127.0.0.1' -H "X-Tirith-Token: $TIRITH_TOKEN" -H 'Content-Type: application/json' --data-binary "{\"cwd\":\"$here\",\"session_id\":\"s\",\"tool_name\":\"$1\",\"tool_input\":$2}" >> "$out"
  echo >> "$out"
}
hook WebFetch '{"url":"https://mail.example/inbox"}'
hook Write "{\"file_path\":\"$here/notes.md\",\"content\":\"seen\"}"
hook Bash '{"command":"tirith safari read https://mail.example/inbox"}'
hook mcp__tirith_browser__browser_navigate '{"url":"https://mail.example/inbox"}'
hook Bash '{"command":"tirith say --project KEY hola"}'
hook Bash '{"command":"tirith ask-owner \"¿sigo?\""}'
hook Bash '{"command":"tirith safari open https://mail.example/compose"}'
hook Bash '{"command":"curl -s -X POST https://mail.example/send -d to=ana"}'
hook mcp__tirith_browser__browser_click '{"element":"Send button","ref":"e12"}'
sleep SLOW
echo '{"type":"assistant","message":{"content":[{"type":"text","text":"3 offers listed (B)"}]}}'
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

/// A Tirith home with the bot «Correo» (routine `r1`) on the fake agent, a branch `v3` with one commit on its main, a server for its hooks, and a shadow experiment started: the app, its home and the experiment.
async fn shadowed(name: &str, slow: u32) -> (crate::app::Shared, PathBuf, i64) {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let home = std::env::temp_dir().join(format!("tirith-shadow-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let agent = home.join("fake-agent.sh");
    std::fs::write(
        &agent,
        AGENT
            .replace("PORT", &port.to_string())
            .replace("KEY", KEY)
            .replace("SLOW", &slow.to_string()),
    )
    .unwrap();
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
    let dir = home.join("manager").join(KEY);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("memory.md"), "# Memory\n- open every mail\n").unwrap();
    crate::bots::versioned(&app, KEY, "library");
    let draft = home.join("lab").join(KEY).join("draft");
    let d = draft.display().to_string();
    git(&dir, &["worktree", "add", "-q", "-b", "v3", &d]);
    std::fs::write(
        draft.join("memory.md"),
        "# Memory\n- search with a filter\n",
    )
    .unwrap();
    git(&draft, &["commit", "-q", "-am", "v3: search with a filter"]);
    git(&dir, &["worktree", "remove", &d]);
    let app2 = app.clone();
    tokio::spawn(
        async move { drop(axum::serve(listener, crate::server::router(app2, port)).await) },
    );
    let b = json!({"bot": KEY, "branch": "v3", "routine": "r1", "mode": "shadow", "hypothesis": "a filter: −40 % tokens"});
    let v = super::start(&app, &b).unwrap();
    assert_eq!(v["mode"], "shadow", "{v}");
    (app, home, v["id"].as_i64().unwrap())
}

/// Waits until `f` holds (up to a minute).
async fn until(what: &str, f: impl Fn() -> bool) {
    for _ in 0..600 {
        if f() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("never: {what}");
}

/// A's real run of the routine, as the bot's session ends it: its answer in the chat, its row kept.
fn a_run(app: &crate::app::Shared, answer: &str) -> i64 {
    app.chat(KEY, answer, KEY);
    let sql = "INSERT INTO turns (conv, role, \"trigger\", routine, started_at, ms, status, tokens_in, tokens_out, answer) VALUES (?1, ?1, 'schedule', 'r1', '2026-11-21T08:00:00Z', 900, 'ok', 100, 20, ?2)";
    let id = app.insert(sql, params![KEY, answer]);
    crate::lab::finished(app, id, KEY, "r1");
    id
}

fn b_turns(app: &crate::app::Shared, e: i64) -> Vec<(i64, String, String, String)> {
    let sql = "SELECT id, conv, variant, \"trigger\" FROM turns WHERE experiment = ?1 AND variant = 'B' ORDER BY id";
    app.query(sql, [e], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn shadow_runs_beside_each_real_run() {
    let (app, home, e) = shadowed("beside", 1).await;
    let (live, tree) = (
        home.join("manager").join(KEY),
        home.join("lab").join(KEY).join(e.to_string()),
    );
    let mut pairs = vec![];
    for n in 1..=2 {
        crate::lab::fired(&app, KEY, "r1", TASK);
        let conv = format!("shadow-{e}-{n}");
        let told = home.join(format!("told-{conv}.txt"));
        until("B starts beside A", || told.exists()).await;
        assert_eq!(
            crate::bots::folder(&app, KEY),
            live,
            "A runs in the bot's own folder meanwhile"
        );
        let a = a_run(&app, &format!("2 offers (A{n})"));
        until("B ends", || b_turns(&app, e).len() == n).await;
        let b = b_turns(&app, e)[n - 1].clone();
        assert_eq!(
            (b.1.as_str(), b.3.as_str()),
            (conv.as_str(), "shadow"),
            "{b:?}"
        );
        let pwd = std::fs::read_to_string(home.join(format!("pwd-{conv}.txt"))).unwrap();
        assert_eq!(
            Path::new(pwd.trim()).canonicalize().unwrap(),
            tree.canonicalize().unwrap(),
            "B in the branch's worktree"
        );
        let told = std::fs::read_to_string(&told).unwrap();
        assert!(
            told.contains("Check the inbox") && told.contains("search with a filter"),
            "the same task, the branch's memory: {told}"
        );
        let variant: Vec<String> =
            app.query("SELECT variant FROM turns WHERE id = ?1", [a], |r| r.get(0));
        assert_eq!(variant, vec!["A".to_string()], "the real run is A");
        pairs.push((a, b.0));
    }
    let sql = "SELECT a_turn, b_turn FROM lab_comparisons WHERE experiment = ?1 ORDER BY id";
    let compared: Vec<(i64, i64)> = app.query(sql, [e], |r| Ok((r.get(0)?, r.get(1)?)));
    assert_eq!(compared, pairs, "each B compared with the A it ran beside");
    let row = &super::list(&app, KEY)[0];
    assert_eq!(
        (row["mode"].as_str(), row["status"].as_str()),
        (Some("shadow"), Some("running")),
        "2 of 8: not due yet"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "a test of a sandbox: not run, by the owner's order"]
async fn shadow_never_reaches_outside() {
    let (app, home, e) = shadowed("outside", 0).await;
    crate::lab::fired(&app, KEY, "r1", TASK);
    let conv = format!("shadow-{e}-1");
    until("B ends", || b_turns(&app, e).len() == 1).await;
    a_run(&app, "2 offers (A)");
    let heard = std::fs::read_to_string(home.join(format!("heard-{conv}.txt"))).unwrap();
    let said: Vec<Value> = heard
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|x| panic!("{x}: {heard}")))
        .collect();
    let denied = |i: usize| said[i]["hookSpecificOutput"]["permissionDecision"] == "deny";
    let why = |i: usize| {
        said[i]["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };
    for (i, what) in [
        "a live read",
        "a write in its worktree",
        "tirith safari read",
        "a browser look",
    ]
    .iter()
    .enumerate()
    {
        assert!(!denied(i), "{what} runs as always: {}", said[i]);
    }
    for (i, what) in [
        "tirith say",
        "tirith ask-owner",
        "tirith safari open",
        "a POST",
        "a click on Send",
    ]
    .iter()
    .enumerate()
    {
        assert!(
            denied(i + 4) && why(i + 4).contains("not available in a shadow run"),
            "{what} is refused: {}",
            said[i + 4]
        );
    }
    let chat: Vec<(String, String)> = app.query("SELECT project, text FROM chat", [], |r| {
        Ok((r.get(0)?, r.get(1)?))
    });
    assert!(
        chat.iter().any(|(_, t)| t == "2 offers (A)"),
        "A reaches the chat: {chat:?}"
    );
    assert!(
        chat.iter().all(|(p, t)| p == KEY && !t.contains("(B)")),
        "nothing of B: {chat:?}"
    );
    let cards: i64 = app
        .query("SELECT COUNT(*) FROM owner_items", [], |r| r.get(0))
        .remove(0);
    assert_eq!(cards, 0, "no card");
    assert!(
        crate::manager::say(&app, &conv, "hola", "").is_err(),
        "B's conversation is no one's"
    );
    // Outside a shadow run the same call is no business of the lab's.
    let say = json!({"command": "tirith say hola"});
    assert_eq!(
        crate::lab::refused(
            &home.join("manager").join(KEY).display().to_string(),
            "Bash",
            "Bash",
            &say
        ),
        None
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_shadow_runs_tool_must_say_it_reads() {
    let acts = |tool: &str| super::outward_browser::acts(tool, &json!({}));
    for tool in [
        "mcp__mail__search_threads",
        "mcp__crm__getUser",
        "mcp__apps__gmail_search_emails",
    ] {
        assert!(!acts(tool), "{tool} reads");
    }
    for tool in [
        "mcp__mail__sendEmail",
        "mcp__crm__approve_invoice",
        "mcp__x__go",
        "mcp__crm__get_and_delete",
    ] {
        assert!(acts(tool), "{tool} may act");
    }
    let dir = Path::new("/home/tirith/w");
    let outward = |cmd: &str| super::outward::outward("Bash", &json!({"command": cmd}), dir);
    assert!(
        !outward("ls /home/tirith -la"),
        "a server's home is no command"
    );
    assert!(
        !outward("tirith turn checks 1/1 \"marker file\""),
        "a run's own validation"
    );
    assert!(outward("~/.local/bin/tirith say hola"));
    assert!(outward("cd /tmp && tirith lane start x 1 y"));
}

#[test]
fn a_shadow_run_cannot_act_by_a_path_or_a_word_that_reads() {
    let dir = Path::new("/home/tirith/w");
    let outward = |cmd: &str| super::outward::outward("Bash", &json!({"command": cmd}), dir);
    for cmd in [
        "/Applications/Tirith.app/Contents/MacOS/tirith lane start p 1 x",
        "\"/opt/x/tirith\" say hola",
        "cd /tmp\n./tirith room say r hola",
    ] {
        assert!(outward(cmd), "{cmd}");
    }
    assert!(!outward("ls /home/tirith -la") && !outward("cat /home/tirith/notes.md"));
    for tool in [
        "mcp__mail__mark_as_read",
        "mcp__tracker__resolve_issue",
        "mcp__x__check_in",
        "mcp__cal__set_status",
    ] {
        assert!(super::outward_browser::acts(tool, &json!({})), "{tool}");
    }
    assert!(!super::outward_browser::acts(
        "mcp__mail__read_thread",
        &json!({})
    ));
}
