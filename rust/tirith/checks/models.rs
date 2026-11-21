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
