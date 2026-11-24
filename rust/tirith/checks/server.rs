use super::{hook, origin_host_ok, web_file};
#[test]
fn same_origin_checks_host() {
    let ok = |host, origin| origin_host_ok(Some(host), origin, 7777);
    // CLI, hooks and the page
    assert!(
        ok("127.0.0.1", None)
            && ok("127.0.0.1:7777", None)
            && ok("localhost:7777", Some("http://localhost:7777"))
            && ok("127.0.0.1:7777", Some("http://127.0.0.1:7777"))
    );
    // DNS rebinding (foreign Host, even with a matching-looking Origin) and a foreign Origin with a good Host
    assert!(
        !ok("evil.example:7777", None)
            && !ok("evil.example", Some("http://evil.example:7777"))
            && !ok("127.0.0.1.evil.example", None)
            && !ok("127.0.0.1:9999", None)
    );
    assert!(!ok("127.0.0.1:7777", Some("http://evil.example")));
}
#[test]
fn content_kinds_split_page_from_attachment() {
    let k = crate::attach::content_kind;
    assert_eq!(
        (k("a.PNG", false), k("a.jpg", true), k("x.bin", true)),
        ("image/png", "image/jpeg", "application/octet-stream")
    );
    assert_eq!(
        (
            k("i.html", true),
            k("i.html", false),
            k("s.js", false),
            k("s.js", true)
        ),
        (
            "text/html; charset=utf-8",
            "text/plain; charset=utf-8",
            "text/plain; charset=utf-8",
            "application/javascript; charset=utf-8"
        )
    );
    assert_eq!(
        (
            k("f.woff2", true),
            k("f.woff2", false),
            k("d.json", false),
            k("n.txt", true)
        ),
        (
            "font/woff2",
            "application/octet-stream",
            "application/json",
            "application/octet-stream"
        )
    );
}

/// A face in a list or a head is an avatar in one fixed box: the served CSS sizes it and keeps its glow and its
/// float inside that box (`overflow: hidden` on the face and on its SVG layers), so no face can grow its row,
/// mount its neighbours or cover the name and text beside it — as the owner's 2026-11-21 phone captures showed.
/// The motion itself stays, now inside the vendored face element: the served face script still carries the
/// size as `--fs` and renders that element, which fills its box. Read through `web_file`, the handler that
/// serves the page, so what is pinned is what is served.
#[tokio::test]
async fn face_avatars_stay_in_their_box() {
    let app = home("faces");
    let served = |path: &'static str| {
        let app = app.clone();
        async move {
            let res = web_file(axum::extract::State(app), path.parse().unwrap()).await;
            assert_eq!(res.status(), axum::http::StatusCode::OK, "{path} is served");
            let body = axum::body::to_bytes(res.into_body(), 1 << 20)
                .await
                .unwrap();
            String::from_utf8(body.to_vec())
                .unwrap()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        }
    };
    let bots = served("/css/bots.css").await;
    let chat = served("/css/chat.css").await;
    let face = served("/js/views/bots/face.js").await;
    // The box itself: a face is exactly its `--fs` square, never the size its glow would paint.
    assert!(
        bots.contains(".bot.face { position: relative; width: var(--fs); height: var(--fs); }"),
        "a face's box is its --fs square: {bots:.200}"
    );
    assert!(
        face.contains("--fs:${size}px"),
        "the face script carries its size as --fs"
    );
    // In a list, a picker and a head the paint stays in the box: the face and every SVG layer in it clip.
    assert!(bots.contains(".nbot .bot.face, .mc-face .bot.face, .rface .bot.face, .bshapes .bot.face { overflow: hidden; }"), "list, head and picker faces clip to their box");
    assert!(bots.contains(".nbot .bot.face .fsvg, .mc-face .bot.face .fsvg, .rface .bot.face .fsvg, .bshapes .bot.face .fsvg { overflow: hidden; }"), "their SVG layers clip too, glow included");
    // And the box is fixed by the stylesheet, whatever size a caller asks the script for.
    assert!(
        bots.contains(".nbot .bot.face { width: 64px; height: 64px; }"),
        "the sidebar list's avatars are 64px"
    );
    assert!(
        bots.contains(".rface .bot.face { width: 40px; height: 40px; }"),
        "a room card's avatar is 40px"
    );
    assert!(
        bots.contains(".bshapes .bot.face { width: 58px; height: 58px; }"),
        "the shape picker's faces are 58px"
    );
    assert!(
        bots.contains(".mc-wface .bot.face { width: 56px; height: 56px; }"),
        "the face of a bot at work in a chat is 56px"
    );
    assert!(
        chat.contains(".mc-head .mc-face .bot { width: 26px; height: 26px; }"),
        "a chat head's avatar is 26px on the desktop"
    );
    assert!(
        chat.contains(".mc-head .mc-face .bot { width: 64px; height: 64px; }"),
        "and the phone head's big face is a fixed 64px"
    );
    // The movement is not the price: the animated element fills the box and moves inside it.
    assert!(
        bots.contains(".bot.face > bot-face { display: block; width: 100%; height: 100%; }"),
        "the face element fills its box"
    );
    assert!(
        face.contains("<bot-face") && face.contains("vendor/botface/bot-face.js"),
        "the face script renders the vendored animated face"
    );
}

