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
