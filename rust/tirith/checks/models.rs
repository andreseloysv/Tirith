#[test]
fn codex_lane_runs_commands() {
    let c = crate::config::Config::default();
    let (_, _, resume, args) = super::flags(&c, "codex");
    for a in [resume, args] {
        assert!(a.iter().any(|x| x == super::NO_SANDBOX));
        assert!(!a.iter().any(|x| x == "--sandbox"));
    }
}
