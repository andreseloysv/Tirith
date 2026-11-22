// `/status` and the register tell one truth.
use chrono::{Local, TimeZone, Utc};

#[test]
fn a_lost_run_reads_failed_in_status() {
    let home = std::env::temp_dir().join(format!("tirith-status-runs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        "[roles.bot-perdida]\nextends = \"manager\"\nscope = \"own\"\nname = \"Riega\"\n\n[[routes]]\non = \"schedule\"\nto = \"bot-perdida\"\nmode = \"turn\"\nid = \"r1\"\ntitle = \"Regar\"\ntask = \"water\"\nat = \"08:00\"\ndays = [7]\n",
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    let time = |day, hour| Local.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap();
    let kept = |t: chrono::DateTime<Local>| {
        t.with_timezone(&Utc)
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string()
    };
    app.set_kv("routine:perdida:r1", Some(&kept(time(15, 9))));
    let now = time(22, 12);
    let before = super::routines::routines(&app, now);
    assert!(before.contains("Regar: pending"), "{before}");

    let due = kept(time(22, 8));
    let new = crate::expected::New {
        conv: "bot-perdida",
        routine: "r1",
        title: "Regar",
        trigger: "schedule",
        kind: "schedule",
        due: &due,
        task: "",
    };
    crate::expected::expect(&app, &new);
    crate::expected::started(&app, "bot-perdida", "r1");
    crate::expected::orphaned(&app, "bot-perdida"); // it left no turn of its own, and no event says it was due
    let after = super::routines::routines(&app, now);
    assert!(
        after.contains("Regar: failed") && after.contains("1 failed"),
        "{after}"
    );
    app.set_kv("routine:perdida:r1", Some(&kept(time(22, 11)))); // a change moves its stamp
    let changed = super::routines::routines(&app, now);
    assert!(changed.contains("Regar: failed"), "{changed}");
    let _ = std::fs::remove_dir_all(&home);
}
