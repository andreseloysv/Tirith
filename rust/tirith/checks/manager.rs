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

#[test]
fn judged_reads_a_label_or_a_score_and_one_line_why() {
    assert_eq!(super::judged("approval — el dueño da las gracias por el arreglo"), ("approval".to_string(), None, "el dueño da las gracias por el arreglo".to_string()));
    assert_eq!(super::judged("Correction: pide rehacer la portada").0, "correction");
    assert_eq!(super::judged("`noise` — se queja de los avisos").0, "noise");
    assert_eq!(super::judged("\n\n0.25 — mostly not needed\n"), (String::new(), Some(0.25), "mostly not needed".to_string()));
    assert_eq!(super::judged("alone — era un valor por defecto reversible").0, "alone");
    assert_eq!(super::judged("").0, "");
}

#[test]
fn card_id_of_reads_the_card_an_event_line_names() {
    assert_eq!(super::card_id_of("Card o12 closed by the manager: ya no hace falta").as_deref(), Some("o12"));
    assert_eq!(super::card_id_of("The owner answered o3 (decision: ¿sigo?): sí").as_deref(), Some("o3"));
    assert_eq!(super::card_id_of("o7 (payment): ¿pago 3 $?").as_deref(), Some("o7"));
    assert_eq!(super::card_id_of("merged lane lo-que-sea into main"), None);
    assert_eq!(super::card_id_of("owner without a card number"), None);
}
