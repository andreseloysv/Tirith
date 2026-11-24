// `catch_up` and the register agree: an occurrence is caught up or it is lost, never both and never neither.
use super::calendar;
use chrono::{DateTime, Local, NaiveDate, TimeDelta};
use serde_json::json;

fn at(day: u32, hhmm: &str) -> DateTime<Local> {
    calendar::on(NaiveDate::from_ymd_opt(2026, 11, day).unwrap(), hhmm).unwrap()
}

fn shown(kept: &str) -> String {
    calendar::local(kept).map_or(kept.into(), |t| t.format("%d %H:%M").to_string())
}

/// What was written down for `conv`: each occurrence, and whether it was handed as a run.
fn occurrences(app: &crate::app::Shared, conv: &str) -> Vec<(String, bool)> {
    let sql = "SELECT due, COALESCE(task, '') <> '' FROM expected_runs WHERE conv = ?1 AND \"trigger\" = 'schedule' ORDER BY due, id";
    let rows = app.query(sql, [conv], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?))
    });
    rows.into_iter()
        .map(|(due, ran)| (shown(&due), ran))
        .collect()
}

/// The lines that say something was not done (not a run's own that could not start).
fn lost_lines(app: &crate::app::Shared, conv: &str) -> Vec<String> {
    let sql = "SELECT text FROM chat WHERE project = ?1 AND role = 'tirith' ORDER BY id";
    let mut said = app.query(sql, [conv], |r| r.get::<_, String>(0));
    said.retain(|t| t.contains("no se hizo") && !t.contains("no arrancó"));
    said
}

fn watched(
    app: &crate::app::Shared,
    (conv, routine, time): (&str, &str, &str),
    since: DateTime<Local>,
) {
    let sig = json!([time, [1, 2, 3, 4, 5, 6, 7], true]).to_string();
    let key = format!("expected:watch:{conv}:{routine}:{time}");
    app.set_kv(
        &key,
        Some(&json!({"sig": sig, "since": calendar::utc(since)}).to_string()),
    );
    app.set_kv("expected:alive", Some(&calendar::utc(since)));
}

