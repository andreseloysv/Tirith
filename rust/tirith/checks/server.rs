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

/// One `PreToolUse` Bash hook call as the daemon gets it: with the caller's `X-Tirith-Role` and/or `X-Tirith-Lane` header (empty for none), from the project's folder.
async fn pre_bash(app: &crate::app::Shared, role: &str, lane: &str, cmd: &str) -> serde_json::Value {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in [("x-tirith-role", role), ("x-tirith-lane", lane)] {
        if !v.is_empty() {
            headers.insert(k, v.parse().unwrap());
        }
    }
    let repo = app.cfg().project("p1").unwrap().repo.clone();
    let body = serde_json::json!({"cwd": repo, "tool_name": "Bash", "session_id": "s1", "tool_input": {"command": cmd}});
    hook(axum::extract::State(app.clone()), axum::extract::Path("PreToolUse".to_string()), headers, axum::body::Bytes::from(body.to_string())).await.0
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
