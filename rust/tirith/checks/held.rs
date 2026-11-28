//! A routine's run sends the phones only its final report; its working notes stay in the bot's chat.
use serde_json::json;

fn said(app: &crate::app::Shared, project: &str, texts: &[&str]) {
    super::talk(project, |t| (t.phones, t.in_run) = (true, true));
    for text in texts {
        let msg = json!({"message": {"content": [{"type": "text", "text": text}]}});
        super::blocks::assistant(app, project, &msg);
    }
}

#[test]
fn a_held_run_sends_only_its_final_report() {
    let home = std::env::temp_dir().join(format!("tirith-held-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("config.toml"), "").unwrap();
    let app = crate::app::App::open_at(home.clone());
    let notes = ["Miro el correo.", "Abro tres hilos.", "Leo el segundo."];
    let report = "Informe: dos correos nuevos de Ana.";
    said(&app, "held-test", &[notes[0], notes[1], notes[2], report]);
    assert_eq!(super::run_over("held-test", true).as_deref(), Some(report));
    let chat: Vec<String> = app.query(
        "SELECT text FROM chat WHERE role = 'manager' ORDER BY id",
        [],
        |r| r.get(0),
    );
    assert_eq!(
        chat,
        [notes[0], notes[1], notes[2], report],
        "the notes stay in the chat"
    );
    said(&app, "held-test", &[notes[0], "∅"]);
    assert_eq!(
        super::run_over("held-test", true),
        None,
        "nothing new stays quiet"
    );
    said(&app, "held-test", &[notes[0], report]);
    assert_eq!(
        super::run_over("held-test", false),
        None,
        "a failed run is its one line"
    );
    assert_eq!(
        super::run_over("held-test", true),
        None,
        "what was held is gone"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
