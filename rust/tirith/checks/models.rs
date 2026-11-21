#[test]
fn models_state_gives_face_and_show() {
    let dir = std::env::temp_dir().join(format!("tirith-models-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.toml"), "# the owner's\n").unwrap();
    let app = crate::app::App::open_at(dir);
    let v = super::state(&app);
    let manager = &v["roles"]["manager"];
    assert_eq!(manager["name"], "Project Manager");
    assert_eq!(manager["face"]["shape"], "hex");
    assert_eq!(manager["face"]["color"], "#8e7cf0");
    assert_eq!(v["roles"]["judge"]["show"], false);
    assert_eq!(v["roles"]["ceo"]["show"], true);
    assert_eq!(v["roles"]["manager-general"]["face"]["shape"], "dome"); // its own face, not the project manager's it extends
}

#[test]
fn muse_lane_is_an_exec_run() {
    let c = crate::config::Config::default();
    let (_, effort, resume, args) = super::flags(&c, "muse");
    assert!(args[0] == "exec" && args.iter().any(|x| x == "--prompt-file"), "{args:?}");
    assert!(!args.iter().chain(&resume).any(|x| x == "-p"), "{args:?} {resume:?}");
    assert_eq!(effort.as_deref(), Some("--reasoning-effort"));
}

#[test]
fn codex_lane_runs_commands() {
    let c = crate::config::Config::default();
    let (_, _, resume, args) = super::flags(&c, "codex");
    for a in [resume, args] {
        assert!(a.iter().any(|x| x == "--dangerously-bypass-approvals-and-sandbox"));
        assert!(!a.iter().any(|x| x == "--sandbox"));
    }
}

#[tokio::test]
async fn models_agent_and_model_match() {
    use serde_json::json;
    let dir = std::env::temp_dir().join(format!("tirith-models-match-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Which agent a model belongs to comes from the configuration: the grouped choices, or the prefix of each agent's command.
    std::fs::write(
        dir.join("config.toml"),
        "[agents.anthropic]\ncommand = \"/usr/local/bin/frontier\"\n\n[agents.muse]\ncommand = \"muse\"\n\n[models.choices]\nanthropic = [\"frontier-big-5\", \"sonnetish\"]\nmuse = [\"muse-spark-1.3\"]\n",
    )
    .unwrap();
    let app = crate::app::App::open_at(dir.clone());
    let cfg = app.cfg();
    assert_eq!(super::agent_of(&cfg, "sonnetish").as_deref(), Some("anthropic"), "its group in [models.choices]");
    assert_eq!(super::agent_of(&cfg, "frontier-other").as_deref(), Some("anthropic"), "unlisted, by its command's prefix");
    assert_eq!(super::agent_of(&cfg, "muse-wonder").as_deref(), Some("muse"));
    assert_eq!(super::agent_of(&cfg, "something-else"), None);
    // The crossed pair of 2026-11-21 is refused, by the API and the command line alike (both save through here), and nothing is saved.
    let err = super::save(&app, "worker", Some(&json!({"agent": "muse", "model": "frontier-big-5", "effort": "medium"}))).err().unwrap();
    assert!(err.contains("frontier-big-5") && err.contains("anthropic"), "{err}");
    assert!(app.kv("models:worker").is_none());
    assert!(super::save(&app, "worker", Some(&json!({"agent": "anthropic", "fallback": ["muse:frontier-big-5"]}))).is_err(), "a crossed fallback too");
    // Choosing a model of another agent switches the agent to the one that has it.
    let s = super::save(&app, "worker", Some(&json!({"agent": "muse"}))).unwrap();
    assert_eq!(s.agent, "muse");
    let s = super::save(&app, "worker", Some(&json!({"model": "frontier-big-5", "effort": "medium"}))).unwrap();
    assert_eq!((s.agent.as_str(), s.model.as_deref(), s.effort.as_deref()), ("anthropic", Some("frontier-big-5"), Some("medium")));
    // The good pair passes, and the page sees each model's agent.
    assert!(super::save(&app, "worker", Some(&json!({"agent": "muse", "model": "muse-spark-1.3"}))).is_ok());
    let st = super::state(&app);
    assert_eq!(st["choices"]["owners"]["frontier-big-5"], "anthropic");
    assert_eq!(st["choices"]["models"].as_array().map(Vec::len), Some(3));
    // A lane never starts on a crossed pair: it fails at once, with the reason.
    let lane = super::lane_argv(&app, Some("muse"), Some("frontier-big-5"), std::path::Path::new("/tmp/brief.md"), &dir, None);
    assert!(lane.is_err_and(|e| e.contains("anthropic")));
    // A crossed pair saved before this check is said at start in the general manager's chat.
    app.set_kv("models:judge", Some(&json!({"agent": "muse", "model": "frontier-big-5"}).to_string()));
    super::start(app.clone());
    let said = app.query("SELECT summary FROM events WHERE kind = 'factory.invalid'", [], |r| r.get::<_, String>(0));
    assert!(said.iter().any(|s| s.contains("judge") && s.contains("frontier-big-5")), "{said:?}");
    let chat = app.query("SELECT text FROM chat WHERE project = 'manager-general'", [], |r| r.get::<_, String>(0));
    assert!(chat.iter().any(|t| t.contains("judge")), "{chat:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
