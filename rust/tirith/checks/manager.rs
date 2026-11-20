#[test]
fn restart_waits_for_a_running_turn() {
    super::talk("restart-test", |t| (t.busy, t.pid) = (true, 0));
    super::restart();
    assert!(super::talk("restart-test", |t| t.stale));
}

#[test]
fn turn_read_only_allows_reads() {
    for cmd in ["tirith status", "tirith plan show tirith", "tirith lane log some-lane", "tirith timeout get", "tirith merge status"] {
        let input = serde_json::json!({"command": cmd});
        assert_eq!(super::turn_permission(true, "Bash", &input)["behavior"], "allow", "{cmd}");
    }
    assert_eq!(super::turn_permission(true, "Read", &serde_json::json!({"file_path": "/x"}))["behavior"], "allow");
    ["Write", "Edit", "MultiEdit", "NotebookEdit"].into_iter().for_each(|t| {
        assert_eq!(super::turn_permission(true, t, &serde_json::json!({}))["behavior"], "deny", "{t}");
        assert_eq!(super::turn_permission(false, t, &serde_json::json!({}))["behavior"], "allow", "{t} without access=read");
    });
}

#[test]
fn lane_ask_shows_in_open_questions() {
    let lane = |id: &str, status: &str| crate::app::Lane { id: id.into(), status: status.into(), item: "Traducir".into(), question: "¿Aprobar 0,74 $?".into(), ..Default::default() };
    let cards = super::waiting_cards(&[lane("l1", "waiting"), lane("l2", "running")]);
    let q = cards[0]["question"].to_string();
    assert!(cards.len() == 1 && cards[0]["id"] == "l1" && q.contains("lane bloqueada esperando al dueño") && q.contains("0,74"), "{q}");
}
