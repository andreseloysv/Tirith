#[test]
fn health_reports_paused_reason() {
    let t = super::paused_text("tirith", "commit guard: the pre-commit hook is outdated or has other words than config.toml (tirith guard install tirith)");
    assert!(t.contains("paused: commit guard"), "{t}");
    assert!(t.contains("fix: tirith guard install tirith"), "{t}");
    assert!(super::paused_text("p", "3 lanes wait for the merge queue").contains("fix: wait for the merges"));
    assert!(super::paused_text("p", "less than 5 GB free on /x").contains("fix: free disk space"));
}
#[test]
fn unasked_turns_follow_the_last_mark_of_their_conversation() {
    let mark = |at: &str, p: &str, unasked: bool| (at.to_string(), p.to_string(), unasked);
    let msg = |at: &str, p: &str, text: &str| (at.to_string(), p.to_string(), text.to_string());
    let marks = vec![mark("2026-11-20T10:00:00Z", "p", false), mark("2026-11-20T10:05:00Z", "p", true), mark("2026-11-20T10:10:00Z", "p", false), mark("2026-11-20T10:05:00Z", "q", false)];
    let msgs = vec![
        msg("2026-11-20T09:59:00Z", "p", "before any mark: not counted"),
        msg("2026-11-20T10:01:00Z", "p", "the owner asked: not counted"),
        msg("2026-11-20T10:06:00Z", "p", "an event with phone = true started this turn: counted"),
        msg("2026-11-20T10:07:00Z", "p", "∅ a routine silence never reaches the phones"),
        msg("2026-11-20T10:08:00Z", "q", "another conversation, asked: not counted"),
        msg("2026-11-20T10:11:00Z", "p", "the owner asked again: not counted"),
    ];
    assert_eq!(super::unasked(&marks, &msgs), 1);
}
#[test]
fn owner_measures_count_interruptions_and_needless_cards() {
    let home = std::env::temp_dir().join(format!("tirith-owner-measures-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("TIRITH_HOME", &home);
    let app = crate::app::App::open();
    let at = |ago: u64| chrono::DateTime::from_timestamp(crate::app::now_s().saturating_sub(ago) as i64, 0).unwrap().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    for id in ["m1", "m2"] {
        app.save_lane(&crate::app::Lane { id: id.into(), project: "p".into(), status: "merged".into(), created_at: at(3600), merged_at: Some(at(60)), ..Default::default() });
    }
    let event = |ago: u64, kind: &str| app.insert("INSERT INTO events (at, lane, project, kind, tool, summary, session) VALUES (?1,'','p',?2,'','','')", rusqlite::params![at(ago), kind]);
    event(90000, "lane.failed"); // older than 24 h: not counted
    event(500, "owner.message");
    event(400, "lane.failed"); // its route is turn + chat + phone = true: a notice, and it starts an unasked turn
    event(300, "merge.landed"); // phone = false since factory step 4: neither
    event(200, "owner.message");
    let say = |ago: u64, text: &str| app.insert("INSERT INTO chat (at, role, text, project) VALUES (?1,'manager',?2,'p')", rusqlite::params![at(ago), text]);
    say(450, "answer to the owner");
    say(350, "unasked one");
    say(340, "∅ nothing new");
    say(250, "still unasked");
    say(150, "answer to the owner again");
    let card = |id: &str, ago: u64, status: &str| app.insert("INSERT INTO owner_items (id, at, kind, project, question, status, answer, lane) VALUES (?1,?2,'decision','p','q?',?3,'','')", rusqlite::params![id, at(ago), status]);
    card("o0", 90000, "closed"); // older than 24 h: not counted
    card("o1", 400, "answered");
    card("o2", 390, "closed");
    card("o3", 380, "closed");
    card("o4", 370, "open");
    let metrics = super::compute(&app);
    let get = |id: &str| metrics.iter().find(|m| m.id == id).unwrap_or_else(|| panic!("no metric {id}"));
    // 2 unasked phone messages + 1 notice with phone = true + 4 cards, over 2 merged items.
    let interruptions = get("interruptions");
    assert_eq!((interruptions.label.as_str(), interruptions.value, interruptions.target, interruptions.red), ("Interrupciones por ítem (24 h)", 3.5, Some(1.0), true), "{}", interruptions.facts);
    assert!(interruptions.facts.contains("2 phone messages") && interruptions.facts.contains("1 notices") && interruptions.facts.contains("4 cards"), "{}", interruptions.facts);
    // Of the 3 cards resolved in 24 h, 2 were closed without an answer.
    let needless = get("needless_cards");
    assert_eq!((needless.label.as_str(), needless.value, needless.target, needless.red), ("Tarjetas innecesarias (%)", 66.7, Some(25.0), true), "{}", needless.facts);
    assert!(needless.facts.contains("[o2, o3]") && needless.facts.contains("answered 1"), "{}", needless.facts);
    // No mood judged yet: satisfaction sits at 0 and is not red, whatever the owner wrote.
    let satisfaction = get("satisfaction");
    assert_eq!((satisfaction.label.as_str(), satisfaction.value, satisfaction.target, satisfaction.red), ("Satisfacción del dueño (24 h)", 0.0, Some(0.0), false), "{}", satisfaction.facts);

    // The judge's scores land (each with its factory stamp): 5 owner messages judged 1 approval, 2 corrections,
    // 1 repeat and 1 noise complaint, and the one answered card judged decidable alone.
    for ago in [150, 140, 130] {
        event(ago, "owner.message");
    }
    let score = |event: i64, eval: &str, label: &str| {
        app.insert("INSERT INTO scores (at, event, eval, label, score, why, factory, role_hash) VALUES (?1, ?2, ?3, ?4, NULL, 'why', 'v1+abc12345', 'judge123')", rusqlite::params![at(100), event, eval, label])
    };
    for (i, label) in ["approval", "correction", "correction", "repeat", "noise"].iter().enumerate() {
        score(i as i64 + 1, "owner-mood", label);
    }
    score(90000, "owner-mood", "approval"); // its `at` below is what counts, so move it out of the day
    app.insert("UPDATE scores SET at = ?1 WHERE event = 90000", rusqlite::params![at(90000)]);
    let answered = app.insert("INSERT INTO events (at, lane, project, kind, tool, summary, session) VALUES (?1,'','p','card.answered','','The owner answered o1 (decision: q?): yes','')", rusqlite::params![at(120)]);
    score(answered, "card-needed", "alone");
    let closed_ev = app.insert("INSERT INTO events (at, lane, project, kind, tool, summary, session) VALUES (?1,'','p','card.closed','','Card o2 closed by the manager: no longer needed','')", rusqlite::params![at(110)]);
    score(closed_ev, "card-needed", "alone"); // already counted as closed without an answer: never twice
    let metrics = super::compute(&app);
    let get = |id: &str| metrics.iter().find(|m| m.id == id).unwrap_or_else(|| panic!("no metric {id}"));
    // (1 approval − 2 corrections − 1 repeat − 1 noise) / 5 owner messages, judged, never asked.
    let satisfaction = get("satisfaction");
    assert_eq!((satisfaction.value, satisfaction.red), (-0.6, true), "{}", satisfaction.facts);
    assert!(satisfaction.facts.contains("1 approvals") && satisfaction.facts.contains("2 corrections") && satisfaction.facts.contains("over 5 owner messages"), "{}", satisfaction.facts);
    // The answered card the eval says the factory could have decided alone joins the needless ones: 3 of 3.
    let needless = get("needless_cards");
    assert_eq!(needless.value, 100.0, "{}", needless.facts);
    assert!(needless.facts.contains("judged decidable alone: [o1]"), "{}", needless.facts);
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
#[test]
fn health_merged_reports_empty_plans() {
    let t = super::empty_plan_note(&["tirith".to_string(), "p2".to_string()]);
    assert!(t.contains("tirith plan empty") && t.contains("fix: tirith plan add tirith <step>"), "{t}");
    assert!(t.contains("p2 plan empty"), "{t}");
    assert!(super::empty_plan_note(&[]).is_empty());
}
