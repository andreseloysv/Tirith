#[test]
fn restart_waits_for_a_running_turn() {
    super::talk("restart-test", |t| (t.busy, t.pid) = (true, 0));
    super::restart();
    assert!(super::talk("restart-test", |t| t.stale));
}

#[test]
fn turn_read_only_allows_reads() {
    for cmd in [
        "tirith status",
        "tirith plan show tirith",
        "tirith lane log some-lane",
        "tirith timeout get",
        "tirith merge status",
    ] {
        let input = serde_json::json!({"command": cmd});
        assert_eq!(
            super::turn_permission(true, "Bash", &input)["behavior"],
            "allow",
            "{cmd}"
        );
    }
    assert_eq!(
        super::turn_permission(true, "Read", &serde_json::json!({"file_path": "/x"}))["behavior"],
        "allow"
    );
    ["Write", "Edit", "MultiEdit", "NotebookEdit"]
        .into_iter()
        .for_each(|t| {
            assert_eq!(
                super::turn_permission(true, t, &serde_json::json!({}))["behavior"],
                "deny",
                "{t}"
            );
            assert_eq!(
                super::turn_permission(false, t, &serde_json::json!({}))["behavior"],
                "allow",
                "{t} without access=read"
            );
        });
}

#[test]
fn lane_ask_shows_in_open_questions() {
    let lane = |id: &str, status: &str| crate::app::Lane {
        id: id.into(),
        status: status.into(),
        item: "Traducir".into(),
        question: "¿Aprobar 0,74 $?".into(),
        ..Default::default()
    };
    let cards = super::waiting_cards(&[lane("l1", "waiting"), lane("l2", "running")]);
    let q = cards[0]["question"].to_string();
    assert!(
        cards.len() == 1
            && cards[0]["id"] == "l1"
            && q.contains("lane bloqueada esperando al dueño")
            && q.contains("0,74"),
        "{q}"
    );
}

#[test]
fn judged_reads_a_label_or_a_score_and_one_line_why() {
    assert_eq!(
        super::judged("approval — el dueño da las gracias por el arreglo"),
        (
            "approval".to_string(),
            None,
            "el dueño da las gracias por el arreglo".to_string()
        )
    );
    assert_eq!(
        super::judged("Correction: pide rehacer la portada").0,
        "correction"
    );
    assert_eq!(super::judged("`noise` — se queja de los avisos").0, "noise");
    assert_eq!(
        super::judged("\n\n0.25 — mostly not needed\n"),
        (String::new(), Some(0.25), "mostly not needed".to_string())
    );
    assert_eq!(
        super::judged("alone — era un valor por defecto reversible").0,
        "alone"
    );
    assert_eq!(super::judged("").0, "");
}

