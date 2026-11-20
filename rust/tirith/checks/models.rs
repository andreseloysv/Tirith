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
        assert!(a.iter().any(|x| x == super::NO_SANDBOX));
        assert!(!a.iter().any(|x| x == "--sandbox"));
    }
}
