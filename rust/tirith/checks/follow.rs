// The streaming CLI folds a line that comes during a turn into it (one result for both) or gives it a turn of its
// own right after (a system init, about 10 ms later): both measured against the real CLI on 2026-11-22.
use super::super::live::{end, handed, idle, line};
use super::{follow, settle};
use serde_json::json;

fn app(name: &str) -> crate::app::Shared {
    let home = std::env::temp_dir().join(format!("tirith-follow-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("config.toml"), "[network]\nsandbox = false\n").unwrap();
    crate::app::App::open_at(home)
}

fn triggers(app: &crate::app::Shared, conv: &str) -> Vec<String> {
    app.query(
        "SELECT \"trigger\" FROM turns WHERE conv = ?1 ORDER BY id",
        [conv],
        |r| r.get(0),
    )
}

#[test]
fn a_message_folded_into_the_running_turn_leaves_the_conversation_free() {
    let (app, conv) = (app("folded"), "follow-folded");
    handed(&app, conv, ("schedule".into(), "r1".into()));
    handed(&app, conv, ("owner".into(), String::new())); // during that turn
    end(&app, conv, Some(true), Some("both answered"));
    assert!(
        !idle(conv),
        "for a moment it may still get a turn of its own"
    );
    settle(&app, conv, 1);
    assert!(
        idle(conv),
        "it joined that turn: a routine waiting behind it may run"
    );
    assert_eq!(triggers(&app, conv), ["schedule"]);
    // The next message is a turn of its own, under its own trigger.
    handed(&app, conv, ("room".into(), String::new()));
    end(&app, conv, Some(true), Some("next"));
    assert_eq!(triggers(&app, conv), ["schedule", "room"]);
    assert!(idle(conv));
}

#[test]
fn a_message_with_a_turn_of_its_own_keeps_its_trigger() {
    let (app, conv) = (app("own"), "follow-own");
    handed(&app, conv, ("schedule".into(), "r1".into()));
    handed(&app, conv, ("owner".into(), String::new()));
    end(&app, conv, Some(true), Some("one"));
    let init = json!({"type": "system", "subtype": "init", "session_id": "s"});
    follow(&app, conv, &init);
    line(conv, &init);
    settle(&app, conv, 1); // too late to fold anything: a turn is open
    end(&app, conv, Some(true), Some("two"));
    assert_eq!(triggers(&app, conv), ["schedule", "owner"]);
    assert!(idle(conv));
}
