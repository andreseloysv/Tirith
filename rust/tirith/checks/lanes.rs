#[test]
fn brief_warns_against_blind_waits() {
    let text = crate::factory::current().prompt("brief");
    assert!(text.contains("Never wait blindly") && text.contains("two minutes") && text.contains("find out why"));
}

#[test]
fn codex_run_completed_ends_lane() {
    // The shape of the real incident (forest-art-second-pass-docs-art-9728): its record clocks are not wall time, so only the order in the log counts.
    let ev = |kind: &str, at: i64| format!("{{\"schema_version\":1,\"recorded_at\":{at},\"payload_type\":\"{kind}\",\"payload\":{{\"kind\":\"run_terminal\",\"terminal\":\"completed\",\"text\":\"done\"}}}}");
    let (done, started, launch) = (ev("run.terminal.completed", 1780531400005239), ev("run.lifecycle.started", 1780531400000001), "--- tirith: lane run started 2026-11-14T12:00:00Z");
    assert!(!super::run_completed("run.terminal.completed\n")); // a plain mention is not the event
    let nested = "{\"payload_type\":\"tool.result\",\"payload\":{\"text\":\"\\\"payload_type\\\":\\\"run.terminal.completed\\\"\"}}";
    assert!(!super::run_completed(&format!("{launch}\n{started}\n{nested}\n"))); // nor one inside output the agent read
    assert!(super::run_completed(&done)); // the start of a long run fell out of the tail window
    assert!(super::run_completed(&format!("{launch}\n{started}\n{done}\n")));
    assert!(!super::run_completed(&format!("{done}\n{launch}\n{started}\n"))); // an earlier attempt's terminal, left in the appended log
    assert!(!super::run_completed(&format!("{done}\n{started}\n"))); // a later run is under way (a launch line may predate this code)
}

#[test]
fn idle_lane_after_api_retry_is_stopped() {
    let mut seen = std::collections::HashMap::new();
    let (real, retry) = ("{\"type\":\"assistant\"}\n", "{\"subtype\":\"api_retry\"}\n");
    assert_eq!(super::quiet_s(&mut seen, "a", real, 0, 1000), 0);
    assert_eq!(super::quiet_s(&mut seen, "a", &format!("{real}{retry}{retry}"), 0, 2200), 1200);
    assert_eq!(super::quiet_s(&mut seen, "a", &format!("{real}{retry}x\n"), 0, 2201), 0);
}

#[test]
fn long_running_lane_is_reported_once() {
    let start = "2026-11-13T10:00:00Z";
    let t0 = chrono::DateTime::parse_from_rfc3339(start).unwrap().timestamp() as u64;
    assert!(!super::long_running_due(Some(start), t0 + 29 * 60, 30, false));
    assert!(super::long_running_due(Some(start), t0 + 30 * 60, 30, false));
    assert!(!super::long_running_due(Some(start), t0 + 40 * 60, 30, true)); // once
    assert!(!super::long_running_due(Some(start), t0 + 40 * 60, 0, false)); // off
    assert!(!super::long_running_due(None, t0, 30, false));
    let log = "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"input\":{\"command\":\"sleep 590\"}}]}}\nnot json\n{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"hi\"},{\"type\":\"tool_use\",\"input\":{\"command\":\"ls\"}}]}}\n";
    assert_eq!(super::last_commands(log, 1), ["ls"]);
    assert_eq!(super::last_commands(log, 5), ["sleep 590", "ls"]);
}
#[test]
fn lane_runs_clippy_before_finish() {
    let home = std::env::temp_dir().join(format!("tirith-clippy-home-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("TIRITH_HOME", &home);
    let app = crate::app::App::open();
    let dir = std::env::temp_dir().join(format!("tirith-clippy-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let lane = crate::app::Lane { workspace: dir.display().to_string(), ..Default::default() };
    assert!(!super::brief(&app, &lane).contains("cargo clippy -- -D warnings"));
    std::fs::write(dir.join("Cargo.toml"), "").unwrap();
    assert!(super::brief(&app, &lane).contains("cargo clippy -- -D warnings"));
}
#[test]
fn low_disk_asks_owner_once() {
    let asks = [(8.0, false), (4.0, false), (3.0, true), (5.5, true), (9.0, true)].map(|(gb, asked)| super::low_disk_ask(gb, asked));
    assert_eq!(asks, [None, Some(true), None, None, Some(false)]);
    let dir = std::env::temp_dir().join(format!("tirith-lowdisk-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("big")).unwrap();
    std::fs::write(dir.join("big/f"), vec![1u8; 300_000]).unwrap();
    let rows = super::biggest_dirs(&dir, 5);
    assert!(rows[0].0.ends_with("big"));
    std::fs::remove_dir_all(&dir).unwrap();
}
#[test]
fn diskdiet_clears_idle_build_cache() {
    let root = std::env::temp_dir().join(format!("tirith-builds-{}", std::process::id()));
    ["tirith/cargo", "zoo/cargo"].iter().for_each(|p| std::fs::create_dir_all(root.join(p)).unwrap());
    assert_eq!(super::clear_idle_builds(&root, &["zoo".into()]), [root.join("tirith")]);
    assert!(!root.join("tirith").exists() && root.join("zoo/cargo").is_dir());
    std::fs::remove_dir_all(&root).unwrap();
}
#[test]
fn lane_waits_for_payment_answer() {
    assert!(super::is_payment("¿Aprobar 0,74 $ de voces TTS de pago?") && !super::is_payment("Which name?"));
}
#[test]
fn autopilot_skips_owner_wait_items() {
    let skipped = ["Elegir dominio (espera: el dueño decide)", "CEO: revisar", "Manager de guardia: atender el chat"];
    assert!(skipped.iter().all(|t| super::is_autopilot_skipped(t)));
    assert!(!super::is_autopilot_skipped("Arreglar el parser") && !super::is_autopilot_skipped("Arreglar el CEO: bug"));
}
#[test]
fn autopilot_waits_for_dependency_items() {
    let open = ["Generar los vídeos de la demo en 3 idiomas", "Subir a YouTube"];
    let up = "Subir los vídeos de la demo a YouTube: wait until they are on main, made by the item «Generar los vídeos de la demo en 3 idiomas»";
    assert!(super::waits_for_open_item(up, &open));
    assert!(!super::waits_for_open_item(up, &["Subir a YouTube"]));
    assert!(!super::waits_for_open_item("Arreglar el parser", &open));
}
#[test]
fn stale_owner_questions_close() {
    use crate::manager::stale_reason;
    let off = "Should I turn off the autopilot of tirith?";
    assert_eq!(stale_reason(off, true, 50.0), None); // still on: the question applies
    assert!(stale_reason(off, false, 50.0).is_some()); // already off: stale
    assert!(stale_reason("¿Apagar el autopilot de superpolitika?", false, 50.0).is_some());
    assert!(stale_reason("Enable the autopilot again?", true, 50.0).is_some());
    assert_eq!(stale_reason("Enable the autopilot again?", false, 50.0), None);
    assert_eq!(stale_reason("Approve $0.74 for the translation?", false, 50.0), None);
}
#[test]
fn low_disk_question_closes_when_freed() {
    use crate::manager::stale_reason;
    let q = "Only 4.6 GB free on this disk: lanes will fail with No space left on device.";
    assert_eq!(stale_reason(q, true, 4.6), None); // still low
    assert_eq!(stale_reason(q, true, 9.0), None); // not above 10 GB yet
    assert!(stale_reason(q, true, 39.0).is_some());
}
