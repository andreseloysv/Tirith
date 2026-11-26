// Server mode fails closed; what only a Mac has says so on a Linux server.
use super::{mac_only, refusal, server_mode};
use crate::app::Shared;

fn app(name: &str, config: &str) -> Shared {
    let home = std::env::temp_dir().join(format!("tirith-closed-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("config.toml"), config).unwrap();
    crate::app::App::open_at(home)
}

#[test]
fn server_mode_fails_closed() {
    let off = "[server]\nmode = \"server\"\n[network]\nsandbox = false\n";
    let shut = app("off", off);
    assert!(server_mode(&shut));
    let why = refusal(&shut, "demo-1-lane").expect("no sandbox, no lane");
    assert!(
        why.contains("demo-1-lane does not start") && why.contains("sandbox = false"),
        "{why}"
    );
    let err = crate::sandbox::wrap(&shut, "demo-1-lane", vec!["true".into()]).err();
    assert_eq!(
        err.as_deref(),
        Some(why.as_str()),
        "wrap starts nothing, and says why"
    );
    let events = shut.query(
        "SELECT summary FROM events WHERE kind = 'net-off'",
        [],
        |r| r.get::<_, String>(0),
    );
    assert_eq!(events, [why], "and leaves an event");
    // the same home is no server without the mode: it runs unconfined with an event, as before
    let desk = app("desk", "[network]\nsandbox = false\n");
    assert!(!server_mode(&desk) && refusal(&desk, "x").is_none());
    assert!(crate::sandbox::wrap(&desk, "demo-1-lane", vec!["true".into()]).is_ok());
}

#[test]
fn mac_only_tools_say_so() {
    assert!(mac_only("Safari", true).is_ok());
    let said = mac_only("Safari", false).unwrap_err();
    assert_eq!(said, "this needs the Mac: Safari is not on a Linux server");
}
