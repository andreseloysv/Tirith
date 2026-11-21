#[test]
fn muse_chat_turn_is_an_exec_run() {
    let a = super::muse_argv("muse", false, (Some("m"), Some("high")), "/w", Some("s-1"), "/w/p.txt", &[]);
    let has = |x: &str| a.iter().any(|y| y == x);
    assert!(a[1] == "exec" && has("--json") && has("--yolo") && has("--prompt-file"), "{a:?}");
    assert!(has("--model") && has("--reasoning-effort") && has("--session-id"), "{a:?}");
    // The failure this exists for: the other agent's chat flags must never reach this binary.
    assert!(!has("-p") && !has("--input-format") && !has("--output-format") && !has("--append-system-prompt"), "{a:?}");
    assert!(!has("--disable-write"), "{a:?}");
}

/// An app at a fresh home whose config puts Muse in place, with this `[browser] use` ("" = the section is absent, Tirith's own browser).
fn browser_home(tag: &str, use_mine: bool) -> crate::app::Shared {
    let dir = std::env::temp_dir().join(format!("tirith-browser-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let browser = if use_mine { "[browser]\nuse = \"mine\"\n" } else { "" };
    std::fs::write(dir.join("config.toml"), format!("# the owner's\n[agents.muse]\ncommand = \"muse\"\n{browser}[[projects]]\nid = \"p1\"\nrepo = \"/tmp\"\n")).unwrap();
    let app = crate::app::App::open_at(dir);
    assert_eq!(app.cfg().error, None, "config with [browser] must not read as unknown keys");
    app
}

/// The `tirith_browser` server of a Muse argv: its JSON after `-c mcpServers.tirith_browser=`.
fn browser_of(argv: &[String]) -> serde_json::Value {
    let at = argv.iter().position(|a| a == "-c").unwrap_or_else(|| panic!("no -c in {argv:?}"));
    let json = argv[at + 1].strip_prefix("mcpServers.tirith_browser=").unwrap_or_else(|| panic!("not the browser server: {:?}", argv[at + 1]));
    serde_json::from_str(json).unwrap()
}

#[test]
fn muse_runs_get_the_browser_mcp() {
    // Default (use = "tirith"): a lane's argv carries the launcher with --isolated; a bot's turn argv carries it bare (the launcher then opens the persistent profile).
    let app = browser_home("tirith", false);
    let lane = crate::models::lane_argv(&app, Some("muse"), None, std::path::Path::new("/tmp/brief.md"), std::path::Path::new("/tmp"), None).unwrap();
    let server = browser_of(&lane.argv);
    assert!(server["command"].as_str().is_some_and(|c| c.ends_with("node")), "node runs it: {server}");
    assert!(server["args"][0].as_str().is_some_and(|a| a.ends_with("tools/browser/launch.mjs")), "the repo's launcher: {server}");
    assert_eq!(server["args"][1], "--isolated", "a lane never shares or keeps a profile: {server}");
    assert_eq!(server["env"]["TIRITH_HOME"], "${TIRITH_HOME}", "the whole-value variable both spawns set (Muse refuses a literal env value): {server}");
    let bot = super::muse_argv("muse", false, (None, None), "/w", None, "/w/p.txt", &crate::models::browser_args(&app, false));
    let server = browser_of(&bot);
    assert_eq!(server["args"].as_array().map(|a| a.len()), Some(1), "a bot gets the persistent profile, no mode flag: {server}");
    // While a live session holds that profile (its SingletonLock names this very process), a bot falls back to --isolated instead of failing.
    let profile = app.home.join("browser/profile");
    std::fs::create_dir_all(&profile).unwrap();
    std::os::unix::fs::symlink(format!("somehost-{}", std::process::id()), profile.join("SingletonLock")).unwrap();
    let server = browser_of(&crate::models::browser_args(&app, false));
    assert_eq!(server["args"][1], "--isolated", "a held profile means isolated, not an error: {server}");
    // use = "mine": a bot connects to the owner's Edge (--extension); a lane still never touches the owner's browser.
    let app = browser_home("mine", true);
    let bot = super::muse_argv("muse", false, (None, None), "/w", None, "/w/p.txt", &crate::models::browser_args(&app, false));
    let server = browser_of(&bot);
    assert_eq!(server["args"][1], "--extension", "a bot with use = \"mine\" drives the owner's Edge: {server}");
    let lane = crate::models::lane_argv(&app, Some("muse"), None, std::path::Path::new("/tmp/brief.md"), std::path::Path::new("/tmp"), None).unwrap();
    let server = browser_of(&lane.argv);
    assert_eq!(server["args"][1], "--isolated", "a lane is isolated even with use = \"mine\": {server}");
    assert!(!lane.argv.iter().any(|a| a.contains("--extension")), "no lane ever gets the owner's browser: {:?}", lane.argv);
}

#[test]
fn muse_read_only_turn_is_read_only_and_fresh_without_a_session() {
    let a = super::muse_argv("muse", true, (None, None), "/w", None, "/w/p.txt", &[]);
    let has = |x: &str| a.iter().any(|y| y == x);
    assert!(has("--disable-write"), "{a:?}");
    assert!(!has("--session-id") && !has("--model") && !has("--reasoning-effort"), "{a:?}");
}
