#[test]
fn muse_manager_turn_is_an_exec_run() {
    let a = super::muse_argv("muse", "manager", Some("m"), Some("high"), "/w", Some("s-1"), "/w/p.txt");
    let has = |x: &str| a.iter().any(|y| y == x);
    assert!(a[1] == "exec" && has("--json") && has("--yolo") && has("--prompt-file"), "{a:?}");
    assert!(has("--model") && has("--reasoning-effort") && has("--session-id"), "{a:?}");
    // The failure this exists for: the other agent's chat flags must never reach this binary.
    assert!(!has("-p") && !has("--input-format") && !has("--output-format") && !has("--append-system-prompt"), "{a:?}");
    assert!(!has("--disable-write"), "{a:?}");
}

#[test]
fn muse_ceo_turn_is_read_only_and_fresh_without_a_session() {
    let a = super::muse_argv("muse", "ceo", None, None, "/w", None, "/w/p.txt");
    let has = |x: &str| a.iter().any(|y| y == x);
    assert!(has("--disable-write"), "{a:?}");
    assert!(!has("--session-id") && !has("--model") && !has("--reasoning-effort"), "{a:?}");
}
