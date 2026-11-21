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
