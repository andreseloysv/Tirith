//! One webhook event runs a bot's webhook routine once, as a turn of its owner, whether its session is `turn` or `chat`; never on another bot.
use serde_json::{json, Value};

/// A home with bots named in `cards` ((name, session)), each with one webhook routine on `when`: (app, keys).
fn home(name: &str, cards: &[(&str, &str, &str)]) -> (crate::app::Shared, Vec<String>) {
    let home = std::env::temp_dir().join(format!("tirith-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = crate::app::App::open_at(home);
    let keys = cards
        .iter()
        .map(|(who, session, when)| {
            let card = json!({"name": who, "shape": "drop", "color": "#22d3ee", "session": session, "access": "read", "routes": []});
            let id = crate::bots::save(&app, card).unwrap()["id"].as_str().unwrap().to_string();
            let key = format!("bot-{id}");
            // No agent ever starts here: what a bot is handed only waits.
            let none = json!({"agent": "hooktest-no-such-agent", "model": null, "effort": null, "fallback": []});
            app.set_kv(&format!("models:{key}"), Some(&none.to_string()));
            let routine = json!({"title": format!("Task of {who}"), "trigger": "webhook", "when": when, "prompt": format!("Do the task of {who}")});
            crate::bots::routine::routine(&app, id, routine).unwrap();
            key
        })
        .collect();
    (app, keys)
}

/// A GitLab pipeline event whose status is `status`, as the room brings it.
fn pipeline(status: &str) -> String {
    let body = json!({"object_kind": "pipeline", "object_attributes": {"id": 5, "status": status, "ref": "main"}});
    json!({"v": 1, "at": "t", "headers": {"user-agent": "GitLab/16", "x-gitlab-event": "Pipeline Hook"}, "body": body}).to_string()
}

/// (conv, routine title, state, why) of every run written down.
fn runs(app: &crate::app::Shared) -> Vec<(String, String, String, String)> {
    app.query(
        "SELECT conv, title, state, COALESCE(why, '') FROM expected_runs ORDER BY id",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
}

/// What the roles in `keys` ran as a pass of their own (a review) because of an event.
async fn passes(app: &crate::app::Shared) -> Vec<(String, String)> {
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    app.query("SELECT role, text FROM ceo_reviews", [], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })
}

async fn one_bot(session: &str) {
    let (app, keys) = home(
        &format!("hookone{session}"),
        &[("Dev", session, "gitlab:pipeline.success")],
    );
    crate::bots::webhook(&app, &keys[0], &pipeline("success"));
    let all = runs(&app);
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!(
        (all[0].0.as_str(), all[0].1.as_str()),
        (keys[0].as_str(), "Task of Dev")
    );
    assert!(!all[0].3.contains("nobody can do it"), "{all:?}");
    assert_eq!(
        passes(&app).await,
        Vec::<(String, String)>::new(),
        "no role ran a pass for the event"
    );
    let _ = std::fs::remove_dir_all(&app.home);
}

#[tokio::test]
async fn a_turn_bots_webhook_routine_runs_once_on_it() {
    one_bot("turn").await;
}
#[tokio::test]
async fn a_chat_bots_webhook_routine_still_runs_once_on_it() {
    one_bot("chat").await;
}

#[tokio::test]
async fn an_event_matching_two_bots_runs_each_on_its_own_bot() {
    let both = [
        ("Dev", "turn", "gitlab:pipeline.success"),
        ("Review", "turn", "gitlab:pipeline.success"),
        ("Talk", "chat", "gitlab:pipeline.success"),
    ];
    let (app, keys) = home("hooktwo", &both);
    // The event reaches the first bot's room: only its routine runs; the others' wait for their own rooms.
    crate::bots::webhook(&app, &keys[0], &pipeline("success"));
    crate::bots::webhook(&app, &keys[1], &pipeline("success"));
    let all: Vec<Value> = runs(&app).iter().map(|r| json!([r.0, r.1, r.3])).collect();
    assert_eq!(all.len(), 2, "{all:?}");
    assert_eq!(all[0], json!([keys[0], "Task of Dev", ""]), "{all:?}");
    assert_eq!(all[1], json!([keys[1], "Task of Review", ""]), "{all:?}");
    assert_eq!(
        passes(&app).await,
        Vec::<(String, String)>::new(),
        "a routine is never run by another role"
    );
    let _ = std::fs::remove_dir_all(&app.home);
}
#[tokio::test]
async fn a_turn_bots_scheduled_routine_runs_once_on_it() {
    let (app, keys) = home("hooksched", &[("Dev", "turn", "x:y")]);
    let id = keys[0].strip_prefix("bot-").unwrap().to_string();
    let rid = crate::bots::bot(&app, &keys[0]).unwrap()["routines"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    crate::bots::run::fire(&app, &id, &rid, "run-now", "").unwrap();
    let all = runs(&app);
    assert_eq!(
        (all.len(), all[0].0.as_str(), all[0].3.as_str()),
        (1, keys[0].as_str(), ""),
        "{all:?}"
    );
    assert!(passes(&app).await.is_empty());
    let _ = std::fs::remove_dir_all(&app.home);
}