#[tokio::test(flavor = "multi_thread")]
async fn stopped_over_one_and_three_days_is_one_row_for_each_time() {
    let mut wrong = vec![];
    for (away, skip, back) in [
        (1u32, false, "10:15"),
        (1, false, "06:00"),
        (1, true, "10:15"),
        (1, true, "06:00"),
        (3, false, "10:15"),
        (3, false, "06:00"),
        (3, true, "10:15"),
        (3, true, "06:00"),
    ] {
        let name = format!(
            "catchup{away}{}{}",
            if skip { "skip" } else { "once" },
            &back[..2]
        );
        let home = std::env::temp_dir().join(format!("tirith-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let key = format!("bot-{name}");
        let catch_up = if skip { "catch_up = \"skip\"\n" } else { "" };
        std::fs::write(
            home.join("config.toml"),
            format!("[agents.fake]\ncommand = \"/nonexistent/agent\"\nchat_args = []\n[network]\nsandbox = false\n\n[roles.{key}]\nextends = \"manager\"\nscope = \"own\"\nsession = \"chat\"\nname = \"Riega\"\n\n[[routes]]\non = \"schedule\"\nto = \"{key}\"\nmode = \"turn\"\nid = \"r1\"\ntitle = \"Regar\"\ntask = \"water\"\ndays = [1, 2, 3, 4, 5, 6, 7]\nat = \"08:00\"\n{catch_up}"),
        )
        .unwrap();
        let app = crate::app::App::open_at(home.clone());
        app.set_kv(&format!("models:{key}"), Some(r#"{"agent":"fake"}"#));

        // It last ran the day before it stopped, which was shortly before a time of its own.
        let first = 24 - away;
        let ran = at(first - 1, "08:00") + TimeDelta::seconds(20);
        app.set_kv(&format!("routine:{name}:r1"), Some(&calendar::utc(ran)));
        watched(&app, (&key, "r1", "08:00"), at(first, "07:30"));
        let now = at(24, back);
        crate::bots::tick_at(&app, now);
        crate::bots::tick_at(&app, now + TimeDelta::seconds(30));

        let come: Vec<u32> = if back < "08:00" {
            (first..24).collect()
        } else {
            (first..=24).collect()
        };
        let caught = (!skip).then(|| *come.last().unwrap());
        let mut want: Vec<(String, bool)> = come
            .iter()
            .map(|day| (format!("{day} 08:00"), Some(*day) == caught))
            .collect();
        let got = occurrences(&app, &key);
        let lines = lost_lines(&app, &key);
        let lost = want.iter().filter(|o| !o.1).count();
        if got != want || lines.len() != usize::from(lost > 0) {
            wrong.push(format!(
                "{name} back: wanted {want:?}, got {got:?}, lines {lines:?}"
            ));
        }
        if back < "08:00" {
            crate::bots::tick_at(&app, at(24, "08:00") + TimeDelta::seconds(20));
            want.push(("24 08:00".into(), true));
            let got = occurrences(&app, &key);
            if got != want {
                wrong.push(format!("{name} at its time: wanted {want:?}, got {got:?}"));
            }
        }
        let _ = std::fs::remove_dir_all(&home);
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_pass_is_caught_up_and_not_also_lost() {
    let home = std::env::temp_dir().join(format!("tirith-catchup-pass-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("config.toml"), "[network]\nsandbox = false\n[[routes]]\non = \"schedule\"\nto = \"ceo\"\nmode = \"turn\"\ndays = [1, 2, 3, 4, 5, 6, 7]\nat = \"07:00\"\n").unwrap();
    let app = crate::app::App::open_at(home.clone());
    app.set_kv("born", Some(&calendar::utc(at(20, "12:00"))));
    watched(&app, ("ceo", "pass", "07:00"), at(21, "06:30"));
    let now = at(24, "06:00");
    super::tick(&app, now);
    assert!(
        super::pass_due(&app, "ceo", true, now),
        "the last time that came is caught up"
    );
    super::tick(&app, now + TimeDelta::seconds(30));
    let sql = "SELECT due, state FROM expected_runs WHERE conv = 'ceo' ORDER BY due, id";
    let got: Vec<(String, String)> = app
        .query(sql, [], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .into_iter()
        .map(|(due, state)| (shown(&due), state))
        .collect();
    let want = [
        ("21 07:00", "failed"),
        ("22 07:00", "failed"),
        ("23 07:00", "waiting"),
    ];
    assert_eq!(got, want.map(|(d, s)| (d.to_string(), s.to_string())));
    assert_eq!(lost_lines(&app, "ceo").len(), 1);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_restart_hands_a_caught_up_run_again() {
    let home = std::env::temp_dir().join(format!("tirith-catchup-restart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("config.toml"), "[agents.fake]\ncommand = \"/nonexistent/agent\"\nchat_args = []\n[network]\nsandbox = false\n[roles.bot-cogido]\nextends = \"manager\"\nscope = \"own\"\nsession = \"chat\"\nname = \"Riega\"\n").unwrap();
    let app = crate::app::App::open_at(home.clone());
    app.set_kv("models:bot-cogido", Some(r#"{"agent":"fake"}"#));
    let due = calendar::utc(Local::now() - TimeDelta::days(1));
    let new = super::New {
        conv: "bot-cogido",
        routine: "r1",
        title: "Regar",
        trigger: "schedule",
        kind: "schedule",
        due: &due,
        task: "water, late",
    };
    super::expect(&app, &new); // caught up today, and not started when NeoBot stopped
    super::start(app.clone());
    let why = app.query("SELECT COALESCE(why, '') FROM expected_runs", [], |r| {
        r.get::<_, String>(0)
    });
    assert!(
        !why[0].contains("se paró antes"),
        "handed again, not lost: {why:?}"
    );
    let _ = std::fs::remove_dir_all(&home);
}