#[tokio::test]
async fn bot_routes_from_the_page() {
    let home = std::env::temp_dir().join(format!("tirith-bot-routes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = crate::app::App::open_at(home.clone());
    // The closed list of events the card offers (from /api/models): lane.failed is in it, schedule (the routines) is not.
    let events = crate::models::state(&app)["events"].clone();
    assert!(
        events.as_array().is_some_and(
            |es| es.iter().any(|e| e == "lane.failed") && es.iter().all(|e| e != "schedule")
        ),
        "the page's events: {events}"
    );
    // A new bot from its card on the page (POST /api/bots): a chat session of its own, read-only, hearing lane.failed as a note.
    let card = |routes: serde_json::Value| serde_json::json!({"name": "Avisos", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read", "routes": routes});
    let bot = crate::bots::save(&app, card(serde_json::json!([{"on": "lane.failed", "mode": "note", "chat": false, "phone": false}]))).unwrap();
    let (id, key) = (
        bot["id"].as_str().unwrap().to_string(),
        format!("bot-{}", bot["id"].as_str().unwrap()),
    );
    // Saved in ~/.tirith/config.toml (by toml_edit): its role with its session and access, and its route.
    let text = std::fs::read_to_string(home.join("config.toml")).unwrap();
    assert!(
        text.contains(&format!("[roles.{key}]"))
            && text.contains("session = \"chat\"")
            && text.contains("access = \"read\""),
        "its role:\n{text}"
    );
    assert!(
        text.contains("on = \"lane.failed\"")
            && text.contains(&format!("to = \"{key}\""))
            && text.contains("mode = \"note\""),
        "its route:\n{text}"
    );
    // And read back as the page sees it, over the factory.
    let shown = crate::bots::all(&app)
        .into_iter()
        .find(|b| b["key"] == key.as_str())
        .expect("the bot, as the page sees it");
    assert_eq!(
        (
            shown["session"].as_str(),
            shown["access"].as_str(),
            shown["routes"][0]["on"].as_str(),
            shown["routes"][0]["mode"].as_str()
        ),
        (
            Some("chat"),
            Some("read"),
            Some("lane.failed"),
            Some("note")
        )
    );
    let f = &app.cfg().factory;
    assert_eq!(
        (
            f.role::<String>(&key, "session").as_deref(),
            f.role::<String>(&key, "access").as_deref()
        ),
        (Some("chat"), Some("read"))
    );
    assert!(
        f.routes_on("lane.failed")
            .iter()
            .any(|r| r.0 == key && r.1 == "note"),
        "the factory routes lane.failed to it"
    );
    // The closed list is closed: an unnamed event, schedule (its routines) and a lane session are refused by the API, and nothing is written.
    for bad in [
        serde_json::json!([{"on": "no.such.event", "mode": "turn"}]),
        serde_json::json!([{"on": "schedule", "mode": "turn"}]),
        serde_json::json!([{"on": "lane.failed", "mode": "shout"}]),
    ] {
        let mut b = card(bad);
        b["id"] = serde_json::json!(id);
        assert!(crate::bots::save(&app, b).is_err());
    }
    let mut b = card(serde_json::json!([]));
    (b["id"], b["session"]) = (serde_json::json!(id.clone()), serde_json::json!("lane"));
    assert!(
        crate::bots::save(&app, b).is_err(),
        "a bot's session is chat or turn"
    );
    assert_eq!(
        std::fs::read_to_string(home.join("config.toml")).unwrap(),
        text,
        "a refused card changes nothing"
    );
    // It receives the event it hears (a note handed to its conversation for its next turn), and not one it does not.
    super::emit_event(
        &app,
        "some-project",
        "lane.failed",
        "lane x failed: no commits",
        None,
    );
    super::emit_event(&app, "some-project", "main.red", "main is red", None);
    assert_eq!(
        super::talk(&key, |t| t.notes.clone()),
        ["lane x failed: no commits"],
        "the bot received lane.failed, by its page route alone"
    );
    // Changed from the page again, its routes are replaced, not added beside: it hears main.red now, lane.failed no longer.
    let mut b =
        card(serde_json::json!([{"on": "main.red", "mode": "note", "chat": false, "phone": true}]));
    b["id"] = serde_json::json!(id);
    crate::bots::save(&app, b).unwrap();
    assert!(
        app.cfg()
            .factory
            .routes_on("lane.failed")
            .iter()
            .all(|r| r.0 != key),
        "the old route is gone from the factory it runs with"
    );
    super::emit_event(&app, "some-project", "lane.failed", "lane y failed", None);
    super::emit_event(&app, "some-project", "main.red", "main is red again", None);
    assert_eq!(
        super::talk(&key, |t| t.notes.clone()),
        ["lane x failed: no commits", "main is red again"]
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn lane_card_becomes_project_card() {
    let home = std::env::temp_dir().join(format!("tirith-lane-card-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = crate::app::App::open_at(home.clone());
    // A blocked audit lane, waiting, with its owner card (opened with --lane).
    app.save_lane(&crate::app::Lane {
        id: "audit-lane".into(),
        project: "jobs".into(),
        status: "waiting".into(),
        item: "auditoría bloqueada".into(),
        question: "¿sigo?".into(),
        ..Default::default()
    });
    app.insert("INSERT INTO owner_items (id, at, kind, project, question, status, answer, lane) VALUES ('o33', '2026-11-20T10:00:00Z', 'decision', 'jobs', '¿Apruebas el gasto de la auditoría?', 'open', '', 'audit-lane')", &[]);
    let row = |app: &crate::app::Shared| {
        app.query(
            "SELECT status, lane, project, kind, question FROM owner_items WHERE id = 'o33'",
            [],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            },
        )
        .into_iter()
        .next()
        .expect("card o33")
    };
    // While its lane still waits, nothing touches the card.
    super::close_stale_questions(&app);
    assert_eq!(row(&app).0.as_str(), "open");
    assert_eq!(row(&app).1.as_str(), "audit-lane");
    // The lane is answered (no longer waiting) and the sweep runs: the card is not closed unanswered, it becomes a card of its project.
    crate::lanes::answer(
        &app,
        "audit-lane",
        "verified: the audit stays blocked",
        "the manager",
    )
    .unwrap();
    assert_ne!(app.lane("audit-lane").unwrap().status, "waiting");
    super::close_stale_questions(&app);
    let (status, lane, project, kind, question) = row(&app);
    assert_eq!(
        (
            status.as_str(),
            lane.as_str(),
            project.as_str(),
            kind.as_str(),
            question.as_str()
        ),
        (
            "open",
            "",
            "jobs",
            "decision",
            "¿Apruebas el gasto de la auditoría?"
        )
    );
    // Still open for the owner under its own id, and a second sweep does not close it either: its question is not stale.
    assert!(
        super::owner_open(&app)
            .iter()
            .any(|o| o["id"] == "o33" && o["lane"] == ""),
        "o33 stays among the open cards"
    );
    super::close_stale_questions(&app);
    assert_eq!(row(&app).0.as_str(), "open");
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn agent_say_is_not_the_owner() {
    let home = std::env::temp_dir().join(format!("tirith-agent-say-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[[projects]]\nid = \"sayagent\"\nrepo = \"{}\"\n",
            home.display()
        ),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    // No live session, so what the manager is handed waits in its talk's pending, where it can be read here instead of starting an agent (a stdin channel would race the restart test, which marks every talk stale).
    super::talk("sayagent", |t| {
        (t.tx, t.alive, t.pending, t.notes) = (None, true, vec![], vec![])
    });
    let handed = || texts("sayagent");
    // The Manager's note from inside its session (its role set), even one shaped like the owner's /model: the manager gets it marked as the agent's, and nothing is the owner's.
    super::say(&app, "sayagent", "/model manager other", "manager").unwrap();
    let got = handed();
    assert!(
        got.contains("[agent manager] /model manager other"),
        "handed to the manager as the agent's, not run as a model command: {got}"
    );
    let chats = app.query(
        "SELECT role FROM chat WHERE text = '/model manager other'",
        [],
        |r| r.get::<_, String>(0),
    );
    assert_eq!(
        chats,
        ["manager"],
        "in the chat under its role, not as the owner"
    );
    let owners = app.query(
        "SELECT COUNT(*) FROM events WHERE kind = 'owner.message'",
        [],
        |r| r.get::<_, i64>(0),
    );
    assert_eq!(
        owners,
        [0],
        "no owner.message, so no owner-mood judging and no satisfaction from it"
    );
    // A lane's say is under its lane's name the same way.
    super::say(
        &app,
        "sayagent",
        "ya terminé la parte del login",
        "lane-login-1",
    )
    .unwrap();
    assert!(handed().contains("[agent lane-login-1] ya terminé la parte del login"));
    assert_eq!(
        app.query(
            "SELECT role FROM chat WHERE text = 'ya terminé la parte del login'",
            [],
            |r| r.get::<_, String>(0)
        ),
        ["lane-login-1"]
    );
    // Without a role or lane it still is the owner: an owner.message and an owner chat row.
    super::say(&app, "sayagent", "hola, soy el dueño", "").unwrap();
    assert!(handed().contains("hola, soy el dueño"));
    assert_eq!(
        app.query(
            "SELECT role FROM chat WHERE text = 'hola, soy el dueño'",
            [],
            |r| r.get::<_, String>(0)
        ),
        ["owner"]
    );
    assert_eq!(
        app.query(
            "SELECT COUNT(*) FROM events WHERE kind = 'owner.message'",
            [],
            |r| r.get::<_, i64>(0)
        ),
        [1]
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

/// One raw POST /api/chat against the test daemon, as the `tirith say` CLI sends it (with the session's lane/role headers).
async fn post_chat(port: u16, headers: &str, body: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    s.write_all(format!("POST /api/chat HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{headers}Connection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).await.unwrap();
    out
}

#[tokio::test]
async fn agent_say_is_not_the_owner_over_http() {
    let home = std::env::temp_dir().join(format!("tirith-agent-say-http-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[[projects]]\nid = \"sayhttp\"\nrepo = \"{}\"\n",
            home.display()
        ),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    super::talk("sayhttp", |t| {
        (t.tx, t.alive, t.pending, t.notes) = (None, true, vec![], vec![])
    });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let app2 = app.clone();
    let srv = tokio::spawn(async move {
        drop(axum::serve(listener, crate::server::router(app2, port)).await)
    });
    // `tirith say` from a manager session (X-Tirith-Role set, as client::call sends it): the daemon reads the agent from the headers.
    let out = post_chat(
        port,
        "X-Tirith-Lane: \r\nX-Tirith-Role: manager\r\n",
        r#"{"text": "nota del manager por http", "project": "sayhttp"}"#,
    )
    .await;
    assert!(out.contains(" 200"), "{out}");
    let handed = texts("sayhttp");
    assert!(
        handed.contains("[agent manager] nota del manager por http"),
        "handed to the manager as the agent's: {handed}"
    );
    assert_eq!(
        app.query(
            "SELECT role FROM chat WHERE text = 'nota del manager por http'",
            [],
            |r| r.get::<_, String>(0)
        ),
        ["manager"]
    );
    // From a lane both headers may ride; the lane is the name that counts.
    let out = post_chat(
        port,
        "X-Tirith-Lane: lane-http-1\r\nX-Tirith-Role: manager\r\n",
        r#"{"text": "nota de la lane por http", "project": "sayhttp"}"#,
    )
    .await;
    assert!(out.contains(" 200"), "{out}");
    assert_eq!(
        app.query(
            "SELECT role FROM chat WHERE text = 'nota de la lane por http'",
            [],
            |r| r.get::<_, String>(0)
        ),
        ["lane-http-1"]
    );
    assert_eq!(
        app.query(
            "SELECT COUNT(*) FROM events WHERE kind = 'owner.message'",
            [],
            |r| r.get::<_, i64>(0)
        ),
        [0],
        "neither say was the owner's"
    );
    srv.abort();
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn progress_line_never_sends_a_raw_command_when_a_summary_exists() {
    let bash = serde_json::json!({"command": "curl -H 'Authorization: Bearer secret' https://example.com", "description": "Check the service is up"});
    assert_eq!(
        super::progress_line("Bash", &bash),
        "Bash: Check the service is up",
        "the tool's own summary, not its command"
    );
    assert_eq!(
        super::progress_line(
            "Read",
            &serde_json::json!({"file_path": "/repo/src/main.rs"})
        ),
        "Read: /repo/src/main.rs"
    );
    assert_eq!(
        super::progress_line("Bash", &serde_json::json!({"command": "rm -rf /tmp/x"})),
        "Bash",
        "no summary: only the tool's name, still no raw command"
    );
    assert_eq!(
        super::progress_line(
            "Grep",
            &serde_json::json!({"pattern": "typing\nsecond line"})
        ),
        "Grep: typing",
        "one short line"
    );
}

#[test]
fn typing_rooms_are_the_turns_rooms_alone() {
    let home = std::env::temp_dir().join(format!("tirith-typing-rooms-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[[projects]]\nid = \"typingturn\"\nrepo = \"{}\"\n",
            home.display()
        ),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    // A turn a room started shows its progress in that room, also when the turn is not marked for the phones.
    super::talk("typingturn", |t| {
        (t.reply_to, t.phones) = (["sealed-room".to_string()].into(), false)
    });
    assert_eq!(super::typing_rooms(&app, "typingturn"), ["sealed-room"]);
    // A page turn with no rooms and no phone mark writes in no room.
    super::talk("typingturn", |t| {
        (t.reply_to, t.phones) = (Default::default(), false)
    });
    assert!(
        super::typing_rooms(&app, "typingturn").is_empty(),
        "a turn without rooms emits no control"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn card_id_of_reads_the_card_an_event_line_names() {
    assert_eq!(
        super::card_id_of("Card o12 closed by the manager: ya no hace falta").as_deref(),
        Some("o12")
    );
    assert_eq!(
        super::card_id_of("The owner answered o3 (decision: ¿sigo?): sí").as_deref(),
        Some("o3")
    );
    assert_eq!(
        super::card_id_of("o7 (payment): ¿pago 3 $?").as_deref(),
        Some("o7")
    );
    assert_eq!(super::card_id_of("merged lane lo-que-sea into main"), None);
    assert_eq!(super::card_id_of("owner without a card number"), None);
}

/// What a conversation's waiting messages say, taken.
fn texts(project: &str) -> String {
    let taken = super::talk(project, |t| std::mem::take(&mut t.pending));
    taken
        .into_iter()
        .map(|m| m.text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn reports_are_never_dropped_silently() {
    let home = std::env::temp_dir().join(format!("tirith-notes-cut-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = crate::app::App::open_at(home.clone());
    // No live session: what the manager is handed waits in its talk's pending.
    super::talk("notescut", |t| {
        (t.tx, t.alive, t.pending, t.notes) = (None, true, vec![], vec![])
    });
    (0..35)
        .for_each(|i| super::queue::queue(&app, "notescut", &format!("report {i}"), "note", false));
    super::deliver(&app, "notescut", "hola", false);
    let got = texts("notescut");
    assert!(
        got.contains("5 older reports were left out")
            && got.contains("report 34")
            && !got.contains("report 4\n"),
        "the 30 newest are kept and the rest is said: {got}"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_judge_that_corrected_itself_is_read_by_its_last_verdict() {
    let rubric = "Answer with one line:\n\n- `unapproved` — it acted.\n- `ok` — it did not.\n";
    let last = |text: &'static str| super::judged(super::verdict::last(text, rubric)).0;
    let fixed = "unapproved — it only asked, so this is actually ok.\n\n**ok — it only opened an approval card and stopped.**";
    assert_eq!(last(fixed), "ok");
    let quoted = "unapproved — hmm.\n\nCorrecting my own output: `ok — it edited its own memory.`";
    assert_eq!(last(quoted), "ok");
    assert_eq!(
        last("unapproved — it sent the e-mail without a yes"),
        "unapproved"
    );
    assert_eq!(last("ok — nothing unapproved was done"), "ok");
    assert_eq!(
        super::verdict::last("0.7 — a small gap", "- `1` — all"),
        "0.7 — a small gap"
    );
    let thought =
        "It ran the command and the year is 2026.\n\n0.9 — the year only, its check not shown";
    let scored = super::judged(super::verdict::last(thought, "- `1` — all"));
    assert_eq!(scored.1, Some(0.9));
    let first = "The owner asks for a new thing, no reaction.\nneutral — a new task";
    let moods = "- `approval` — pleased.\n- `neutral` — anything else.";
    assert_eq!(
        super::judged(super::verdict::last(first, moods)).0,
        "neutral"
    );
    let dashed = "Read-only.\n\n— ok — only a date command ran";
    assert_eq!(super::judged(super::verdict::last(dashed, rubric)).0, "ok");
    let no_score = "It printed the year.\n\n2026 — matches the task";
    assert!(
        !super::verdict::given(no_score, "- `1` — all"),
        "the answer is no score"
    );
}

#[tokio::test]
async fn a_note_outlives_a_restart_until_a_message_hands_it_over() {
    let key = "bot-note01";
    let home = std::env::temp_dir().join(format!("tirith-note-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("config.toml"), format!("[network]\nsandbox = false\n\n[roles.{key}]\nextends = \"manager\"\nscope = \"own\"\nsession = \"chat\"\nname = \"Notas\"\n")).unwrap();
    let app = crate::app::App::open_at(home.clone());
    assert_eq!(app.conv(key), key, "the bot's own conversation");
    super::noted(&app, key, "it is no longer a draft");
    super::talk(key, |t| t.notes.clear()); // a restart: what was in memory is gone
    super::start(app.clone());
    assert_eq!(
        super::waiting_notes(key),
        vec!["it is no longer a draft"],
        "it waits again"
    );
    super::talk(key, |t| t.alive = true); // a session is on its way: none is started here
    super::deliver(&app, key, "hola", false);
    let said = super::talk(key, |t| {
        t.pending
            .iter()
            .map(|m| m.text.clone())
            .collect::<Vec<_>>()
            .join("|")
    });
    assert!(
        said.contains("it is no longer a draft"),
        "the next message carries it: {said}"
    );
    super::start(app.clone());
    assert!(super::waiting_notes(key).is_empty(), "handed once");
    super::talk(key, |t| *t = Default::default());
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
