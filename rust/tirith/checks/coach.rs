use serde_json::json;

fn home(name: &str) -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!("tirith-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    home
}

fn worse_events(app: &crate::app::Shared) -> Vec<String> {
    app.query(
        "SELECT summary FROM events WHERE kind = 'routine.worse'",
        [],
        |r| r.get(0),
    )
}

#[test]
fn routine_worse_numbers() {
    let run = |failed: bool, tokens: f64, cost: Option<f64>| super::worse::Run {
        failed,
        tokens,
        cost,
    };
    // Newest first: the last 5 against the 10 before.
    let mut runs: Vec<_> = (0..5).map(|_| run(false, 130.0, Some(0.01))).collect();
    runs.extend((0..10).map(|_| run(false, 100.0, Some(0.01))));
    let why = super::worse::worse(&runs).expect("30 % more tokens is worse");
    assert!(
        why.contains("tokens per run 130 against 100") && !why.contains('$'),
        "{why}"
    );
    runs[0].tokens = 120.0;
    assert_eq!(
        super::worse::worse(&runs),
        None,
        "under 30 % more is not worse"
    );
    runs[0].cost = Some(0.03);
    assert!(
        super::worse::worse(&runs).is_some_and(|w| w.contains("$ per run")),
        "dollars count too"
    );
    let few: Vec<_> = (0..5).map(|i| run(i < 2, 100.0, None)).collect();
    assert!(
        super::worse::worse(&few).is_some_and(|w| w.contains("2 of the last 5 runs failed")),
        "2 failures are worse, with no 10 runs before needed"
    );
}

#[test]
fn coach_writes_only_in_the_bots_folders_and_the_lab() {
    let home = home("coach-writes");
    let app = crate::app::App::open_at(home.clone());
    let write = |role: &str, path: &str| {
        let file = home.join(path).display().to_string();
        let (tool, input) =
            crate::rules::normalize("Write", &json!({"file_path": file, "content": "x"}), "");
        crate::rules::check(&app, &tool, &input, None, None, Some(role))
    };
    for ok in [
        "manager/bot-abc123/memory.md",
        "manager/bot-abc123/.agents/skills/x/SKILL.md",
        "bots/memory.md",
        "bots/skills/x/SKILL.md",
        "lab/exp-1/notes.md",
        "lab/bot-abc123/draft/.agents/skills/x/SKILL.md",
    ] {
        assert_eq!(write("coach", ok), None, "{ok}");
    }
    for refused in [
        "config.toml",
        "manager/tirith/notes.md",
        "bots/.git/config",
        "lab/.agent/settings.json",
        "lab/bot-abc123/draft/.agents/settings.json",
        "lab/bot-abc123/draft/.agents/skills/x/.git/config",
        "lab/bot-abc123/draft/.git/.agents/skills/x/SKILL.md",
        "/tmp/elsewhere.md",
    ] {
        assert!(write("coach", refused).is_some(), "{refused}");
    }
    // Another role is not confined by the Coach's folders: the lab stays OpenBot's for it.
    assert!(write("manager", "lab/exp-1/notes.md").is_some());
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn coach_runs_on_routine_worse() {
    // The Coach on a fake stream-json agent that keeps the brief it gets and answers «Sin novedad.».
    let home = home("coach-worse");
    let agent = home.join("fake-agent.sh");
    std::fs::write(&agent, r#"#!/bin/sh
echo '{"type":"system","subtype":"init","session_id":"fake-coach"}'
while read -r line; do
  printf '%s\n' "$line" > "$TIRITH_HOME/coach-brief.json"
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"Sin novedad."}]}}'
  echo '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.001,"usage":{"input_tokens":3,"output_tokens":2}}'
