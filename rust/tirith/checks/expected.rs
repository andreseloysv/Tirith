// A scheduled run is never lost in silence, and «∅» is never "I could not" (expected/).
use super::{calendar, claims, store, verdict, words};
use chrono::{DateTime, Local, NaiveDate};
use serde_json::{json, Value};

/// A home with one bot and one routine (`days` at `at`), on an agent whose command is `command`.
fn home_of(
    name: &str,
    command: &str,
    days: &str,
    at: &str,
) -> (crate::app::Shared, std::path::PathBuf, String) {
    let home = std::env::temp_dir().join(format!("tirith-expected-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let key = format!("bot-{name}");
    let config = format!("[agents.fake]\ncommand = \"{command}\"\nchat_args = []\n[network]\nsandbox = false\n\n[roles.{key}]\nextends = \"manager\"\nscope = \"own\"\nsession = \"chat\"\nname = \"Vigila\"\n\n[[routes]]\non = \"schedule\"\nto = \"{key}\"\nmode = \"turn\"\nphone = true\nid = \"r1\"\ntitle = \"Mirar\"\ntask = \"look\"\ndays = [{days}]\nat = \"{at}\"\n");
    std::fs::write(home.join("config.toml"), config).unwrap();
    let app = crate::app::App::open_at(home.clone());
    for role in [key.as_str(), "judge", "ceo"] {
        app.set_kv(&format!("models:{role}"), Some(r#"{"agent":"fake"}"#));
    }
    (app, home, key)
}

fn at(month: u32, day: u32, hhmm: &str) -> DateTime<Local> {
    calendar::on(NaiveDate::from_ymd_opt(2026, month, day).unwrap(), hhmm).unwrap()
}

/// The turns of `key`, the first first, and Tirith's failure lines in its chat.
fn kept(app: &crate::app::Shared, key: &str) -> (Vec<Value>, Vec<String>) {
    let mut rows = crate::turns::rows_of(app, key, 100);
    rows.retain(|r| r["role"] == key);
    rows.reverse();
    let sql = "SELECT text FROM chat WHERE project = ?1 AND role = 'tirith' ORDER BY id";
    let mut said = app.query(sql, [key], |r| r.get::<_, String>(0));
    said.retain(|t| t.starts_with('✗'));
    (rows, said)
}

#[test]
fn nothing_new_is_never_could_not() {
    for text in [
        "No pude entrar en la cuenta",
        "no he podido abrir la página",
        "sin acceso al correo",
        "I could not open the site",
        "couldn't sign in",
        "unable to read the tab",
        "Ich konnte die Seite nicht öffnen",
    ] {
        assert!(claims::could_not(text), "{text}");
        assert!(!claims::quiet(&format!("∅ {text}")), "no silence: {text}");
        assert!(
            claims::honest(&format!("∅ {text}")).starts_with('✗'),
            "{text}"
        );
    }
    for text in [
        "∅ Nada nuevo desde el lunes",
        "∅ No pude encontrar ofertas nuevas",
        "∅ I could not find any new offers",
        "∅ Sin novedades",
    ] {
        assert!(claims::quiet(text), "true silence: {text}");
        assert_eq!(claims::honest(text), text);
    }
    assert_eq!(claims::honest("Hecho: 2 ofertas"), "Hecho: 2 ofertas");
}

#[test]
fn a_run_is_judged_by_what_it_did() {
    let looked = crate::turns::Looks::default();
    let judged = |ok, answer: &str, steps| verdict::judged(false, ok, answer, (steps, &looked));
    assert_eq!(judged(true, "∅ Nada nuevo", 1), None);
    assert_eq!(judged(true, "3 ofertas nuevas", 0), None);
    let no = |got: Option<String>, part: &str| {
        assert!(got.as_deref().is_some_and(|w| w.contains(part)), "{got:?}")
    };
    no(judged(true, "∅ Nada nuevo", 0), "sin mirar");
    no(judged(true, "∅ No pude abrir la página", 2), "no pudo");
    no(
        judged(true, "Miré una.\n\n∅ nada, la otra no pude abrirla", 2),
        "no pudo",
    );
    no(
        judged(true, "2 ofertas\n\n✗ el correo no abre", 2),
        "el correo no abre",
    );
    no(judged(true, "No he podido entrar en la web", 1), "no pudo");
    no(judged(true, "  ", 1), "sin responder");
    no(judged(false, "Credit balance is too low", 0), "sin cuota");
    no(
        judged(
            false,
            "API Error: Unable to connect to API (ECONNREFUSED)",
            0,
        ),
        "sin red",
    );
    assert_eq!(
        verdict::judged(true, true, "∅ nada que hacer", (0, &looked)),
        None,
        "an event brings its data"
    );
}

#[test]
fn a_look_that_failed_is_no_look() {
    let blind = |lines: &[Value]| {
        let mut looks = crate::turns::Looks::default();
        lines.iter().for_each(|m| looks.take(m));
        looks.blind().map(String::from)
    };
    let call = |id: &str, cmd: &str| json!({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": id, "name": "Bash", "input": {"command": cmd}}]}});
    let end = |id: &str, err: bool| json!({"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": id, "is_error": err, "content": "x"}]}});
    let page = "curl -s https://jobs.example/list";
    assert!(
        blind(&[
            call("1", "cat seen.md"),
            end("1", false),
            call("2", page),
            end("2", true)
        ])
        .is_some(),
        "the look failed"
    );
    assert_eq!(
        blind(&[
            call("1", page),
            end("1", true),
            call("2", page),
            end("2", false)
        ]),
        None,
        "it worked when tried again"
    );
    assert_eq!(
        blind(&[
            call("1", "cat seen.md"),
            end("1", true),
            call("2", page),
            end("2", false)
        ]),
        None,
        "no seen file yet is no blindness"
    );
    assert!(
        blind(&[call("1", "ls"), end("1", true)]).is_some(),
        "nothing worked"
    );
    let codex = json!({"type": "item.completed", "item": {"type": "command_execution", "command": page, "exit_code": 6}});
    assert!(blind(&[codex]).is_some());
    let muse = json!({"payload_type": "tool.result", "payload": {"correlation_facts": {"tool_name": "bash"}, "text": format!("{{\"command\":\"{page}\",\"exit_code\":6}}")}});
    assert!(blind(&[muse]).is_some());
}

#[test]
fn a_new_routine_says_when_it_first_runs() {
    // Made on a Friday evening for weekdays: the weekend has none, and the line says so.
    let r = json!({"trigger": "schedule", "at": "08:00", "days": [1, 2, 3, 4, 5]});
    let line = words::first(&r, at(11, 20, "17:59"));
    assert_eq!(line, " · lun a vie 08:00 · primera vez: lun 23/11 08:00");
    let r = json!({"trigger": "schedule", "at": "09:30", "days": [1, 2, 3, 4, 5, 6, 7]});
    assert_eq!(
        words::first(&r, at(11, 22, "07:00")),
        " · cada día 09:30 · primera vez: dom 22/11 09:30"
    );
}

#[test]
fn days_tirith_was_away_leave_a_row_each_and_one_line() {
    let (app, home, key) = home_of("away", "/nonexistent/agent", "1, 2, 3, 4, 5, 6, 7", "08:00");
    app.set_kv("routine:away:r1", Some("2026-01-01T07:00:00Z")); // its last run is long ago: nothing is invented from it
    super::tick(&app, at(11, 20, "18:00"));
    assert!(kept(&app, &key).0.is_empty(), "watched from now on");
    super::tick(&app, at(11, 24, "07:00"));
    let (rows, said) = kept(&app, &key);
    assert_eq!(rows.len(), 3, "21, 22 and 23: {rows:?}");
    for r in &rows {
        assert_eq!(
            (&r["status"], &r["trigger"], &r["routine"]),
            (&json!("failed"), &json!("schedule"), &json!("r1"))
        );
        assert!(r["why"].as_str().unwrap().contains("dormía"), "{r}");
        assert!(
            r["ms"].is_null() && r["agent"] == "",
            "no time, no agent: {r}"
        );
    }
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("3 veces") && said[0].contains("Mirar"),
        "{}",
        said[0]
    );
    super::tick(&app, at(11, 24, "07:01"));
    assert_eq!(kept(&app, &key).0.len(), 3, "never twice");
    let n = crate::numbers::numbers(&app, "away").unwrap();
    let r = &n["routines"][0];
    assert_eq!(
        (&r["runs"], &r["success"], &r["ms"]),
        (&json!(3), &json!(0.0), &Value::Null),
        "{r}"
    );
    assert!(r["last_failure"]["why"].as_str().is_some(), "{r}");
    let worse = app.query(
        "SELECT COUNT(*) FROM turns WHERE conv = ?1 AND status = 'failed' AND why <> ''",
        [&key],
        |r| r.get::<_, i64>(0),
    );
    assert_eq!(worse, vec![3]);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn stopped_is_told_from_asleep_and_a_day_not_its_own_is_no_miss() {
    let (app, home, key) = home_of("stopped", "/nonexistent/agent", "1, 2, 3, 4, 5", "08:00");
    let since = calendar::utc(at(11, 20, "17:59"));
    let sig = json!(["08:00", [1, 2, 3, 4, 5], true]).to_string();
    app.set_kv(
        "expected:watch:bot-stopped:r1:08:00",
        Some(&json!({"sig": sig, "since": since}).to_string()),
    );
    app.set_kv("expected:alive", Some(&since));
    super::tick(&app, at(11, 24, "09:00")); // this process's first beat: it was stopped
    let (rows, said) = kept(&app, &key);
    assert_eq!(rows.len(), 1, "only Monday was one of its days: {rows:?}");
    assert!(
        said.len() == 1
            && said[0].contains("lun 23/11 08:00")
            && said[0].contains("Tirith estaba parado"),
        "{said:?}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_restart_ends_what_it_cut_once() {
    let (app, home, key) = home_of("cut", "/nonexistent/agent", "1", "08:00");
    let new = |routine: &'static str, due: &'static str| {
        let n = store::New {
            conv: &key,
            routine,
            title: "Mirar",
            trigger: "schedule",
            kind: "schedule",
            due,
            task: "look",
        };
        store::expect(&app, &n)
    };
    new("r1", "2026-11-23T07:00:00Z");
    super::started(&app, &key, "r1"); // in the middle
    new("r2", "2026-01-05T07:00:00Z"); // waiting, and its day is long over
    super::start(app.clone());
    super::start(app.clone());
    let (rows, said) = kept(&app, &key);
    assert_eq!((rows.len(), said.len()), (2, 2), "{rows:?} {said:?}");
    assert!(
        rows[0]["why"].as_str().unwrap().contains("a mitad"),
        "{}",
        rows[0]
    );
    assert!(
        rows[1]["why"]
            .as_str()
            .unwrap()
            .contains("antes de que empezara"),
        "{}",
        rows[1]
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_pass_waits_for_its_budget_and_is_lost_with_its_day() {
    let (app, home, _) = home_of("pass", "/nonexistent/agent", "1", "08:00");
    let now = at(11, 24, "07:30");
    assert!(super::pass_due(&app, "ceo", true, now), "the CEO's 07:00");
    assert!(super::pass_due(&app, "ceo", true, now), "still waiting");
    super::blocked(
        &app,
        "ceo",
        "su tope de pasadas en 24 h (max_per_day) estaba gastado",
    );
    super::tick(&app, at(11, 25, "00:01"));
    let (rows, said) = kept(&app, "ceo");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(
        (&rows[0]["trigger"], &rows[0]["routine"]),
        (&json!("schedule"), &json!("pass"))
    );
    assert!(
        said.len() == 1 && said[0].contains("max_per_day"),
        "{said:?}"
    );
    assert!(
        !super::pass_due(&app, "ceo", true, now),
        "that occurrence is over"
    );
    let _ = std::fs::remove_dir_all(&home);
}

async fn until(what: &str, mut f: impl FnMut() -> bool) {
    for _ in 0..450 {
        if f() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("never happened: {what}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_that_never_starts_is_one_row_and_one_line() {
    let (app, home, key) = home_of(
        "nostart",
        "/nonexistent/agent",
        "1, 2, 3, 4, 5, 6, 7",
        "00:00",
    );
    crate::bots::tick(&app);
    until("its row", || !kept(&app, &key).0.is_empty()).await;
    let (rows, said) = kept(&app, &key);
    assert_eq!((rows.len(), said.len()), (1, 1), "{rows:?} {said:?}");
    assert!(
        rows[0]["why"].as_str().unwrap().contains("no arrancó"),
        "{}",
        rows[0]
    );
    let tirith = app.query(
        "SELECT COUNT(*) FROM chat WHERE project = ?1 AND role = 'tirith'",
        [&key],
        |r| r.get::<_, i64>(0),
    );
    assert_eq!(tirith, vec![1], "no second line");
    crate::bots::tick(&app);
    assert_eq!(kept(&app, &key).0.len(), 1, "that occurrence never again");
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_false_silence_is_a_failure_that_is_said() {
    let dir = std::env::temp_dir().join(format!("tirith-expected-agent-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let agent = dir.join("agent.sh");
    let script = "#!/bin/sh\necho '{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"s\"}'\nwhile read -r line; do\n  echo '{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"∅ No pude abrir la página\"}]}}'\n  echo '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}'\ndone\n";
    std::fs::write(&agent, script).unwrap();
    std::fs::set_permissions(&agent, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let (app, home, key) = home_of(
        "falsequiet",
        &agent.display().to_string(),
        "1, 2, 3, 4, 5, 6, 7",
        "00:00",
    );
    crate::bots::tick(&app);
    until("its row", || !kept(&app, &key).0.is_empty()).await;
    let (rows, said) = kept(&app, &key);
    assert_eq!(rows[0]["status"], "failed", "{}", rows[0]);
    assert!(said.len() == 1 && said[0].contains("no pudo"), "{said:?}");
    let shown = app.query(
        "SELECT text FROM chat WHERE project = ?1 AND role = 'manager'",
        [&key],
        |r| r.get::<_, String>(0),
    );
    assert_eq!(
        shown,
        vec!["✗ No pude abrir la página".to_string()],
        "never shown as silence"
    );
    let _ = (
        std::fs::remove_dir_all(&home),
        std::fs::remove_dir_all(&dir),
    );
}

/// A shell call of each agent's shape: it printed `output` and ended with `code`.
fn ran(what: &str, output: &str, code: i64) -> Vec<Vec<Value>> {
    let shell = json!({"command": what, "output": output, "exit_code": code}).to_string();
    vec![
        vec![
            json!({"payload_type": "tool.result", "payload": {"correlation_facts": {"tool_name": "bash"}, "text": shell}}),
        ],
        vec![
            json!({"type": "item.completed", "item": {"type": "command_execution", "command": what, "aggregated_output": output, "exit_code": code}}),
        ],
        vec![
            json!({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": "1", "name": "Bash", "input": {"command": what}}]}}),
            json!({"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": "1", "is_error": code != 0, "content": output}]}}),
        ],
        vec![
            json!({"type": "tool.execution_start", "data": {"toolCallId": "1", "toolName": "bash", "arguments": {"command": what}}}),
            json!({"type": "tool.execution_complete", "data": {"toolCallId": "1", "success": code == 0, "result": {"content": output}}}),
        ],
    ]
}

/// The verdict on `answer` after `calls`, the same for every agent's shape.
fn verdicts(answer: &str, calls: &[(&str, &str, i64)]) -> Vec<Option<String>> {
    (0..4)
        .map(|shape| {
            let mut looks = crate::turns::Looks::default();
            for (what, output, code) in calls {
                ran(what, output, *code)[shape]
                    .iter()
                    .for_each(|m| looks.take(m));
            }
            verdict::judged(false, true, answer, (calls.len() as i64, &looks))
        })
        .collect()
}

#[test]
fn could_not_is_a_failure_in_any_words() {
    // An inner agent's CLI ends well although its every command was refused: only its output says it.
    let nested = "exec\nsandbox-exec: sandbox_apply: Operation not permitted\ntokens used\n30\nI could not run it.";
    let call = [(
        "codex exec --sandbox workspace-write 'look at the plants'",
        nested,
        0,
    )];
    for answer in [
        "El ayudante falló y no pudo mirar: sandbox_apply: Operation not permitted.",
        "Hoy la revisión de plantas queda para otro rato.",
        "The helper had trouble, nothing checked today.",
        "Todo en orden.",
        "∅ Nada nuevo",
    ] {
        for got in verdicts(answer, &call) {
            let why = got.unwrap_or_else(|| panic!("a failure: {answer}"));
            assert!(
                why.contains("sandbox_apply: Operation not permitted"),
                "the tool's own error: {why}"
            );
        }
    }
    // Words alone, in both languages, with no mark.
    for answer in [
        "Hoy no pude revisar las plantas.",
        "El sensor falló y no pudo leer la humedad.",
        "Lo intenté dos veces y no he podido entrar.",
        "I could not check the plants today.",
        "The helper failed and couldn't finish.",
    ] {
        assert!(
            verdicts(answer, &[("ls", "a", 0)])
                .iter()
                .all(Option::is_some),
            "{answer}"
        );
    }
}

#[test]
fn finding_nothing_or_quoting_someone_is_no_failure() {
    let found = [("curl -s https://plants.example/new", "0 new", 0)];
    for answer in [
        "∅ Nada nuevo desde ayer",
        "∅ No pude encontrar plantas nuevas",
        "∅ I could not find any new plants",
        "No pude encontrar ninguna planta nueva.",
        "I couldn't find any new plants today.",
        "Un correo nuevo de Ana: «no pude regarlas, lo siento».",
        "Ana escribe que no pudo regar ayer.",
        "One new message: Sam says they could not come.",
        "> I could not make it\nOne new reply, nothing to do.",
        "El panel avisa de que 2 riegos fallaron esta noche.",
    ] {
        assert_eq!(
            verdicts(answer, &found),
            vec![None, None, None, None],
            "{answer}"
        );
    }
    // An error that a later try undid, and a page that talks about errors.
    let undone = [
        (
            "codex exec 'look'",
            "sandbox-exec: sandbox_apply: Operation not permitted",
            0,
        ),
        ("curl -s https://plants.example/new", "3 new", 0),
    ];
    assert_eq!(
        verdicts("3 plantas nuevas.", &undone),
        vec![None, None, None, None]
    );
    let page = [(
        "curl -s https://forum.example/t/9",
        "Thread\ncurl: (7) someone's paste\nline\nline\nline\nline\nline\n2 replies",
        0,
    )];
    assert_eq!(
        verdicts("2 respuestas nuevas en el foro.", &page),
        vec![None, None, None, None]
    );
}

#[test]
fn an_ordinary_run_is_no_failure() {
    let list = "https://jobs.example/list";
    let no_match = format!("curl -s {list} | grep -i remote");
    let look = format!("curl -s {list}");
    let fine = || vec![(look.as_str(), "3 offers", 0)];
    let cases: Vec<(&str, Vec<(&str, &str, i64)>)> = vec![
        ("∅ No new offers.", vec![(no_match.as_str(), "", 1)]),
        (
            "∅ Sin novedades en los PR.",
            vec![("gh pr list --state open | grep dependabot", "", 1)],
        ),
        (
            "∅ No new offers.",
            vec![
                (
                    "curl -sf https://jobs.example/old",
                    "curl: (22) The requested URL returned error: 404",
                    22,
                ),
                (look.as_str(), "3 offers", 0),
            ],
        ),
        ("∅ I can't see any new offers today.", fine()),
        ("∅ No puedo ver ninguna oferta nueva.", fine()),
        (
            "∅ Nada nuevo. No hay permisos pendientes de aprobar.",
            fine(),
        ),
        (
            "2 ofertas nuevas: A y B.\n\n✗ Descartada: C, pide alemán C2.",
            fine(),
        ),
        (
            "∅ Nothing new for you.",
            vec![(
                "curl -s https://wiki.example/Job",
                "Not logged in\nTalk\nA job is work.",
                0,
            )],
        ),
        (
            "∅ Nada nuevo en el blog.",
            vec![(
                "curl -s https://blog.example",
                "Post of yesterday\nYou've reached your limit of free articles",
                0,
            )],
        ),
        (
            "∅ Nada nuevo en el foro.",
            vec![(
                "curl -s https://forum.example",
                "sandbox_apply: Operation not permitted is the thread's title",
                0,
            )],
        ),
    ];
    let down = "curl: (7) Failed to connect to shop.example port 443";
    let mut cases = cases;
    cases.extend([
        // What failed is the news the task asks for: the judge reads whose failure it is.
        (
            "Tu web está caída desde las 03:00.",
            vec![("curl -sS https://shop.example", down, 7)],
        ),
        (
            "Tu web no responde: Failed to connect to shop.example port 443",
            vec![("curl -sS https://shop.example", down, 7)],
        ),
        (
            "CI is red: the build step failed.",
            vec![("gh run watch 7 --exit-status", "build failed", 1)],
        ),
        ("The deploy failed to start at 02:00.", fine()),
        (
            "El pago de la luz no se pudo completar; revisa la tarjeta.",
            fine(),
        ),
    ]);
    for (answer, calls) in cases {
        assert_eq!(verdicts(answer, &calls), vec![None; 4], "{answer}");
    }
}

#[test]
fn a_script_s_nested_refusal_still_proves_it_and_no_reason_shows_a_secret() {
    let script = [(
        "sh search-offers.sh",
        "sandbox-exec: sandbox_apply: Operation not permitted",
        0,
    )];
    for got in verdicts("∅ Nada nuevo", &script) {
        assert!(got.unwrap().contains("sandbox_apply"));
    }
    let found = [("ls", "a", 0)];
    for answer in [
        "✗ No pude entrar con API_KEY=not-a-real-value",
        "No pude entrar con API_KEY=not-a-real-value",
        "∅ No pude entrar con API_KEY=not-a-real-value",
    ] {
        for got in verdicts(answer, &found) {
            let why = got.unwrap();
            assert!(!why.contains("not-a-real-value"), "{why}");
        }
    }
}

/// Words that no list holds: the judge of the run says that it could not, and the run ends as a failure, once.
#[tokio::test]
async fn the_judge_tells_a_could_not_in_other_words() {
    let dir = std::env::temp_dir().join(format!("tirith-expected-agents-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ended = "La búsqueda con el otro agente terminó mal.";
    for (name, session, verdict, answer, fails) in [
        ("words", "chat", "0.1 — could not: Codex falló", ended, true),
        (
            "wordsturn",
            "turn",
            "0.1 — could not: Codex falló",
            ended,
            true,
        ),
        (
            "wordsgood",
            "chat",
            "0.8 — could not see one check",
            ended,
            false,
        ),
        // No word from the judge: plain words of trouble are never kept as a success.
        (
            "wordsnojudge",
            "chat",
            "no sé",
            "La búsqueda con el otro agente falló.",
            true,
        ),
        (
            "wordsnojudgeok",
            "chat",
            "no sé",
            "Dos novedades del otro agente.",
            false,
        ),
    ] {
        let agent = dir.join(format!("{name}.sh"));
        let script = r##"#!/bin/sh
echo '{"type":"system","subtype":"init","session_id":"s"}'
while read -r line; do
  case "$line" in
    *'grade how well'*) echo '{"type":"assistant","message":{"content":[{"type":"text","text":"VERDICT"}]}}' ;;
    *)
      echo '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"1","name":"Bash","input":{"command":"cat notes"}}]}}'
      echo '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"1","is_error":true,"content":"cat: notes: no such helper"}]}}'
      echo '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"2","name":"Bash","input":{"command":"ls"}}]}}'
      echo '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"2","is_error":false,"content":"a"}]}}'
      echo '{"type":"assistant","message":{"content":[{"type":"text","text":"PAD"}]}}'
      echo '{"type":"assistant","message":{"content":[{"type":"text","text":"ANSWER"}]}}' ;;
  esac
  echo '{"type":"result","subtype":"success","is_error":false,"usage":{"input_tokens":1,"output_tokens":1}}'
done
"##
        .replace("VERDICT", verdict)
        .replace("ANSWER", answer)
        .replace("PAD", &"Mirando. ".repeat(140));
        std::fs::write(&agent, script).unwrap();
        std::fs::set_permissions(&agent, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        let (first, home, key) = home_of(
            name,
            &agent.display().to_string(),
            "1, 2, 3, 4, 5, 6, 7",
            "00:00",
        );
        drop(first);
        let config = std::fs::read_to_string(home.join("config.toml")).unwrap();
        let config = config.replace("session = \"chat\"", &format!("session = \"{session}\""));
        std::fs::write(home.join("config.toml"), config).unwrap();
        let app = crate::app::App::open_at(home.clone());
        crate::bots::tick(&app);
        let row = || kept(&app, &key).0.first().cloned().unwrap_or_default();
        until("the judge's word", || {
            let judged = !app
                .query(
                    "SELECT id FROM scores WHERE eval = 'turn-quality'",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .is_empty();
            judged && !row().is_null() && (!fails || row()["status"] == "failed")
        })
        .await;
        let event = row()["event"].as_i64().unwrap();
        crate::turns::scored(&app, event, "turn-quality", row()["score"].as_f64()); // said again: nothing twice
        let (rows, said) = kept(&app, &key);
        crate::manager::end_session(&key);
        let shown = app.query("SELECT summary FROM events WHERE id = ?1", [event], |r| {
            r.get::<_, String>(0)
        });
        assert!(
            shown[0].contains(answer)
                && shown[0].contains("no such helper")
                && shown[0].contains("«a»"),
            "the judge is shown the end, the errors and what a good call gave: {shown:?}"
        );
        if fails {
            assert_eq!(rows[0]["status"], "failed", "{name}");
            assert_eq!(said.len(), 1, "one line: {said:?}");
            assert!(
                said[0].contains("no pudo")
                    // the judge's cause alone; the answer's words only where no judge gave one
                    && said[0].contains("Codex falló") != said[0].contains("otro agente"),
                "{said:?}"
            );
            let sql = "SELECT state FROM expected_runs WHERE conv = ?1";
            let states = app.query(sql, [&key], |r| r.get::<_, String>(0));
            assert_eq!(states, ["failed"]);
        } else {
            assert_eq!(rows[0]["status"], "ok", "{name}");
            assert!(said.is_empty(), "{said:?}");
        }
        drop(app);
        let _ = std::fs::remove_dir_all(&home);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
