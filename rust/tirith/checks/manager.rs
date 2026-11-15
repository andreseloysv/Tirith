#[test]
fn restart_waits_for_a_running_turn() {
    super::talk("restart-test", |t| (t.busy, t.pid) = (true, 0));
    super::restart();
    assert!(super::talk("restart-test", |t| t.stale));
}

#[test]
fn lane_ask_shows_in_open_questions() {
    let lane = |id: &str, status: &str| crate::app::Lane { id: id.into(), status: status.into(), item: "Traducir".into(), question: "¿Aprobar 0,74 $?".into(), ..Default::default() };
    let cards = super::waiting_cards(&[lane("l1", "waiting"), lane("l2", "running")]);
    let q = cards[0]["question"].to_string();
    assert!(cards.len() == 1 && cards[0]["id"] == "l1" && q.contains("lane bloqueada esperando al dueño") && q.contains("0,74"), "{q}");
}
