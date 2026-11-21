use super::{hook, origin_host_ok};
#[test]
fn same_origin_checks_host() {
    let ok = |host, origin| origin_host_ok(Some(host), origin, 7777);
    // CLI, hooks and the page
    assert!(ok("127.0.0.1", None) && ok("127.0.0.1:7777", None) && ok("localhost:7777", Some("http://localhost:7777")) && ok("127.0.0.1:7777", Some("http://127.0.0.1:7777")));
    // DNS rebinding (foreign Host, even with a matching-looking Origin) and a foreign Origin with a good Host
    assert!(!ok("evil.example:7777", None) && !ok("evil.example", Some("http://evil.example:7777")) && !ok("127.0.0.1.evil.example", None) && !ok("127.0.0.1:9999", None));
    assert!(!ok("127.0.0.1:7777", Some("http://evil.example")));
}
#[test]
fn content_kinds_split_page_from_attachment() {
    let k = crate::attach::content_kind;
    assert_eq!((k("a.PNG", false), k("a.jpg", true), k("x.bin", true)), ("image/png", "image/jpeg", "application/octet-stream"));
    assert_eq!((k("i.html", true), k("i.html", false), k("s.js", false), k("s.js", true)), ("text/html; charset=utf-8", "text/plain; charset=utf-8", "text/plain; charset=utf-8", "application/javascript; charset=utf-8"));
    assert_eq!((k("f.woff2", true), k("f.woff2", false), k("d.json", false), k("n.txt", true)), ("font/woff2", "application/octet-stream", "application/json", "application/octet-stream"));
}

