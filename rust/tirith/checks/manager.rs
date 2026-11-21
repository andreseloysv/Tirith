#[test]
fn restart_waits_for_a_running_turn() {
    super::talk("restart-test", |t| (t.busy, t.pid) = (true, 0));
    super::restart();
    assert!(super::talk("restart-test", |t| t.stale));
}

#[test]
fn turn_read_only_allows_reads() {
    for cmd in ["tirith status", "tirith plan show tirith", "tirith lane log some-lane", "tirith timeout get", "tirith merge status"] {
        let input = serde_json::json!({"command": cmd});
        assert_eq!(super::turn_permission(true, "Bash", &input)["behavior"], "allow", "{cmd}");
    }
    assert_eq!(super::turn_permission(true, "Read", &serde_json::json!({"file_path": "/x"}))["behavior"], "allow");
    ["Write", "Edit", "MultiEdit", "NotebookEdit"].into_iter().for_each(|t| {
        assert_eq!(super::turn_permission(true, t, &serde_json::json!({}))["behavior"], "deny", "{t}");
        assert_eq!(super::turn_permission(false, t, &serde_json::json!({}))["behavior"], "allow", "{t} without access=read");
    });
}

#[test]
fn lane_ask_shows_in_open_questions() {
    let lane = |id: &str, status: &str| crate::app::Lane { id: id.into(), status: status.into(), item: "Traducir".into(), question: "¿Aprobar 0,74 $?".into(), ..Default::default() };
    let cards = super::waiting_cards(&[lane("l1", "waiting"), lane("l2", "running")]);
    let q = cards[0]["question"].to_string();
    assert!(cards.len() == 1 && cards[0]["id"] == "l1" && q.contains("lane bloqueada esperando al dueño") && q.contains("0,74"), "{q}");
}

