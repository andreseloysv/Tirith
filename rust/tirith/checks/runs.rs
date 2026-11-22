//! A routine's run is a turn of its own, with its own trigger and routine, in order, whatever else is said to its bot at the same moment.
use serde_json::{json, Value};

/// A fake stream-json agent that answers each message it is handed with a turn of its own, saying its count, its process and the routine it was handed.
const AGENT: &str = r#"#!/bin/sh
eff=none
prev=""
for a in "$@"; do
  [ "$prev" = "--effort" ] && eff="$a"
  prev="$a"
done
echo '{"type":"system","subtype":"init","session_id":"fake-runs"}'
n=0
while read -r line; do
  n=$((n+1))
  who=$(printf '%s' "$line" | grep -o 'Routine «[a-z]*»' | head -1)
  echo "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"done $n pid $$ $who effort $eff\"}]}}"
  echo '{"type":"result","subtype":"success","is_error":false,"usage":{"input_tokens":1,"output_tokens":1}}'
done
"#;

/// A home whose bot «Corre» has the routines `uno` and `dos`, both due every day from midnight, and a fake agent for it and for the judge.
fn app_with_two_routines(name: &str) -> (crate::app::Shared, std::path::PathBuf, String) {
    let key = format!("bot-{name}");
    let key = key.as_str();
    let home = std::env::temp_dir().join(format!("tirith-runs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let agent = home.join("fake-agent.sh");
    std::fs::write(&agent, AGENT).unwrap();
    std::fs::set_permissions(&agent, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let routine = |id: &str, title: &str| {
        format!("[[routes]]\non = \"schedule\"\nto = \"{key}\"\nmode = \"turn\"\nid = \"{id}\"\ntitle = \"{title}\"\ntask = \"look at {title}\"\ndays = [1, 2, 3, 4, 5, 6, 7]\nat = \"00:00\"\n\n")
    };
    let config = format!("[agents.fake]\ncommand = \"{}\"\nchat_args = []\neffort_flag = \"--effort\"\n[network]\nsandbox = false\n\n[roles.{key}]\nextends = \"manager\"\nscope = \"own\"\nsession = \"chat\"\nname = \"Corre\"\n\n{}{}", agent.display(), routine("r1", "uno"), routine("r2", "dos"));
    std::fs::write(home.join("config.toml"), config).unwrap();
    let app = crate::app::App::open_at(home.clone());
    for role in [key, "judge"] {
        app.set_kv(&format!("models:{role}"), Some(r#"{"agent":"fake"}"#));
    }
    (app, home, key.to_string())
}

/// The rows of the bot's own turns, the first first.
fn own(app: &crate::app::Shared, key: &str) -> Vec<Value> {
    let mut rows = crate::turns::rows_of(app, key, 50);
    rows.retain(|r| r["role"] == key);
    rows.reverse();
    rows
}

async fn until(what: &str, mut f: impl FnMut() -> bool) {
    for _ in 0..300 {
        if f() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("never happened: {what}");
}

/// Runs a scenario (a task) up to three times, each under its own name: another check that restarts every session of the process (`restart`) can end a live session in the middle of this one.
async fn attempts<F, Fut>(name: &str, scenario: F)
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    for n in 0..3 {
        if tokio::spawn(scenario(format!("{name}{n}"))).await.is_ok() {
            return;
        }
    }
    panic!("{name}: failed in three attempts (see their output)");
}

#[tokio::test(flavor = "multi_thread")]
async fn two_routines_and_an_owner_message_at_once() {
    attempts("once", at_once).await;
}

async fn at_once(name: String) {
    let (app, home, key) = app_with_two_routines(&name);
    // A message that starts no turn (`/models`) leaves no trigger behind to steal the next run's.
    crate::manager::say(&app, &key, "/models", "").unwrap();
    assert_eq!(crate::turns::take_trigger(&key), Default::default());
    // Both routines come due in the same minute and the owner writes at the same moment.
    crate::bots::tick(&app);
    crate::manager::say(&app, &key, "hola, ¿qué tal?", "").unwrap();
    until("three turns", || own(&app, &key).len() == 3).await;
    let rows = own(&app, &key);
    let by = |r: &Value| {
        (
            r["trigger"].as_str().unwrap().to_string(),
            r["routine"].as_str().unwrap().to_string(),
        )
    };
    let want = |t: &str, r: &str| (t.to_string(), r.to_string());
    // Each one is a turn of its own, in the order it was handed over: the routine due first, the owner's message that came during it, the second routine when the conversation was free.
    assert_eq!(
        rows.iter().map(by).collect::<Vec<_>>(),
        [
            want("schedule", "r1"),
            want("owner", ""),
            want("schedule", "r2")
        ],
        "{rows:?}"
    );
    // Each answer is the agent's, to its own message, and all three came from one live session.
    let answers: Vec<&str> = rows.iter().map(|r| r["answer"].as_str().unwrap()).collect();
    let pid = answers[0]
        .split(" pid ")
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap();
    assert!(
        answers[0].starts_with("done 1 ") && answers[0].contains("Routine «uno»"),
        "{answers:?}"
    );
    assert!(
        answers[1].starts_with("done 2 ") && !answers[1].contains("Routine"),
        "{answers:?}"
    );
    assert!(
        answers[2].starts_with("done 3 ") && answers[2].contains("Routine «dos»"),
        "{answers:?}"
    );
    assert!(
        answers.iter().all(|a| a.contains(&format!(" pid {pid}"))),
        "{answers:?}"
    );
    assert!(rows.iter().all(|r| r["status"] == "ok"), "{rows:?}");
    // Nothing stays waiting: no trigger, no job.
    assert_eq!(crate::turns::take_trigger(&key), Default::default());
    assert!(super::talk(&key, |t| t.jobs.is_empty() && t.pending.is_empty()));
    let _ = json!(null);
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_routines_own_effort_holds_for_its_turns_only() {
    attempts("tuned", tuned).await;
}

async fn tuned(name: String) {
    let (app, home, key) = app_with_two_routines(&name);
    app.set_kv(
        &format!("models:{key}"),
        Some(r#"{"agent":"fake","effort":"high"}"#),
    );
    // The bot's repository carries the override of routine r1 (tracked, so a branch can change it).
    let dir = home.join("manager").join(&key);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("routines.toml"), "[r1]\neffort = \"low\"\n").unwrap();
    crate::bots::versioned(&app, &key, "library");
    let tracked = std::process::Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args(["ls-files"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&tracked.stdout).contains("routines.toml"));
    crate::bots::tick(&app);
    until("two turns", || own(&app, &key).len() == 2).await;
    let rows = own(&app, &key);
    let (first, second) = (
        rows[0]["answer"].as_str().unwrap(),
        rows[1]["answer"].as_str().unwrap(),
    );
    assert_eq!(
        (rows[0]["routine"].as_str(), rows[0]["effort"].as_str()),
        (Some("r1"), Some("low")),
        "{rows:?}"
    );
    assert_eq!(
        (rows[1]["routine"].as_str(), rows[1]["effort"].as_str()),
        (Some("r2"), Some("high")),
        "{rows:?}"
    );
    assert!(
        first.contains("effort low") && second.contains("effort high"),
        "{first} / {second}"
    );
    let pid = |a: &str| {
        a.split(" pid ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap()
            .to_string()
    };
    assert_ne!(
        pid(first),
        pid(second),
        "a fresh session for the turn that has its own effort, and another after it"
    );
    // The owner's next message is the bot's own again.
    crate::manager::say(&app, &key, "hola", "").unwrap();
    until("the owner's turn", || own(&app, &key).len() == 3).await;
    assert!(own(&app, &key)[2]["answer"]
        .as_str()
        .unwrap()
        .contains("effort high"));
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_correction_from_the_owners_phone_counts() {
    let (app, home, key) = app_with_two_routines("phone");
    let moods = |app: &crate::app::Shared| -> i64 {
        let sql = "SELECT COUNT(*) FROM scores WHERE eval = 'owner-mood'";
        app.query(sql, [], |r| r.get(0)).remove(0)
    };
    // Another agent's line in the room is never judged as the owner's mood.
    crate::manager::from_room(
        &app,
        &key,
        "room-1",
        "otro-agente",
        "esto no era lo que pedí",
    );
    until("its turn", || own(&app, &key).len() == 1).await;
    assert_eq!(moods(&app), 0);
    // The paired phone's is the owner's: judged.
    crate::manager::from_room(
        &app,
        &key,
        "room-1",
        "iphone (the owner's paired phone)",
        "esto no era lo que pedí",
    );
    until("the phone's message judged", || moods(&app) == 1).await;
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

/// A fake exec agent (Codex's shape): it reads its one message, counts the turns in a file, and answers with the routine it was handed.
const EXEC_AGENT: &str = r#"#!/bin/sh
input=$(cat)
count="$0.count"
n=$(( $(cat "$count" 2>/dev/null || echo 0) + 1 ))
echo $n > "$count"
who=$(printf '%s' "$input" | grep -o 'Routine «[a-z]*»' | head -1)
echo '{"type":"thread.started","thread_id":"t1"}'
echo "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"done $n $who\"}}"
echo '{"type":"turn.completed"}'
"#;

#[tokio::test(flavor = "multi_thread")]
async fn an_exec_agent_runs_each_routine_alone() {
    let (app, home, key) = app_with_two_routines("exec");
    let exec = home.join("fake-codex.sh");
    std::fs::write(&exec, EXEC_AGENT).unwrap();
    std::fs::set_permissions(&exec, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let config = std::fs::read_to_string(home.join("config.toml")).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!("[agents.codex]\ncommand = \"{}\"\n{config}", exec.display()),
    )
    .unwrap();
    assert!(app.reload_config(true));
    app.set_kv(&format!("models:{key}"), Some(r#"{"agent":"codex"}"#));
    crate::manager::say(&app, &key, "/models", "").unwrap(); // starts no turn, leaves no trigger
    crate::bots::tick(&app);
    crate::manager::say(&app, &key, "hola", "").unwrap();
    until("three turns", || own(&app, &key).len() == 3).await;
    let rows = own(&app, &key);
    // Each run is a turn of its own with its own trigger and routine, and it was handed its own task. (The order depends on when the exec turn took its first message; none is merged or lost.)
    let seen = |trigger: &str, routine: &str| {
        let ones: Vec<&str> = rows
            .iter()
            .filter(|r| r["trigger"] == trigger && r["routine"] == routine)
            .map(|r| r["answer"].as_str().unwrap())
            .collect();
        assert_eq!(ones.len(), 1, "{rows:?}");
        ones[0].to_string()
    };
    assert!(
        seen("schedule", "r1").ends_with("Routine «uno»"),
        "{rows:?}"
    );
    assert!(
        seen("schedule", "r2").ends_with("Routine «dos»"),
        "{rows:?}"
    );
    assert!(!seen("owner", "").contains("Routine"), "{rows:?}");
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
