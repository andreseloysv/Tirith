#[test]
fn unapproved_is_measured() {
    let home = std::env::temp_dir().join(format!("tirith-unapproved-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = crate::app::App::open_at(home.clone());
    let at = |ago: u64| super::metric::iso(crate::app::now_s().saturating_sub(ago) as i64);
    let get = |app: &crate::app::Shared| {
        super::compute::compute(app)
            .into_iter()
            .find(|m| m.id == "unapproved")
            .expect("the unapproved measure")
    };
    // Nothing judged yet: 0 and not red.
    let m = get(&app);
    assert_eq!(
        (m.label.as_str(), m.value, m.target, m.at_least, m.red),
        ("Unapproved (24 h)", 0.0, Some(0.0), false, false)
    );
    let score = |ago: u64, label: &str, why: &str| {
        app.insert("INSERT INTO scores (at, event, eval, label, score, why, factory, role_hash) VALUES (?1, 1, 'unapproved', ?2, NULL, ?3, 'v1+abc12345', 'judge123')", rusqlite::params![at(ago), label, why])
    };
    // Only ok verdicts: still green.
    score(300, "ok", "solo leyó");
    let m = get(&app);
    assert_eq!((m.value, m.red), (0.0, false), "{}", m.facts);
    // Two unapproved verdicts in 24 h (and one older, not counted): red, with the why in its facts.
    score(200, "unapproved", "envió un correo sin tarjeta");
    score(100, "unapproved", "borró una carpeta del dueño");
    score(90000, "unapproved", "viejo");
    let m = get(&app);
    assert_eq!((m.value, m.red), (2.0, true), "{}", m.facts);
    assert!(
        m.facts.contains("judged in 24 h: 3")
            && m.facts.contains("envió un correo sin tarjeta")
            && !m.facts.contains("viejo"),
        "{}",
        m.facts
    );
    // Red, it is routed like every measure: its episode is fresh (tick announces metric.red with this line) and the factory routes metric.red to a role.
    assert_eq!(
        super::episode::episode(&app, m.id, m.red, 1_000, 900),
        (true, false, Some(1_000))
    );
    assert!(super::metric::describe(&m).contains("[unapproved] is 2"));
    assert!(app
        .cfg()
        .factory
        .routes_on("metric.red")
        .iter()
        .any(|r| r.1 == "turn"));
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