#[test]
fn judged_reads_a_label_or_a_score_and_one_line_why() {
    assert_eq!(super::judged("approval — el dueño da las gracias por el arreglo"), ("approval".to_string(), None, "el dueño da las gracias por el arreglo".to_string()));
    assert_eq!(super::judged("Correction: pide rehacer la portada").0, "correction");
    assert_eq!(super::judged("`noise` — se queja de los avisos").0, "noise");
    assert_eq!(super::judged("\n\n0.25 — mostly not needed\n"), (String::new(), Some(0.25), "mostly not needed".to_string()));
    assert_eq!(super::judged("alone — era un valor por defecto reversible").0, "alone");
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
    assert!(events.as_array().is_some_and(|es| es.iter().any(|e| e == "lane.failed") && es.iter().all(|e| e != "schedule")), "the page's events: {events}");
    // A new bot from its card on the page (POST /api/bots): a chat session of its own, read-only, hearing lane.failed as a note.
    let card = |routes: serde_json::Value| serde_json::json!({"name": "Avisos", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read", "routes": routes});
    let bot = crate::bots::save(&app, card(serde_json::json!([{"on": "lane.failed", "mode": "note", "chat": false, "phone": false}]))).unwrap();
    let (id, key) = (bot["id"].as_str().unwrap().to_string(), format!("bot-{}", bot["id"].as_str().unwrap()));
    // Saved in ~/.tirith/config.toml (by toml_edit): its role with its session and access, and its route.
    let text = std::fs::read_to_string(home.join("config.toml")).unwrap();
    assert!(text.contains(&format!("[roles.{key}]")) && text.contains("session = \"chat\"") && text.contains("access = \"read\""), "its role:\n{text}");
    assert!(text.contains("on = \"lane.failed\"") && text.contains(&format!("to = \"{key}\"")) && text.contains("mode = \"note\""), "its route:\n{text}");
    // And read back as the page sees it, over the factory.
    let shown = crate::bots::all(&app).into_iter().find(|b| b["key"] == key.as_str()).expect("the bot, as the page sees it");
    assert_eq!((shown["session"].as_str(), shown["access"].as_str(), shown["routes"][0]["on"].as_str(), shown["routes"][0]["mode"].as_str()), (Some("chat"), Some("read"), Some("lane.failed"), Some("note")));
    let f = &app.cfg().factory;
    assert_eq!((f.role::<String>(&key, "session").as_deref(), f.role::<String>(&key, "access").as_deref()), (Some("chat"), Some("read")));
    assert!(f.routes_on("lane.failed").iter().any(|r| r.0 == key && r.1 == "note"), "the factory routes lane.failed to it");
    // The closed list is closed: an unnamed event, schedule (its routines) and a lane session are refused by the API, and nothing is written.
    for bad in [serde_json::json!([{"on": "no.such.event", "mode": "turn"}]), serde_json::json!([{"on": "schedule", "mode": "turn"}]), serde_json::json!([{"on": "lane.failed", "mode": "shout"}])] {
        let mut b = card(bad);
        b["id"] = serde_json::json!(id);
        assert!(crate::bots::save(&app, b).is_err());
    }
    let mut b = card(serde_json::json!([]));
    (b["id"], b["session"]) = (serde_json::json!(id.clone()), serde_json::json!("lane"));
    assert!(crate::bots::save(&app, b).is_err(), "a bot's session is chat or turn");
    assert_eq!(std::fs::read_to_string(home.join("config.toml")).unwrap(), text, "a refused card changes nothing");
    // It receives the event it hears (a note handed to its conversation for its next turn), and not one it does not.
    super::emit_event(&app, "some-project", "lane.failed", "lane x failed: no commits", None);
    super::emit_event(&app, "some-project", "main.red", "main is red", None);
    assert_eq!(super::talk(&key, |t| t.notes.clone()), ["lane x failed: no commits"], "the bot received lane.failed, by its page route alone");
    // Changed from the page again, its routes are replaced, not added beside: it hears main.red now, lane.failed no longer.
    let mut b = card(serde_json::json!([{"on": "main.red", "mode": "note", "chat": false, "phone": true}]));
    b["id"] = serde_json::json!(id);
    crate::bots::save(&app, b).unwrap();
    assert!(app.cfg().factory.routes_on("lane.failed").iter().all(|r| r.0 != key), "the old route is gone from the factory it runs with");
    super::emit_event(&app, "some-project", "lane.failed", "lane y failed", None);
    super::emit_event(&app, "some-project", "main.red", "main is red again", None);
    assert_eq!(super::talk(&key, |t| t.notes.clone()), ["lane x failed: no commits", "main is red again"]);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn lane_card_becomes_project_card() {
    let home = std::env::temp_dir().join(format!("tirith-lane-card-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = crate::app::App::open_at(home.clone());
    // A blocked audit lane, waiting, with its owner card (opened with --lane).
    app.save_lane(&crate::app::Lane { id: "audit-lane".into(), project: "jobs".into(), status: "waiting".into(), item: "auditoría bloqueada".into(), question: "¿sigo?".into(), ..Default::default() });
    app.insert("INSERT INTO owner_items (id, at, kind, project, question, status, answer, lane) VALUES ('o33', '2026-11-20T10:00:00Z', 'decision', 'jobs', '¿Apruebas el gasto de la auditoría?', 'open', '', 'audit-lane')", &[]);
    let row = |app: &crate::app::Shared| {
        app.query("SELECT status, lane, project, kind, question FROM owner_items WHERE id = 'o33'", [], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?)))
            .into_iter()
            .next()
            .expect("card o33")
    };
    // While its lane still waits, nothing touches the card.
    super::close_stale_questions(&app);
    assert_eq!(row(&app).0.as_str(), "open");
    assert_eq!(row(&app).1.as_str(), "audit-lane");
    // The lane is answered (no longer waiting) and the sweep runs: the card is not closed unanswered, it becomes a card of its project.
    crate::lanes::answer(&app, "audit-lane", "verified: the audit stays blocked", "the manager").unwrap();
    assert_ne!(app.lane("audit-lane").unwrap().status, "waiting");
    super::close_stale_questions(&app);
    let (status, lane, project, kind, question) = row(&app);
    assert_eq!((status.as_str(), lane.as_str(), project.as_str(), kind.as_str(), question.as_str()), ("open", "", "jobs", "decision", "¿Apruebas el gasto de la auditoría?"));
    // Still open for the owner under its own id, and a second sweep does not close it either: its question is not stale.
    assert!(super::owner_open(&app).iter().any(|o| o["id"] == "o33" && o["lane"] == ""), "o33 stays among the open cards");
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
    let app = crate::app::App::open_at(home.clone());
    // A live session's stdin, so what the manager is handed can be read here instead of starting an agent.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    super::talk("manager-general", |t| (t.tx, t.alive) = (Some(tx), true));
    let handed = |rx: &mut tokio::sync::mpsc::UnboundedReceiver<String>| std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>().join("\n");
    // The Manager's note from inside its session (its role set), even one shaped like the owner's /model: the manager gets it marked as the agent's, and nothing is the owner's.
    super::say(&app, "", "/model manager other", "manager").unwrap();
    let got = handed(&mut rx);
    assert!(got.contains("[agent manager] /model manager other"), "handed to the manager as the agent's, not run as a model command: {got}");
    let chats = app.query("SELECT role FROM chat WHERE text = '/model manager other'", [], |r| r.get::<_, String>(0));
    assert_eq!(chats, ["manager"], "in the chat under its role, not as the owner");
    let owners = app.query("SELECT COUNT(*) FROM events WHERE kind = 'owner.message'", [], |r| r.get::<_, i64>(0));
    assert_eq!(owners, [0], "no owner.message, so no owner-mood judging and no satisfaction from it");
    // A lane's say is under its lane's name the same way.
    super::say(&app, "", "ya terminé la parte del login", "lane-login-1").unwrap();
    assert!(handed(&mut rx).contains("[agent lane-login-1] ya terminé la parte del login"));
    assert_eq!(app.query("SELECT role FROM chat WHERE text = 'ya terminé la parte del login'", [], |r| r.get::<_, String>(0)), ["lane-login-1"]);
    // Without a role or lane it still is the owner: an owner.message and an owner chat row.
    super::say(&app, "", "hola, soy el dueño", "").unwrap();
    assert!(handed(&mut rx).contains("hola, soy el dueño"));
    assert_eq!(app.query("SELECT role FROM chat WHERE text = 'hola, soy el dueño'", [], |r| r.get::<_, String>(0)), ["owner"]);
    assert_eq!(app.query("SELECT COUNT(*) FROM events WHERE kind = 'owner.message'", [], |r| r.get::<_, i64>(0)), [1]);
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn card_id_of_reads_the_card_an_event_line_names() {
    assert_eq!(super::card_id_of("Card o12 closed by the manager: ya no hace falta").as_deref(), Some("o12"));
    assert_eq!(super::card_id_of("The owner answered o3 (decision: ¿sigo?): sí").as_deref(), Some("o3"));
    assert_eq!(super::card_id_of("o7 (payment): ¿pago 3 $?").as_deref(), Some("o7"));
    assert_eq!(super::card_id_of("merged lane lo-que-sea into main"), None);
    assert_eq!(super::card_id_of("owner without a card number"), None);
}
