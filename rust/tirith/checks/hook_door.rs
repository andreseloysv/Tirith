// The hook-only door: the rules refuse as always, and nothing else of OpenBot is read or changed through it.
use super::hook::rules_alone;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue};
use serde_json::{json, Value};

#[tokio::test]
#[ignore = "a test of a sandbox: not run, by the owner's order"]
async fn the_rules_door_gives_the_rules_and_nothing_else() {
    let home = std::env::temp_dir().join(format!("tirith-hook-door-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("config.toml"), "").unwrap();
    let app = crate::app::App::open_at(home.clone());
    let lane = crate::app::Lane {
        id: "someones".into(),
        status: "running".into(),
        session: "their-session".into(),
        note: "their note".into(),
        ..Default::default()
    };
    app.lanes.lock().unwrap().insert(lane.id.clone(), lane);
    let ask = |event: &'static str, body: Value| {
        let mut headers = HeaderMap::new();
        let token = crate::server::testing::token(&app, "someones");
        headers.insert("x-tirith-token", HeaderValue::from_str(&token).unwrap());
        let (app, body) = (app.clone(), Bytes::from(body.to_string()));
        async move {
            rules_alone(State(app), Path(event.to_string()), headers, body)
                .await
                .0
        }
    };
    let forged = json!({"session_id": "forged", "last_assistant_message": "forged?", "tool_name": "Bash", "tool_input": {"command": "ls"}});
    for event in [
        "PostToolUse",
        "Stop",
        "UserPromptSubmit",
        "SessionStart",
        "PreToolUse",
    ] {
        assert_eq!(ask(event, forged.clone()).await, json!({}), "{event}");
    }
    let kept = app.lanes.lock().unwrap()["someones"].clone();
    assert_eq!(
        (kept.session.as_str(), kept.note.as_str()),
        ("their-session", "their note"),
        "no lane is another's to write"
    );
    let refused = json!({"cwd": home.display().to_string(), "tool_name": "Bash", "tool_input": {"command": "echo x > sub/.git/hooks/pre-commit"}});
    assert_eq!(
        ask("PreToolUse", refused).await["hookSpecificOutput"]["permissionDecision"],
        "deny",
        "the rules still refuse"
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = super::hook_door::routes(app.clone());
    tokio::spawn(async move { axum::serve(listener, router).await });
    for path in ["/api/state", "/api/stream", "/api/rooms", "/"] {
        let got = tokio::task::spawn_blocking(move || {
            use std::io::{Read, Write};
            let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            let ask =
                format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
            s.write_all(ask.as_bytes()).unwrap();
            let mut answer = String::new();
            s.read_to_string(&mut answer).unwrap();
            answer
        })
        .await
        .unwrap();
        assert!(got.starts_with("HTTP/1.1 403"), "{path}: {got}");
    }
    let _ = std::fs::remove_dir_all(&home);
}
