// A routine's change of schedule says its days, its time and when it next runs.
use serde_json::json;

#[test]
fn a_change_of_days_or_time_is_said_with_the_next_run() {
    let home = std::env::temp_dir().join(format!("tirith-reschedule-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        "[roles.bot-horario]\nextends = \"manager\"\nscope = \"own\"\nsession = \"chat\"\nname = \"Riega\"\n\n[[routes]]\non = \"schedule\"\nto = \"bot-horario\"\nmode = \"turn\"\nid = \"r1\"\ntitle = \"Regar\"\ntask = \"water\"\ndays = [1, 2, 3, 4, 5]\nat = \"08:00\"\n",
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    let change = |body: serde_json::Value| {
        super::routine::routine(&app, "horario".into(), body).unwrap();
        let sql =
            "SELECT text FROM chat WHERE project = 'bot-horario' AND role = 'routine' ORDER BY id";
        app.query(sql, [], |r| r.get::<_, String>(0))
    };
    let said = change(json!({"id": "r1", "prompt": "water them all"}));
    assert!(said.is_empty(), "other changes say nothing: {said:?}");
    let said = change(json!({"id": "r1", "at": "18:45"}));
    let last = said.last().cloned().unwrap_or_default();
    assert!(
        last.contains("Regar") && last.contains("Mon to Fri 18:45") && last.contains("next time: "),
        "a new time: {said:?}"
    );
    let said = change(json!({"id": "r1", "days": [6]}));
    let last = said.last().cloned().unwrap_or_default();
    assert!(
        last.contains("Sat 18:45") && last.contains("next time: Sat "),
        "new days: {said:?}"
    );
    let said = change(json!({"id": "r1", "on": false}));
    assert!(
        said.len() == 3 && said[2].contains("turned off"),
        "switched off has no next run: {said:?}"
    );
    let said = change(json!({"id": "r1", "on": true}));
    assert!(
        said.len() == 4 && said[3].contains("next time: Sat "),
        "switched on again: {said:?}"
    );
    let _ = std::fs::remove_dir_all(&home);
}