/// An app at a fresh home with one project `p1` in a folder of its own (no tirith.toml: the default manifest, no project rules).
fn home(tag: &str) -> crate::app::Shared {
    let dir = std::env::temp_dir().join(format!("tirith-server-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("repo")).unwrap();
    std::fs::write(dir.join("config.toml"), format!("[[projects]]\nid = \"p1\"\nname = \"P One\"\nrepo = \"{}\"\n", dir.join("repo").display())).unwrap();
    let app = crate::app::App::open_at(dir);
    assert!(app.cfg().project("p1").is_some(), "config: {:?}", app.cfg().error);
    app
}

/// One `PreToolUse` hook call as the daemon gets it: with the caller's `X-Tirith-Role` and/or `X-Tirith-Lane` header (empty for none), from `cwd`, with the tool name and input the agent used.
async fn pre(app: &crate::app::Shared, role: &str, lane: &str, cwd: &str, tool: &str, input: serde_json::Value) -> serde_json::Value {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in [("x-tirith-role", role), ("x-tirith-lane", lane)] {
        if !v.is_empty() {
            headers.insert(k, v.parse().unwrap());
        }
    }
    let body = serde_json::json!({"cwd": cwd, "tool_name": tool, "session_id": "s1", "tool_input": input});
    hook(axum::extract::State(app.clone()), axum::extract::Path("PreToolUse".to_string()), headers, axum::body::Bytes::from(body.to_string())).await.0
}

/// One `PreToolUse` Bash hook call from the project's folder.
async fn pre_bash(app: &crate::app::Shared, role: &str, lane: &str, cmd: &str) -> serde_json::Value {
    let repo = app.cfg().project("p1").unwrap().repo.clone();
    pre(app, role, lane, &repo, "Bash", serde_json::json!({"command": cmd})).await
}

#[tokio::test]
async fn chat_roles_may_not_wait() {
    let app = home("wait");
    let deny = |v: &serde_json::Value| v["hookSpecificOutput"]["permissionDecision"] == "deny";
    let reason = |v: &serde_json::Value| v["hookSpecificOutput"]["permissionDecisionReason"].as_str().unwrap_or("").to_string();
    // A manager (a chat role, named by X-Tirith-Role and no lane) waiting inside its turn is refused, with the reason that says what to do instead.
    let long = pre_bash(&app, "manager", "", "sleep 30; tirith status").await;
    assert!(deny(&long), "a manager's sleep 30 is refused: {long}");
    assert_eq!(reason(&long), "no esperes dentro del turno: contesta y termina; Tirith te avisa cuando una lane entre, falle o pregunte");
    // Five seconds is already a wait; a loop's short sleep is one too (it adds up behind the owner's messages).
    assert!(deny(&pre_bash(&app, "manager", "", "sleep 5").await), "a manager's sleep 5 is refused");
    let looped = pre_bash(&app, "manager", "", "for i in 1 2 3 4 5; do tirith status; sleep 2; done").await;
    assert!(deny(&looped), "a manager's for loop with sleep is refused: {looped}");
    assert_eq!(reason(&looped), reason(&long), "the loop is refused for the same reason");
    // A bot in its own chat is a chat role too: the same wait is refused under its role.
    assert!(deny(&pre_bash(&app, "bot-abc", "", "sleep 30").await), "a bot's sleep 30 is refused");
    // A short sleep outside a loop is no wait for the owner: it passes (no denial).
    assert!(!deny(&pre_bash(&app, "manager", "", "sleep 2").await), "a manager's sleep 2 passes");
    assert!(!deny(&pre_bash(&app, "manager", "", "sleep 4; echo ok").await), "a manager's sleep 4 passes");
    // A lane does not change: the same waiting commands from a lane (X-Tirith-Lane, no role) pass, allowed by the rules as before.
    app.save_lane(&crate::app::Lane { id: "lane-wait".into(), project: "p1".into(), status: "running".into(), workspace: "/tmp/ws-lane-wait".into(), ..Default::default() });
    for cmd in ["sleep 30; tirith status", "for i in 1 2 3 4 5; do tirith status; sleep 2; done"] {
        let v = pre_bash(&app, "", "lane-wait", cmd).await;
        assert!(!deny(&v), "a lane's `{cmd}` passes: {v}");
        assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "allow", "a lane's allowed call needs no prompt: {v}");
    }
}

#[tokio::test]
async fn every_agent_obeys_the_rules() {
    let app = home("agents");
    let deny = |v: &serde_json::Value| v["hookSpecificOutput"]["permissionDecision"] == "deny";
    let reason = |v: &serde_json::Value| v["hookSpecificOutput"]["permissionDecisionReason"].as_str().unwrap_or("").to_string();
    let repo = app.cfg().project("p1").unwrap().repo.clone();
    let wait = "no esperes dentro del turno: contesta y termina; Tirith te avisa cuando una lane entre, falle o pregunte";
    let cmd = |c: &str| serde_json::json!({"command": c});
    // The muse names for a shell: `bash` (its command in `command`) and `monitor`. A manager waiting in either is refused, as with `clocksleep`, its tool for waiting outright.
    let looped = pre(&app, "manager", "", &repo, "bash", cmd("for i in 1 2 3 4 5; do tirith status; sleep 30; done")).await;
    assert!(deny(&looped), "a manager's bash for loop with sleep 30 is refused: {looped}");
    assert_eq!(reason(&looped), wait);
    let watched = pre(&app, "manager", "", &repo, "monitor", cmd("while true; do tirith status; sleep 30; done")).await;
    assert!(deny(&watched), "a manager's monitor while loop with sleep 30 is refused: {watched}");
    assert_eq!(reason(&watched), wait);
    let slept = pre(&app, "manager", "", &repo, "clocksleep", serde_json::json!({"seconds": 30})).await;
    assert!(deny(&slept), "a manager's clocksleep is refused: {slept}");
    assert_eq!(reason(&slept), wait);
    // A short sleep of the manager is no wait, whatever the agent calls the shell.
    assert!(!deny(&pre(&app, "manager", "", &repo, "bash", cmd("sleep 2")).await), "a manager's bash sleep 2 passes");
    // From a lane, the muse names obey the other rules too: no skipping the commit checks, no writing the plan or Tirith's own files, by `edit_file` (the file in `path`) or by `apply_patch` (its files on the patch's `*** … File:` lines, relative to its folder).
    let ws = "/tmp/ws-lane-agents";
    app.save_lane(&crate::app::Lane { id: "lane-agents".into(), project: "p1".into(), status: "running".into(), workspace: ws.into(), ..Default::default() });
    let skipped = pre(&app, "", "lane-agents", ws, "bash", cmd("git commit --no-verify -m x")).await;
    assert!(deny(&skipped), "a lane's bash git commit --no-verify is refused: {skipped}");
    assert!(reason(&skipped).contains("Commit checks are never skipped"), "the refusal names the commit checks: {skipped}");
    let config = app.home.join("config.toml").display().to_string();
    for (what, tool, input) in [
        ("an edit_file on the plan", "edit_file", serde_json::json!({"path": "docs/TODO.md", "find": "a", "replace": "b"})),
        ("a write_file on the plan", "write_file", serde_json::json!({"path": format!("{ws}/docs/TODO.md"), "content": "x"})),
        ("an edit_file on <home>/config.toml", "edit_file", serde_json::json!({"path": config.clone(), "find": "a", "replace": "b"})),
        ("an apply_patch on the plan (not its first file)", "apply_patch", serde_json::json!({"patch": "*** Begin Patch\n*** Update File: src/x.rs\n@@\n-a\n+b\n*** Update File: docs/TODO.md\n@@\n-a\n+b\n*** End Patch"})),
        ("an apply_patch on <home>/config.toml (not its first file)", "apply_patch", serde_json::json!({"patch": format!("*** Begin Patch\n*** Add File: src/y.rs\n+x\n*** Update File: {config}\n@@\n-a\n+b\n*** End Patch")})),
        ("an apply_patch moving a file onto <home>/config.toml", "apply_patch", serde_json::json!({"patch": format!("*** Begin Patch\n*** Update File: src/z.rs\n*** Move to: {config}\n@@\n-a\n+b\n*** End Patch")})),
    ] {
        let v = pre(&app, "", "lane-agents", ws, tool, input).await;
        assert!(deny(&v), "{what} from a lane is refused: {v}");
    }
    // The same waiting loop from a lane passes, allowed by the rules as before.
    let lane_loop = pre(&app, "", "lane-agents", ws, "bash", cmd("for i in 1 2 3 4 5; do tirith status; sleep 30; done")).await;
    assert!(!deny(&lane_loop), "a lane's bash for loop with sleep 30 passes: {lane_loop}");
    assert_eq!(lane_loop["hookSpecificOutput"]["permissionDecision"], "allow", "a lane's allowed call needs no prompt: {lane_loop}");
    // Without any role header, the hook's folder names the chat role: <home>/manager/tirith is the manager's, so its bash wait is refused just the same.
    let mgr_dir = app.home.join("manager/tirith").display().to_string();
    let by_folder = pre(&app, "", "", &mgr_dir, "bash", cmd("for i in 1 2 3; do tirith status; sleep 30; done")).await;
    assert!(deny(&by_folder), "a bash wait from <home>/manager/tirith with no role header is refused: {by_folder}");
    assert_eq!(reason(&by_folder), wait);
    // The sandbox audit annotates a muse shell's and editor's accesses too, and the events keep the names the agent used, not the translated ones.
    let _ = pre(&app, "manager", "", &repo, "bash", cmd("cat ~/.ssh/id_rsa")).await;
    let _ = pre(&app, "", "lane-agents", ws, "edit_file", serde_json::json!({"path": "/home/owner/.ssh/id_rsa", "find": "a", "replace": "b"})).await;
    assert_eq!(app.query("SELECT COUNT(*) FROM events WHERE kind = 'access' AND tool = 'credential'", [], |r| r.get::<_, i64>(0)), [2], "the manager's bash credential read and the lane's edit_file on a credential are annotated");
    let blocked: Vec<String> = app.query("SELECT tool FROM events WHERE kind = 'blocked' ORDER BY id", [], |r| r.get::<_, String>(0));
    for tool in ["bash", "monitor", "clocksleep", "edit_file", "write_file", "apply_patch"] {
        assert!(blocked.iter().any(|t| t == tool), "a blocked {tool} call is recorded under its own name: {blocked:?}");
    }
    assert!(!blocked.iter().any(|t| t == "Bash" || t == "Edit" || t == "Write"), "no event carries a translated name: {blocked:?}");
}
