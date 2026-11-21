use rusqlite::params;
use std::path::{Path, PathBuf};
use std::process::Command;

fn home(name: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("tirith-lab-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    home
}

const KEY: &str = "bot-mail01";

/// A Tirith home with one bot, «Correo», and its routine `r1` (check the inbox), its folder a repository with a memory.
fn app_with_a_bot(name: &str) -> (crate::app::Shared, PathBuf) {
    let home = home(name);
    std::fs::write(
        home.join("config.toml"),
        format!("[roles.{KEY}]\nextends = \"manager\"\nscope = \"own\"\nsession = \"chat\"\nname = \"Correo\"\n\n[[routes]]\non = \"schedule\"\nto = \"{KEY}\"\nmode = \"turn\"\nid = \"r1\"\ntitle = \"gmail-safari\"\ntask = \"Check the inbox and list the new offers\"\ndays = [1]\nat = \"08:00\"\n"),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    let dir = home.join("manager").join(KEY);
    std::fs::create_dir_all(dir.join(".agents/skills/inbox")).unwrap();
    std::fs::write(dir.join("memory.md"), "# Memory\n- open every mail\n").unwrap();
    std::fs::write(
        dir.join(".agents/skills/inbox/SKILL.md"),
        "open each mail\n",
    )
    .unwrap();
    crate::bots::versioned(&app, KEY, "library");
    (app, home)
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Coach",
            "-c",
            "user.email=coach@localhost",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// What the Coach does: a branch with one commit on the bot's main, made in a draft worktree that it removes.
fn branch(home: &Path, name: &str, memory: &str) {
    let dir = home.join("manager").join(KEY);
    let draft = home.join("lab").join(KEY).join("draft");
    git(
        &dir,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            name,
            &draft.display().to_string(),
        ],
    );
    std::fs::write(draft.join("memory.md"), memory).unwrap();
    git(&draft, &["add", "-A"]);
    git(
        &draft,
        &[
            "commit",
            "-q",
            "-m",
            &format!("{name}: search with a filter"),
        ],
    );
    git(&dir, &["worktree", "remove", &draft.display().to_string()]);
}

fn start(app: &crate::app::Shared, branch: &str) -> Result<i64, String> {
    let b = serde_json::json!({"bot": KEY, "branch": branch, "routine": "r1", "mode": "live", "hypothesis": "a filter instead of opening each mail: −40 % tokens, same result"});
    super::start(app, &b).map(|v| v["id"].as_i64().unwrap())
}

/// A finished run of experiment `e` as its turn row: variant, tokens, time, status, reaction.
fn run(
    app: &crate::app::Shared,
    e: i64,
    variant: &str,
    tokens: i64,
    ms: i64,
    status: &str,
    reaction: Option<i64>,
) -> i64 {
    app.insert("INSERT INTO turns (conv, role, \"trigger\", routine, started_at, ms, status, tokens_in, tokens_out, answer, experiment, variant, reaction) VALUES (?1, ?1, 'schedule', 'r1', '2026-11-21T08:00:00Z', ?2, ?3, ?4, 0, 'answer', ?5, ?6, ?7)", params![KEY, ms, status, tokens, e, variant, reaction])
}

/// `n` comparisons of experiment `e`, each with B's verdict.
fn compared(app: &crate::app::Shared, e: i64, verdicts: &[&str], b: (i64, &str)) {
    for v in verdicts {
        let a = run(app, e, "A", 1000, 60_000, "ok", None);
        let b = run(app, e, "B", b.0, 50_000, b.1, None);
        app.insert("INSERT INTO lab_comparisons (experiment, a_turn, b_turn, b_is, verdict) VALUES (?1, ?2, ?3, 'a', ?4)", params![e, a, b, v]);
    }
}

fn lines(app: &crate::app::Shared, kind: &str) -> Vec<String> {
    app.query(
        "SELECT summary FROM events WHERE kind = ?1 ORDER BY id",
        [kind],
        |r| r.get(0),
    )
}

#[test]
fn lab_live_decides_by_the_rule() {
    let lab = crate::factory::current().lab.clone();
    assert_eq!(
        (
            lab.inputs,
            lab.not_worse_share,
            lab.min_saving,
            lab.max_days
        ),
        (8, 0.5, 0.15, 7),
        "[lab] in factory.toml"
    );
    let (app, home) = app_with_a_bot("decides");
    let dir = home.join("manager").join(KEY);

    // The branch must be one commit on the bot's main.
    assert!(start(&app, "nothing").unwrap_err().contains("no branch"));
    branch(
        &home,
        "v3",
        "# Memory\n- search with a filter, never open each mail\n",
    );
    let e = start(&app, "v3").expect("one commit over main starts");
    let tree = home.join("lab").join(KEY).join(e.to_string());
    assert!(
        tree.join("memory.md").is_file(),
        "its worktree is checked out"
    );
    assert!(
        start(&app, "v3").unwrap_err().contains("one at a time"),
        "one experiment per routine"
    );

    // 7 of 8 comparisons: not due yet. The 8th, a tie: it saves 38 % of the tokens and ties every time.
    compared(&app, e, &["tie"; 7], (620, "ok"));
    super::decide(&app, e);
    assert_eq!(super::list(&app, KEY)[0]["status"], "running");
    compared(&app, e, &["tie"], (620, "ok"));
    // Meanwhile the bot's main moved on the same line (its notes numbered at a turn): B's tested text still lands.
    std::fs::write(
        dir.join("memory.md"),
        "# Memory\n- [m1] open every mail\n- [m2] the owner signs as Ana\n",
    )
    .unwrap();
    super::decide(&app, e);
    let row = &super::list(&app, KEY)[0];
    assert_eq!(row["status"], "adopted", "{row}");
    assert_eq!(row["numbers"]["tally"]["ties"], 8);
    assert_eq!(
        lines(&app, "lab.adopted"),
        vec!["Adopté gmail-safari v3: −38 % tokens, misma calidad (8/8)".to_string()]
    );
    let memory = std::fs::read_to_string(dir.join("memory.md")).unwrap();
    assert!(
        memory.contains("search with a filter"),
        "the branch is in the bot's main, its live folder: {memory}"
    );
    assert!(!tree.exists(), "the worktree goes");
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["rev-parse", "--verify", "-q", "refs/heads/v3"])
            .output()
            .unwrap()
            .stdout
            .is_empty(),
        "the branch goes"
    );
    let said = app.query("SELECT text FROM chat WHERE project = ?1", [KEY], |r| {
        r.get::<_, String>(0)
    });
    assert!(
        said.iter().any(|t| t.starts_with("Adopté gmail-safari v3")),
        "one line in the bot's chat: {said:?}"
    );

    // One that loses more than it wins is discarded: the branch goes and the bot's main stays as it was.
    branch(&home, "v4", "# Memory\n- read only the subjects\n");
    let e = start(&app, "v4").unwrap();
    compared(
        &app,
        e,
        &["lose", "lose", "lose", "lose", "lose", "win", "tie", "tie"],
        (300, "ok"),
    );
    super::decide(&app, e);
    assert_eq!(super::list(&app, KEY)[0]["status"], "discarded");
    assert_eq!(
        lines(&app, "lab.discarded"),
        vec!["Descarté gmail-safari v4: peor calidad (3/8), −70 % tokens".to_string()]
    );
    assert!(!std::fs::read_to_string(dir.join("memory.md"))
        .unwrap()
        .contains("subjects"));
    assert!(!home.join("lab").join(KEY).join(e.to_string()).exists());

    // Ties that save but bring a 👎 or a new failure are not adopted, nor a tie that saves too little.
    let rule = |t: super::Tally, b_tokens: f64| {
        let side = |tokens| super::Side {
            runs: 8,
            tokens,
            cost: None,
            ms: 1000.0,
        };
        crate::lab::rule::decide(&lab, &t, &side(1000.0), &side(b_tokens), "x v")
    };
    let ties = super::Tally {
        ties: 8,
        ..Default::default()
    };
    assert!(rule(ties.clone(), 600.0).0);
    assert!(
        !rule(
            super::Tally {
                down: 1,
                ..ties.clone()
            },
            600.0
        )
        .0
    );
    assert!(
        !rule(
            super::Tally {
                new_failures: 1,
                ..ties.clone()
            },
            600.0
        )
        .0
    );
    assert_eq!(
        rule(ties, 900.0),
        (
            false,
            "Descarté x v: ahorra poco (−10 % tokens), misma calidad (8/8)".to_string()
        )
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn lab_live_alternates() {
    let (app, home) = app_with_a_bot("alternates");
    let live = home.join("manager").join(KEY);
    branch(&home, "v3", "# Memory\n- the branch's memory\n");
    let e = start(&app, "v3").unwrap();
    let tree = home.join("lab").join(KEY).join(e.to_string());
    let mut seen = vec![];
    for _ in 0..4 {
        crate::lab::fired(&app, KEY, "r1");
        let folder = crate::bots::folder(&app, KEY);
        let handed = crate::bots::context(&app, KEY).unwrap();
        let id = run(&app, 0, "", 100, 1000, "ok", None);
        app.insert(
            "UPDATE turns SET experiment = NULL, variant = NULL, answer = ?1 WHERE id = ?2",
            params![format!("answer {id}"), id],
        );
        crate::lab::finished(&app, id, KEY, "r1");
        let variant: String = app
            .query("SELECT variant FROM turns WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .remove(0);
        seen.push((
            variant.clone(),
            folder == tree,
            handed.contains("the branch's memory"),
        ));
        assert_eq!(
            crate::bots::folder(&app, KEY),
            live,
            "back in its own folder after the run"
        );
    }
    let want = |v: &str, b: bool| (v.to_string(), b, b);
    assert_eq!(
        seen,
        vec![
            want("A", false),
            want("B", true),
            want("A", false),
            want("B", true)
        ]
    );
    // A turn of another routine (or of the owner) during the experiment is no run of it.
    let other = run(&app, 0, "", 100, 1000, "ok", None);
    app.insert(
        "UPDATE turns SET experiment = NULL, variant = NULL WHERE id = ?1",
        params![other],
    );
    crate::lab::fired(&app, KEY, "r1");
    crate::lab::finished(&app, other, KEY, "");
    let marks: Vec<Option<String>> =
        app.query("SELECT variant FROM turns WHERE id = ?1", [other], |r| {
            r.get(0)
        });
    assert_eq!(marks, vec![None]);
    // Each B was compared with the A before it, as the lab.compare event the judge reads.
    let pairs: Vec<(i64, i64)> = app.query(
        "SELECT a_turn, b_turn FROM lab_comparisons WHERE experiment = ?1 ORDER BY id",
        [e],
        |r| Ok((r.get(0)?, r.get(1)?)),
    );
    assert_eq!(pairs.len(), 2);
    assert!(pairs.iter().all(|(a, b)| b - a == 1), "{pairs:?}");
    let shown = lines(&app, "lab.compare");
    assert!(
        shown.len() == 2
            && shown[0].contains("Check the inbox")
            && shown[0].contains(&format!("answer {}", pairs[0].1)),
        "{shown:?}"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn ab_compare_is_blind() {
    let (b_first, b_is) = super::blind("the task", "A's answer", "B's answer", true);
    let (a_first, a_is) = super::blind("the task", "A's answer", "B's answer", false);
    let at = |t: &str, s: &str| t.find(s).unwrap();
    assert!(at(&b_first, "B's answer") < at(&b_first, "A's answer") && b_is == "a");
    assert!(at(&a_first, "A's answer") < at(&a_first, "B's answer") && a_is == "b");
    assert!(
        !b_first.contains("variant") && !b_first.contains(" B ") && b_first.contains("the task")
    );
    for (label, b_is, want) in [
        ("a", "a", "win"),
        ("b", "a", "lose"),
        ("b", "b", "win"),
        ("a", "b", "lose"),
        ("tie", "a", "tie"),
    ] {
        assert_eq!(
            super::verdict(label, b_is),
            Some(want),
            "{label} with B shown as {b_is}"
        );
    }
    assert_eq!(super::verdict("maybe", "a"), None);
    // The judge's answer, mapped through the comparison's kept letter.
    let (app, home) = app_with_a_bot("blind");
    app.insert("INSERT INTO experiments (id, bot, routine, branch, change, hypothesis, mode, status, created_at) VALUES (7, ?1, 'r1', 'v', 'c', 'h', 'live', 'running', ?2)", params![KEY, crate::app::now_iso()]);
    app.insert("INSERT INTO lab_comparisons (experiment, a_turn, b_turn, event, b_is) VALUES (7, 1, 2, 41, 'b'), (7, 3, 4, 42, 'a')", &[]);
    crate::lab::judged(&app, 41, "ab-compare", "b", "B lists every offer");
    crate::lab::judged(&app, 42, "ab-compare", "b", "a misses one");
    crate::lab::judged(&app, 42, "turn-quality", "a", "another eval");
    let v: Vec<String> = app.query("SELECT verdict FROM lab_comparisons ORDER BY id", [], |r| {
        r.get(0)
    });
    assert_eq!(v, vec!["win".to_string(), "lose".to_string()]);
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
