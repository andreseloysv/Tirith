use serde_json::json;

/// The status and the event line of one kept turn.
fn row(app: &crate::app::Shared, id: i64) -> (String, String) {
    app.query(
        "SELECT t.status, COALESCE(e.summary, '') FROM turns t LEFT JOIN events e ON e.id = t.event WHERE t.id = ?1",
        [id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    )
    .into_iter()
    .next()
    .expect("the turn's row")
}

#[test]
fn approval_turn_waits() {
    let home = std::env::temp_dir().join(format!("tirith-approval-waits-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = crate::app::App::open_at(home.clone());
    let bot = crate::bots::save(&app, json!({"name": "Correo", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let key = format!("bot-{}", bot["id"].as_str().unwrap());
    // A skill it reads, with what needs the owner's yes.
    let skill = home
        .join("manager")
        .join(&key)
        .join(".agents/skills/correo");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), "---\nname: correo\n---\n\n## Pasos\n1. Redactar.\n\n## Qué requiere aprobación\nEnviar cualquier correo.\n\n## Si algo falla\nDecirlo.\n").unwrap();
    let start = || super::Turn::new(&app, &key, &key, ("owner".into(), String::new()));

    // A turn that opens no approval card ends ok.
    let plain = super::save::finish(&app, start(), Some(true), Some("hecho"));
    assert_eq!(row(&app, plain).0, "ok");

    // A turn that opens an approval card in its conversation and ends with it open waits for the owner.
    let mut t = start();
    t.skills.push("correo".into());
    t.tools.push(json!({"name": "Bash", "summary": "tirith ask-owner --kind approval \"Enviar a ana el presupuesto\""}));
    crate::manager::ask_owner(&app, &key, "approval", "Enviar a ana el presupuesto", "")
        .expect("an approval card");
    let waiting = super::save::finish(&app, t, Some(true), Some("Espero tu sí."));
    let (status, summary) = row(&app, waiting);
    assert_eq!(status, "waiting", "{summary}");
    // Its turn.done carries what the unapproved eval reads: the tool calls and the skill's approval section.
    assert!(
        summary.contains("tool calls: Bash: tirith ask-owner --kind approval")
            && summary.contains("«correo» requires approval for: Enviar cualquier correo."),
        "{summary}"
    );
    assert_eq!(super::approval::section("## Pasos\nx\n"), "");

    // A card of another kind, or one already answered, does not make a turn wait.
    let mut t = start();
    t.started_at = "2000-01-01T00:00:00Z".into();
    app.insert(
        "UPDATE owner_items SET status = 'answered' WHERE kind = 'approval'",
        &[],
    );
    crate::manager::ask_owner(&app, &key, "decision", "¿Rojo o azul?", "").unwrap();
    let answered = super::save::finish(&app, t, Some(true), Some("hecho"));
    assert_eq!(row(&app, answered).0, "ok");

    // A judge's turn in the bot's conversation emits no turn.done: the unapproved eval on it would judge itself forever.
    let judge = super::Turn::new(
        &app,
        &key,
        "judge",
        ("event:turn.done".into(), String::new()),
    );
    let judged = super::save::finish(&app, judge, Some(true), Some("ok — solo leyó"));
    assert_eq!(row(&app, judged).1, "");
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

/// A home with a bot whose skill `correo` has a «Cómo validar» and a «Qué requiere aprobación»: (the app, its home, its key).
fn bot_with_a_skill(name: &str) -> (crate::app::Shared, std::path::PathBuf, String) {
    let home = std::env::temp_dir().join(format!("tirith-approval-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = crate::app::App::open_at(home.clone());
    let bot = crate::bots::save(&app, json!({"name": "Correo", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let key = format!("bot-{}", bot["id"].as_str().unwrap());
    let skill = home
        .join("manager")
        .join(&key)
        .join(".agents/skills/correo");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), "---\nname: correo\n---\n\n## Cómo validar\nCada correo tiene asunto.\n\n## Qué requiere aprobación\nEnviar cualquier correo.\n").unwrap();
    (app, home, key)
}

#[test]
fn the_judge_keeps_the_tool_calls_and_the_approvals_of_a_long_run() {
    let (app, home, key) = bot_with_a_skill("evidence");
    let mut t = super::Turn::new(&app, &key, &key, ("schedule".into(), "r1".into()));
    t.skills.push("correo".into());
    // A routine with a long task, many calls and a long answer: the last call is the one that sent.
    (0..40).for_each(|i| t.tools.push(json!({"name": "Bash", "summary": format!("curl https://example.com/{i} {}", "x".repeat(150))})));
    t.tools
        .push(json!({"name": "Bash", "summary": "tirith send --to ana LAST-CALL"}));
    let id = super::save::finish(&app, t, Some(true), Some(&"r".repeat(4000)));
    let (_, summary) = row(&app, id);
    assert!(
        summary.contains("LAST-CALL")
            && summary.contains("«correo» requires approval for: Enviar cualquier correo.")
            && summary.contains("how to validate «correo»: Cada correo tiene asunto."),
        "the judge reads the whole evidence: {} characters",
        summary.len()
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn only_a_chat_turn_that_read_a_skill_is_graded() {
    let (app, home, key) = bot_with_a_skill("graded");
    let kind = |id: i64| -> String {
        let sql = "SELECT e.kind FROM turns t JOIN events e ON e.id = t.event WHERE t.id = ?1";
        app.query(sql, [id], |r| r.get(0)).remove(0)
    };
    let chat = |skills: &[&str]| {
        let mut t = super::Turn::new(&app, &key, &key, ("owner".into(), String::new()));
        skills.iter().for_each(|s| t.skills.push(s.to_string()));
        super::save::finish(&app, t, Some(true), Some("hecho"))
    };
    let plain = chat(&[]);
    assert_eq!(
        kind(plain),
        "turn.done",
        "no skill: no judge turn for the grade"
    );
    let with = chat(&["correo"]);
    assert_eq!(
        kind(with),
        "routine.done",
        "a skill: graded with its «Cómo validar»"
    );
    assert!(row(&app, with)
        .1
        .contains("how to validate «correo»: Cada correo tiene asunto."));
    // The factory grades routine.done and does not grade turn.done.
    let f = &app.cfg().factory;
    assert!(f
        .evals_on("routine.done")
        .iter()
        .any(|e| e.0 == "turn-quality"));
    assert!(f
        .evals_on("turn.done")
        .iter()
        .all(|e| e.0 != "turn-quality"));
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn the_owners_answer_ends_the_wait() {
    let (app, home, key) = bot_with_a_skill("answer");
    let waiting = |app: &crate::app::Shared| -> Vec<String> {
        let sql = "SELECT status FROM turns WHERE conv = ?1 AND role = ?1 ORDER BY id";
        app.query(sql, [&key], |r| r.get(0))
    };
    let card = |question: &str| {
        let t = super::Turn::new(&app, &key, &key, ("schedule".into(), "r1".into()));
        let c = crate::manager::ask_owner(&app, &key, "approval", question, "").unwrap();
        (
            super::save::finish(&app, t, Some(true), Some("Espero tu sí.")),
            c["id"].as_str().unwrap().to_string(),
        )
    };
    let (_, first) = card("Enviar a ana el presupuesto");
    let (_, second) = card("Enviar a luis el contrato");
    assert_eq!(waiting(&app), ["waiting", "waiting"]);
    // Each turn waits while any approval card opened since it started is open.
    crate::manager::owner_answer(&app, &first, "sí", true).unwrap(); // the manager closes it: no turn is started here
    assert_eq!(
        waiting(&app),
        ["waiting", "waiting"],
        "a card is still open"
    );
    crate::manager::close_card(&app, &second, "the owner", "ya no").unwrap();
    assert_eq!(
        waiting(&app),
        ["ok", "ok"],
        "no approval card is open: no turn waits"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
