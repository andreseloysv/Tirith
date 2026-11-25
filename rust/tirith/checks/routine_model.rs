use super::super::tuning;
use super::saved;
use serde_json::{json, Value};

/// A home with two agents, each with its models, and one bot (on the first agent, as every bot starts).
fn bot(name: &str) -> (crate::app::Shared, String, String) {
    let home = std::env::temp_dir().join(format!(
        "tirith-routine-model-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let config = "[network]\nsandbox = false\n[agents.anthropic]\ncommand = \"/bin/echo\"\n[agents.muse]\ncommand = \"/bin/echo\"\n[models.choices]\nanthropic = [\"fast-1\", \"deep-1\"]\nmuse = [\"other-1\"]\n";
    std::fs::write(home.join("config.toml"), config).unwrap();
    let app = crate::app::App::open_at(home);
    let card = json!({"name": "Saluda", "shape": "drop", "color": "#22d3ee"});
    let made = super::super::save(&app, card).unwrap();
    let id = made["id"].as_str().unwrap().to_string();
    (app, id.clone(), format!("bot-{id}"))
}

fn routine_of(app: &crate::app::Shared, id: &str) -> Value {
    let bots = super::super::all(app);
    let bot = bots.iter().find(|b| b["id"] == id).unwrap();
    bot["routines"][0].clone()
}

#[tokio::test]
async fn a_routine_keeps_a_model_and_an_effort_of_its_own() {
    let (app, id, key) = bot("own");
    let dir = super::folder(&app, &key);
    let card = json!({"title": "Saludo", "prompt": "saluda", "days": [1, 2], "at": "08:00", "model": "fast-1", "effort": "low"});
    let done = saved(&app, id.clone(), card).unwrap();
    let rid = done["id"].as_str().unwrap().to_string();
    let own = |effort: Option<&str>, model: Option<&str>| {
        Some((effort.map(String::from), model.map(String::from)))
    };
    assert_eq!(tuning::read(&dir, &rid), own(Some("low"), Some("fast-1")));
    let shown = routine_of(&app, &id);
    assert_eq!(
        (&shown["model"], &shown["effort"]),
        (&json!("fast-1"), &json!("low")),
        "its card shows them"
    );
    // A change that names neither leaves them; "" takes one back to the bot's own.
    saved(&app, id.clone(), json!({"id": rid, "at": "09:00"})).unwrap();
    assert_eq!(tuning::read(&dir, &rid), own(Some("low"), Some("fast-1")));
    saved(&app, id.clone(), json!({"id": rid, "model": ""})).unwrap();
    assert_eq!(tuning::read(&dir, &rid), own(Some("low"), None));
    assert_eq!(routine_of(&app, &id)["model"], "");
    // Its run starts with them: the bot's own setting gives way for that run alone.
    let (mut effort, mut model) = (Some("max".to_string()), Some("deep-1".to_string()));
    tuning::tune(&app, &key, &rid);
    tuning::tuned(&key, &mut effort, &mut model);
    assert_eq!(
        (effort.as_deref(), model.as_deref()),
        (Some("low"), Some("deep-1"))
    );
    tuning::untune(&app, &key);
    // Another agent's model, or what is no word, is refused before anything is saved: the time stays too.
    let other = saved(
        &app,
        id.clone(),
        json!({"id": rid, "at": "10:00", "model": "other-1"}),
    );
    assert!(
        other.unwrap_err().contains("is agent muse's"),
        "it names whose model it is"
    );
    assert!(saved(&app, id.clone(), json!({"id": rid, "effort": "very high!"})).is_err());
    assert_eq!(routine_of(&app, &id)["at"], "09:00");
    assert_eq!(tuning::read(&dir, &rid), own(Some("low"), None));
    // The bot's repository tracks the file, and a deleted routine leaves nothing of its own behind.
    let log = std::process::Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args(["log", "--format=%s", "--", "routines.toml"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&log.stdout).contains("routine"),
        "committed as the routine's change"
    );
    saved(&app, id.clone(), json!({"id": rid, "delete": true})).unwrap();
    assert_eq!(tuning::read(&dir, &rid), None);
    let _ = std::fs::remove_dir_all(&app.home);
}