/// An app at a fresh home with one project `p1` in a folder of its own (no tirith.toml: the default manifest, no project rules).
fn home(tag: &str) -> crate::app::Shared {
    let dir = std::env::temp_dir().join(format!("tirith-server-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("repo")).unwrap();
    std::fs::write(
        dir.join("config.toml"),
        format!(
            "[[projects]]\nid = \"p1\"\nname = \"P One\"\nrepo = \"{}\"\n",
            dir.join("repo").display()
        ),
    )
    .unwrap();
    let app = crate::app::App::open_at(dir);
    assert!(
        app.cfg().project("p1").is_some(),
        "config: {:?}",
        app.cfg().error
    );
    app
}

/// One `PreToolUse` hook call as the daemon gets it: with the token of the caller's role or lane (none of both: no token), from `cwd`, with the tool name and input the agent used.
async fn pre(
    app: &crate::app::Shared,
    role: &str,
    lane: &str,
    cwd: &str,
    tool: &str,
    input: serde_json::Value,
) -> serde_json::Value {
    let mut headers = axum::http::HeaderMap::new();
    let caller = if lane.is_empty() {
        (!role.is_empty()).then(|| format!("{role}@"))
    } else {
        Some(lane.to_string())
    };
    if let Some(caller) = caller {
        let token = super::testing::token(app, &caller);
        headers.insert(super::testing::HEADER, token.parse().unwrap());
    }
    let who = super::identify::agent(app, &headers).map_or(super::Who::Nobody, super::Who::Agent);
    let body =
        serde_json::json!({"cwd": cwd, "tool_name": tool, "session_id": "s1", "tool_input": input});
    hook(
        axum::extract::State(app.clone()),
        axum::extract::Path("PreToolUse".to_string()),
        who,
        axum::body::Bytes::from(body.to_string()),
    )
    .await
    .0
}

/// One `PreToolUse` Bash hook call from the project's folder.
async fn pre_bash(
    app: &crate::app::Shared,
    role: &str,
    lane: &str,
    cmd: &str,
) -> serde_json::Value {
    let repo = app.cfg().project("p1").unwrap().repo.clone();
    pre(
        app,
        role,
        lane,
        &repo,
        "Bash",
        serde_json::json!({"command": cmd}),
    )
    .await
}

#[tokio::test]
async fn chat_roles_may_not_wait() {
    let app = home("wait");
    let deny = |v: &serde_json::Value| v["hookSpecificOutput"]["permissionDecision"] == "deny";
    let reason = |v: &serde_json::Value| {
        v["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };
    // A manager (a chat role, named by its token and no lane) waiting inside its turn is refused, with the reason that says what to do instead.
    let long = pre_bash(&app, "manager", "", "sleep 30; tirith status").await;
    assert!(deny(&long), "a manager's sleep 30 is refused: {long}");
    assert_eq!(reason(&long), "no esperes dentro del turno: contesta y termina; OpenBot te avisa cuando una lane entre, falle o pregunte");
    // Five seconds is already a wait; a loop's short sleep is one too (it adds up behind the owner's messages).
    assert!(
        deny(&pre_bash(&app, "manager", "", "sleep 5").await),
        "a manager's sleep 5 is refused"
    );
    let looped = pre_bash(
        &app,
        "manager",
        "",
        "for i in 1 2 3 4 5; do tirith status; sleep 2; done",
    )
    .await;
    assert!(
        deny(&looped),
        "a manager's for loop with sleep is refused: {looped}"
    );
    assert_eq!(
        reason(&looped),
        reason(&long),
        "the loop is refused for the same reason"
    );
    // Following a log or watching something until it happens is waiting too: today's `tail -F` of a lane's log, each watch command in command position (also after `;`, `&&` or `do`), the shell's `wait`, `gh run watch` and any `--watch`.
    let watching = [
        "tail -n 0 -F /tmp/lane.log | grep -E --line-buffered run.lifecycle.completed",
        "tail -f /tmp/x.log",
        "tail -fn0 /tmp/x.log",
        "tail -n0F /tmp/x.log",
        "tail --follow /tmp/x.log",
        "watch -n 5 git status",
        "fswatch .",
        "inotifywait -m .",
        "gh run watch",
        "npm test -- --watch",
        "wait",
        "echo start; tail -f /tmp/x.log",
        "true && watch git status",
        "false || wait",
        "echo hi | tail -f /tmp/x.log",
        "(tail -f /tmp/x.log)",
        "echo $(wait)",
        "for i in 1; do tail -f /tmp/x.log; done",
        "if true; then watch git status; fi",
    ];
    for cmd in watching {
        let v = pre_bash(&app, "manager", "", cmd).await;
        assert!(deny(&v), "a manager's `{cmd}` is refused: {v}");
        assert_eq!(
            reason(&v),
            reason(&long),
            "a manager's `{cmd}` is refused for the same reason"
        );
    }
    // A bot in its own chat is a chat role too: the same wait is refused under its role.
    assert!(
        deny(&pre_bash(&app, "bot-abc", "", "sleep 30").await),
        "a bot's sleep 30 is refused"
    );
    assert!(
        deny(&pre_bash(&app, "bot-abc", "", "tail -f /tmp/x.log").await),
        "a bot's tail -f is refused"
    );
    // A short sleep outside a loop is no wait for the owner: it passes (no denial).
    assert!(
        !deny(&pre_bash(&app, "manager", "", "sleep 2").await),
        "a manager's sleep 2 passes"
    );
    assert!(
        !deny(&pre_bash(&app, "manager", "", "sleep 4; echo ok").await),
        "a manager's sleep 4 passes"
    );
    // Reading a log's tail, git's own `--follow`, the word as a mere argument and the quoted text of a message are no wait: they pass.
    for cmd in [
        "tail -n 50 /tmp/x.log",
        "git log --follow src/lib.rs",
        "echo \"watch this\"",
        "tirith say \"please wait; watch this\"",
        "echo watch",
    ] {
        assert!(
            !deny(&pre_bash(&app, "manager", "", cmd).await),
            "a manager's `{cmd}` passes"
        );
    }
    // A lane does not change: the same waiting commands from a lane (by a lane's token, no role) pass, allowed by the rules as before.
    app.save_lane(&crate::app::Lane {
        id: "lane-wait".into(),
        project: "p1".into(),
        status: "running".into(),
        workspace: "/tmp/ws-lane-wait".into(),
        ..Default::default()
    });
    for cmd in [
        "sleep 30; tirith status",
        "for i in 1 2 3 4 5; do tirith status; sleep 2; done",
    ]
    .into_iter()
    .chain(watching)
    {
        let v = pre_bash(&app, "", "lane-wait", cmd).await;
        assert!(!deny(&v), "a lane's `{cmd}` passes: {v}");
        assert_eq!(
            v["hookSpecificOutput"]["permissionDecision"], "allow",
            "a lane's allowed call needs no prompt: {v}"
        );
    }
}

#[tokio::test]
async fn every_agent_obeys_the_rules() {
    let app = home("agents");
    let deny = |v: &serde_json::Value| v["hookSpecificOutput"]["permissionDecision"] == "deny";
    let reason = |v: &serde_json::Value| {
        v["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };
    let repo = app.cfg().project("p1").unwrap().repo.clone();
    let wait = "no esperes dentro del turno: contesta y termina; OpenBot te avisa cuando una lane entre, falle o pregunte";
    let cmd = |c: &str| serde_json::json!({"command": c});
    // The muse names for a shell: `bash` (its command in `command`) and `monitor`. A manager waiting in either is refused, as with `clocksleep`, its tool for waiting outright.
    let looped = pre(
        &app,
        "manager",
        "",
        &repo,
        "bash",
        cmd("for i in 1 2 3 4 5; do tirith status; sleep 30; done"),
    )
    .await;
    assert!(
        deny(&looped),
        "a manager's bash for loop with sleep 30 is refused: {looped}"
    );
    assert_eq!(reason(&looped), wait);
    let watched = pre(
        &app,
        "manager",
        "",
        &repo,
        "monitor",
        cmd("while true; do tirith status; sleep 30; done"),
    )
    .await;
    assert!(
        deny(&watched),
        "a manager's monitor while loop with sleep 30 is refused: {watched}"
    );
    assert_eq!(reason(&watched), wait);
    let followed = pre(
        &app,
        "manager",
        "",
        &repo,
        "monitor",
        cmd("tail -n 0 -F /tmp/lane.log | grep -E --line-buffered run.lifecycle.completed"),
    )
    .await;
    assert!(
        deny(&followed),
        "a manager's monitor tail -F of a lane's log is refused: {followed}"
    );
    assert_eq!(reason(&followed), wait);
    let slept = pre(
        &app,
        "manager",
        "",
        &repo,
        "clocksleep",
        serde_json::json!({"seconds": 30}),
    )
    .await;
    assert!(deny(&slept), "a manager's clocksleep is refused: {slept}");
    assert_eq!(reason(&slept), wait);
    // A short sleep of the manager is no wait, whatever the agent calls the shell.
    assert!(
        !deny(&pre(&app, "manager", "", &repo, "bash", cmd("sleep 2")).await),
        "a manager's bash sleep 2 passes"
    );
    // From a lane, the muse names obey the other rules too: no skipping the commit checks, no writing the plan or OpenBot's own files, by `edit_file` (the file in `path`) or by `apply_patch` (its files on the patch's `*** … File:` lines, relative to its folder).
    let ws = "/tmp/ws-lane-agents";
    app.save_lane(&crate::app::Lane {
        id: "lane-agents".into(),
        project: "p1".into(),
        status: "running".into(),
        workspace: ws.into(),
        ..Default::default()
    });
    let skipped = pre(
        &app,
        "",
        "lane-agents",
        ws,
        "bash",
        cmd("git commit --no-verify -m x"),
    )
    .await;
    assert!(
        deny(&skipped),
        "a lane's bash git commit --no-verify is refused: {skipped}"
    );
    assert!(
        reason(&skipped).contains("Commit checks are never skipped"),
        "the refusal names the commit checks: {skipped}"
    );
    let config = app.home.join("config.toml").display().to_string();
    for (what, tool, input) in [
        (
            "an edit_file on the plan",
            "edit_file",
            serde_json::json!({"path": "docs/TODO.md", "find": "a", "replace": "b"}),
        ),
        (
            "a write_file on the plan",
            "write_file",
            serde_json::json!({"path": format!("{ws}/docs/TODO.md"), "content": "x"}),
        ),
        (
            "an edit_file on <home>/config.toml",
            "edit_file",
            serde_json::json!({"path": config.clone(), "find": "a", "replace": "b"}),
        ),
        (
            "an apply_patch on the plan (not its first file)",
            "apply_patch",
            serde_json::json!({"patch": "*** Begin Patch\n*** Update File: src/x.rs\n@@\n-a\n+b\n*** Update File: docs/TODO.md\n@@\n-a\n+b\n*** End Patch"}),
        ),
        (
            "an apply_patch on <home>/config.toml (not its first file)",
            "apply_patch",
            serde_json::json!({"patch": format!("*** Begin Patch\n*** Add File: src/y.rs\n+x\n*** Update File: {config}\n@@\n-a\n+b\n*** End Patch")}),
        ),
        (
            "an apply_patch moving a file onto <home>/config.toml",
            "apply_patch",
            serde_json::json!({"patch": format!("*** Begin Patch\n*** Update File: src/z.rs\n*** Move to: {config}\n@@\n-a\n+b\n*** End Patch")}),
        ),
    ] {
        let v = pre(&app, "", "lane-agents", ws, tool, input).await;
        assert!(deny(&v), "{what} from a lane is refused: {v}");
    }
    // The same waiting loop from a lane passes, allowed by the rules as before.
    let lane_loop = pre(
        &app,
        "",
        "lane-agents",
        ws,
        "bash",
        cmd("for i in 1 2 3 4 5; do tirith status; sleep 30; done"),
    )
    .await;
    assert!(
        !deny(&lane_loop),
        "a lane's bash for loop with sleep 30 passes: {lane_loop}"
    );
    assert_eq!(
        lane_loop["hookSpecificOutput"]["permissionDecision"], "allow",
        "a lane's allowed call needs no prompt: {lane_loop}"
    );
    // Without any role header, the hook's folder names the chat role: <home>/manager/tirith is the manager's, so its bash wait is refused just the same.
    let mgr_dir = app.home.join("manager/tirith").display().to_string();
    let by_folder = pre(
        &app,
        "",
        "",
        &mgr_dir,
        "bash",
        cmd("for i in 1 2 3; do tirith status; sleep 30; done"),
    )
    .await;
    assert!(
        deny(&by_folder),
        "a bash wait from <home>/manager/tirith with no role header is refused: {by_folder}"
    );
    assert_eq!(reason(&by_folder), wait);
    // The sandbox audit annotates a muse shell's and editor's accesses too, and the events keep the names the agent used, not the translated ones.
    let _ = pre(&app, "manager", "", &repo, "bash", cmd("cat ~/.ssh/id_rsa")).await;
    let _ = pre(
        &app,
        "",
        "lane-agents",
        ws,
        "edit_file",
        serde_json::json!({"path": "/home/owner/.ssh/id_rsa", "find": "a", "replace": "b"}),
    )
    .await;
    assert_eq!(
        app.query(
            "SELECT COUNT(*) FROM events WHERE kind = 'access' AND tool = 'credential'",
            [],
            |r| r.get::<_, i64>(0)
        ),
        [2],
        "the manager's bash credential read and the lane's edit_file on a credential are annotated"
    );
    let blocked: Vec<String> = app.query(
        "SELECT tool FROM events WHERE kind = 'blocked' ORDER BY id",
        [],
        |r| r.get::<_, String>(0),
    );
    for tool in [
        "bash",
        "monitor",
        "clocksleep",
        "edit_file",
        "write_file",
        "apply_patch",
    ] {
        assert!(
            blocked.iter().any(|t| t == tool),
            "a blocked {tool} call is recorded under its own name: {blocked:?}"
        );
    }
    assert!(
        !blocked
            .iter()
            .any(|t| t == "Bash" || t == "Edit" || t == "Write"),
        "no event carries a translated name: {blocked:?}"
    );
}
