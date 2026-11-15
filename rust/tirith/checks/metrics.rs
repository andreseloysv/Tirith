#[test]
fn ceo_read_only_allows_reads() {
    for cmd in ["tirith status", "tirith plan show tirith", "tirith lane log some-lane", "tirith timeout get", "tirith merge status"] {
        let input = serde_json::json!({"command": cmd});
        assert_eq!(super::ceo_permission("Bash", &input)["behavior"], "allow", "{cmd}");
    }
    assert_eq!(super::ceo_permission("Read", &serde_json::json!({"file_path": "/x"}))["behavior"], "allow");
    ["Write", "Edit", "MultiEdit", "NotebookEdit"].into_iter().for_each(|t| assert_eq!(super::ceo_permission(t, &serde_json::json!({}))["behavior"], "deny", "{t}"));
}
#[test]
fn health_reports_paused_reason() {
    let t = super::paused_text("tirith", "commit guard: the pre-commit hook is outdated or has other words than config.toml (tirith guard install tirith)");
    assert!(t.contains("paused: commit guard"), "{t}");
    assert!(t.contains("fix: tirith guard install tirith"), "{t}");
    assert!(super::paused_text("p", "3 lanes wait for the merge queue").contains("fix: wait for the merges"));
    assert!(super::paused_text("p", "less than 5 GB free on /x").contains("fix: free disk space"));
}
#[test]
fn health_merged_reports_empty_plans() {
    let t = super::empty_plan_note(&["tirith".to_string(), "p2".to_string()]);
    assert!(t.contains("tirith plan empty") && t.contains("fix: tirith plan add tirith <step>"), "{t}");
    assert!(t.contains("p2 plan empty"), "{t}");
    assert!(super::empty_plan_note(&[]).is_empty());
}