done
"#).unwrap();
    std::fs::set_permissions(&agent, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[agents.fake]\ncommand = \"{}\"\nchat_args = []\n[network]\nsandbox = false\n",
            agent.display()
        ),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    app.set_kv("models:coach", Some(r#"{"agent":"fake"}"#));
    // The CEO is still the red measures' pass; the Coach is a turn role of its own with the "bots" brief.
    assert_eq!(super::review::delayed_review(&app).0, "ceo");
    assert_eq!(super::roles::brief_key(&app, "ceo"), "ceo");
    assert_eq!(super::roles::brief_key(&app, "coach"), "bots");
    let bot = crate::bots::save(&app, json!({"name": "Correo", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let key = format!("bot-{}", bot["id"].as_str().unwrap());

    // 10 runs of routine r1 at 100 tokens, then 5 at 200: dearer by 100 %.
    let seed = |tokens: i64| {
        app.insert("INSERT INTO turns (conv, role, \"trigger\", routine, started_at, status, tokens_in, tokens_out) VALUES (?1, ?1, 'schedule', 'r1', '2026-11-21T08:00:00Z', 'ok', ?2, 0)", rusqlite::params![key, tokens]);
    };
    (0..10).for_each(|_| seed(100));
    (0..5).for_each(|_| seed(200));
    super::worse::check(&app);
    super::worse::check(&app); // the same episode: not emitted again
    let events = worse_events(&app);
    assert_eq!(events.len(), 1, "{events:?}");
    assert!(
        events[0].contains("Correo") && events[0].contains("r1") && events[0].contains("+100 %"),
        "{events:?}"
    );

    // Its route runs the Coach with the bots brief; its answer lands in its own conversation.
    let mut passes = vec![];
    for _ in 0..300 {
        passes = app.query(
            "SELECT metric, text FROM ceo_reviews WHERE role = 'coach'",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        );
        if !passes.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert_eq!(
        passes,
        vec![(
            "event:routine.worse".to_string(),
            "Sin novedad.".to_string()
        )]
    );
    let brief = std::fs::read_to_string(home.join("coach-brief.json")).unwrap();
    assert!(
        brief.contains("# Bots")
            && brief.contains("## Correo")
            && brief.contains("routine.worse: Routine worse"),
        "{brief}"
    );
    let turn = crate::turns::rows_of(&app, "coach", 5);
    assert_eq!(
        turn[0]["trigger"].as_str(),
        Some("event:routine.worse"),
        "{turn:?}"
    );
    let said = app.query("SELECT text FROM chat WHERE project = 'coach'", [], |r| {
        r.get::<_, String>(0)
    });
    assert!(said.iter().any(|t| t == "Sin novedad."), "{said:?}");
    // Not a CEO review: the page's reviews and the CEO's budget do not count it.
    assert!(super::api::payload(&app)["reviews"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(super::pass::under_budget(&app, "ceo", crate::app::now_s()));

    // Cheaper again ends the episode; dearer again is a new one.
    (0..5).for_each(|_| seed(100));
    super::worse::check(&app);
    assert_eq!(app.kv(&format!("routine_worse:{key}:r1")), None);
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn worse_counts_neither_waiting_nor_b_runs() {
    let home = home("coach-waiting");
    std::fs::write(home.join("config.toml"), "[network]\nsandbox = false\n").unwrap();
    let app = crate::app::App::open_at(home.clone());
    let bot = crate::bots::save(&app, json!({"name": "Espera", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read"})).unwrap();
    let key = format!("bot-{}", bot["id"].as_str().unwrap());
    let seed = |status: &str, tokens: i64, variant: Option<&str>| {
        app.insert("INSERT INTO turns (conv, role, \"trigger\", routine, started_at, status, tokens_in, tokens_out, variant) VALUES (?1, ?1, 'schedule', 'r1', '2026-11-21T08:00:00Z', ?2, ?3, 0, ?4)", rusqlite::params![key, status, tokens, variant]);
    };
    (0..10).for_each(|_| seed("ok", 100, None));
    // The last 5 runs of the routine: two wait for the owner's yes (neither failed nor good), three are fine; the experiment's B runs, failed and dear, are no runs of the routine.
    (0..3).for_each(|_| seed("ok", 100, None));
    (0..2).for_each(|_| seed("waiting", 100, None));
    (0..4).for_each(|_| seed("failed", 900, Some("B")));
    let worse =
        |app: &crate::app::Shared| super::worse::worse(&super::worse::runs(app, &key, "r1"));
    assert_eq!(worse(&app), None);
    // The same two as failures are worse: that is what waiting is not.
    app.insert(
        "UPDATE turns SET status = 'failed' WHERE status = 'waiting'",
        &[],
    );
    assert!(worse(&app).is_some_and(|w| w.contains("2 of the last 5 runs failed")));
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn a_worse_routine_heard_during_a_pass_is_not_lost() {
    // The Coach on a fake agent that keeps each brief it gets and waits for a gate file before it answers.
    let home = home("coach-queued");
    let agent = home.join("fake-agent.sh");
    std::fs::write(&agent, r#"#!/bin/sh
echo '{"type":"system","subtype":"init","session_id":"fake-coach"}'
while read -r line; do
  n=$(( $(ls "$TIRITH_HOME" | grep -c '^coach-brief-') + 1 ))
  printf '%s\n' "$line" > "$TIRITH_HOME/coach-brief-$n.json"
  while [ ! -e "$TIRITH_HOME/go-$n" ]; do sleep 0.05; done
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"Sin novedad."}]}}'
  echo '{"type":"result","subtype":"success","is_error":false,"usage":{"input_tokens":3,"output_tokens":2}}'
done
"#).unwrap();
    std::fs::set_permissions(&agent, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[agents.fake]\ncommand = \"{}\"\nchat_args = []\n[network]\nsandbox = false\n\n[roles.coach2]\nextends = \"coach\"\n",
            agent.display()
        ),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    app.set_kv("models:coach2", Some(r#"{"agent":"fake"}"#));
    let passes = |app: &crate::app::Shared| -> i64 {
        let sql = "SELECT COUNT(*) FROM ceo_reviews WHERE role = 'coach2'";
        app.query(sql, [], |r| r.get(0)).remove(0)
    };
    // A routine goes worse and the Coach starts its pass; while it runs, two more routines go worse.
    super::heard(&app, "coach2", "routine.worse", "Routine worse: first");
    until("the first brief", || {
        home.join("coach-brief-1.json").exists()
    })
    .await;
    super::heard(&app, "coach2", "routine.worse", "Routine worse: second");
    super::heard(&app, "coach2", "routine.worse", "Routine worse: third");
    assert_eq!(passes(&app), 0);
    std::fs::write(home.join("go-1"), "").unwrap();
    until("the second pass", || {
        home.join("coach-brief-2.json").exists()
    })
    .await;
    std::fs::write(home.join("go-2"), "").unwrap();
    until("both passes kept", || passes(&app) == 2).await;
    let second = std::fs::read_to_string(home.join("coach-brief-2.json")).unwrap();
    assert!(
        second.contains("Routine worse: second") && second.contains("Routine worse: third"),
        "the events heard during the pass are run together once it ends: {second}"
    );
    assert!(!home.join("coach-brief-3.json").exists(), "and only once");
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

async fn until(what: &str, mut f: impl FnMut() -> bool) {
    for _ in 0..300 {
        if f() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("never happened: {what}");
}
