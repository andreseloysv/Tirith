use chrono::{Local, TimeZone, Utc};
use rusqlite::params;

/// A local time of Sunday 2026-11-22 as the rows keep it.
fn at(hour: u32, minute: u32) -> String {
    Local
        .with_ymd_and_hms(2026, 11, 22, hour, minute, 0)
        .unwrap()
        .with_timezone(&Utc)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

#[test]
fn status_report_has_the_four_facts() {
    let home = std::env::temp_dir().join(format!("tirith-status-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let route = |id: &str, at: &str, day: u8, more: &str| {
        format!("[[routes]]\non = \"schedule\"\nto = \"bot-st\"\nmode = \"turn\"\nid = \"{id}\"\ntitle = \"{id}\"\ntask = \"do it\"\nat = \"{at}\"\ndays = [{day}]\n{more}\n")
    };
    let routes = [
        route("mail", "07:00", 7, ""),
        route("together", "07:00", 7, ""),
        route("broken", "08:00", 7, ""),
        route("going", "09:00", 7, ""),
        route("edited", "09:30", 7, ""),
        route("early", "18:00", 7, ""),
        route("monday", "10:00", 1, ""),
        route("off", "11:00", 7, "enabled = false\n"),
    ];
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[roles.bot-st]\nextends = \"manager\"\nscope = \"own\"\nname = \"Helper\"\n\n{}",
            routes.concat()
        ),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    let now = Local.with_ymd_and_hms(2026, 11, 22, 12, 0, 0).unwrap();
    let empty = super::routines::routines(&app, now);
    assert!(empty.contains("0 done, 6 pending"), "{empty}");
    assert!(super::error::last_error(&app, now).ends_with("none"));
    assert_eq!(super::lanes::lanes(&app), "Active lanes: none");

    let event = |when: &str, project: &str, kind: &str, tool: &str, text: &str| {
        app.insert("INSERT INTO events (at, lane, project, kind, tool, summary) VALUES (?1, '', ?2, ?3, ?4, ?5)", params![when, project, kind, tool, text])
    };
    let fired = [
        ("mail", at(7, 0)),
        ("together", at(7, 0)),
        ("broken", at(8, 0)),
        ("going", at(9, 0)),
        ("early", at(6, 0)),
    ];
    for (id, when) in &fired {
        event(
            when,
            "bot-st",
            "schedule",
            "",
            &format!("Routine «{id}» is due now."),
        );
        app.set_kv(&format!("routine:st:{id}"), Some(when));
    }
    // A change stamps `last` with no run: that routine neither ran nor runs today.
    app.set_kv("routine:st:edited", Some(&at(10, 0)));
    // Routines due together are each a turn of their own.
    for (routine, when, status) in [
        ("mail", at(7, 1), "ok"),
        ("together", at(7, 2), "ok"),
        ("broken", at(8, 1), "failed"),
    ] {
        app.insert("INSERT INTO turns (conv, role, \"trigger\", routine, started_at, ended_at, status, answer) VALUES ('bot-st', 'bot-st', 'schedule', ?1, ?2, ?2, ?3, 'no credits')", params![routine, when, status]);
    }
    for (id, project, status) in [
        ("a", "web", "running"),
        ("b", "web", "queued"),
        ("c", "web", "running"),
        ("d", "game", "waiting"),
        ("e", "game", "merged"),
    ] {
        let lane = crate::app::Lane {
            id: id.into(),
            project: project.into(),
            status: status.into(),
            ..Default::default()
        };
        app.lanes.lock().unwrap().insert(id.into(), lane);
    }
    let text = super::command::report(&app, now);
    assert!(text.starts_with("Up for "), "{text}");
    for part in [
        "- game: 1 (1 waiting)\n- web: 3 (1 queued, 2 running)",
        "Routines today: 2 done, 2 pending, 1 failed",
        "07:00 Helper · mail: done",
        "07:00 Helper · together: done",
        "08:00 Helper · broken: failed",
        "09:00 Helper · going: started",
        "18:00 Helper · early: pending",
        "Last error (3 h 59 min ago) · Helper · turn failed\nno credits",
    ] {
        assert!(text.contains(part), "{part} is missing:\n{text}");
    }
    for absent in ["edited", "monday", "· off"] {
        assert!(!text.contains(absent), "{absent}:\n{text}");
    }

    // A newer error event wins over the failed turn; it keeps both ends and hides secrets.
    let long = format!(
        "push refused API_KEY=hunter2 {} no disk left",
        "ñ ".repeat(300)
    );
    event(&at(11, 55), "web", "error", "push", &long);
    let error = super::error::last_error(&app, now);
    assert!(
        error.contains("(5 min ago) · web · push\npush refused"),
        "{error}"
    );
    assert!(
        error.ends_with("no disk left") && error.chars().count() < 300,
        "{error}"
    );
    assert!(!error.contains("hunter2"), "{error}");
    assert_eq!(super::span::span(59), "less than 1 min");
    assert_eq!(super::span::span(2 * 86400 + 3 * 3600 + 120), "2 d 3 h");
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
