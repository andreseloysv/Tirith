use super::env::stage_env;
use super::limits::{limit_from, MANAGER_LIMIT_SECS};
use super::locks::stage_lock;
use super::news::merged_note;
use super::red::red_stage_if_stale;
use super::slot::{disk_full, drop_half_made};
use crate::app::now_s;
use crate::config::Stage;
use std::time::Duration;
#[test]
fn stage_timeouts_per_project() {
    let kv = std::collections::HashMap::from([
        ("stage_learned:zoo:build".to_string(), "60".to_string()),
        (
            "stage_limit:tirith:test".to_string(),
            format!("99@{}", now_s()),
        ),
        (
            "stage_limit:tirith:old".to_string(),
            format!("99@{}", now_s() - MANAGER_LIMIT_SECS - 1),
        ),
    ]);
    let get = |k: &str| kv.get(k).cloned();
    let got = [
        ("zoo", "build"),
        ("tirith", "build"),
        ("tirith", "test"),
        ("tirith", "old"),
    ]
    .map(|(p, st)| limit_from(get, p, st, 30));
    assert_eq!(got, [60, 30, 99, 30]);
}
#[tokio::test]
async fn project_test_stages_serialized() {
    assert!(
        std::sync::Arc::ptr_eq(&stage_lock("p1"), &stage_lock("p1"))
            && !std::sync::Arc::ptr_eq(&stage_lock("p1"), &stage_lock("p2"))
    );
    let (lock, running) = (
        stage_lock("ser"),
        std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    );
    let mut tasks = vec![];
    for _ in 0..4 {
        let (lock, running) = (stage_lock("ser"), running.clone());
        tasks.push(tokio::spawn(async move {
            let _g = lock.lock().await;
            assert_eq!(running.fetch_add(1, std::sync::atomic::Ordering::SeqCst), 0);
            tokio::time::sleep(Duration::from_millis(20)).await;
            running.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }));
    }
    drop(lock);
    for t in tasks {
        t.await.unwrap();
    }
    assert!(
        include_str!("../src/merge/stage.rs").contains("let _one_at_a_time = lock.lock().await;")
    );
}
#[test]
fn merged_note_is_short() {
    let long = format!(
        "Fábrica 18, sin ruido repetido: {}",
        "el detalle entero del ítem, ".repeat(60)
    );
    assert!(
        long.chars().count() > 1700,
        "the whole item, as the note used to carry it"
    );
    let note = merged_note("tirith", "abc12345", &[&long]);
    assert_eq!(
        note, "merged on tirith at abc12345: Fábrica 18, sin ruido repetido",
        "only up to the first ': '"
    );
    // Each cut: the first of ': ', ' (' and '. ' in the item, whichever comes first.
    assert!(
        merged_note("p", "s", &["Arreglar el login (check: cargo test) y más"])
            .ends_with(": Arreglar el login")
    );
    assert!(
        merged_note("p", "s", &["Subir la versión. Después avisar al dueño"])
            .ends_with(": Subir la versión")
    );
    assert!(
        merged_note("p", "s", &["Con paréntesis (antes: que los dos puntos)"])
            .ends_with(": Con paréntesis")
    );
    assert!(
        merged_note("p", "s", &["Sin cortes"]).ends_with(": Sin cortes"),
        "a short item without a cut stays whole"
    );
    // No cut in the first 120 chars: still 120 chars at most per item (chars, not bytes).
    let note = merged_note("p", "s", &[&"á".repeat(300), "Otro: ítem"]);
    let heads: Vec<&str> = note.split(": ").nth(1).unwrap().split("; ").collect();
    assert_eq!(heads[0].chars().count(), 120, "{note}");
    assert_eq!(heads[1], "Otro", "each item is cut on its own: {note}");
}
#[test]
fn main_red_uses_latest_main() {
    let head = "0e89bb4aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    assert_eq!(
        red_stage_if_stale("test at d7fa7cd8", head),
        Some("test".into())
    );
    assert_eq!(
        (
            red_stage_if_stale("cargo test at 0e89bb4a", head),
            red_stage_if_stale("test at d7fa7cd8", "")
        ),
        (None, None)
    );
}
#[test]
fn mergeq_enospc_deletes_the_folder_and_holds_no_slot() {
    let dir = std::env::temp_dir().join(format!("tirith-enospc-{}", std::process::id()));
    let err = "git worktree add: fatal: write error: No space left on device";
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    assert!(disk_full(err) && drop_half_made(&dir, err) && !dir.exists());
    // another failure keeps the folder
    std::fs::create_dir_all(&dir).unwrap();
    assert!(!drop_half_made(&dir, "git worktree add: fatal: conflict") && dir.exists());
    std::fs::remove_dir_all(&dir).unwrap();
}
#[test]
fn item_check_uses_stage_env() {
    let st = |name: &str, run: &str| Stage {
        name: name.into(),
        run: run.into(),
        paths: vec![],
        kind: String::new(),
    };
    let stages = [
        st(
            "android",
            "cd android && JAVA_HOME=/jdk17 ANDROID_HOME=\"/sdk\" ./gradlew assembleDebug",
        ),
        st("build", "cargo build"),
    ];
    let env = stage_env(&stages, "cd android && ./gradlew lint");
    assert_eq!(
        env,
        [
            ("JAVA_HOME".into(), "/jdk17".into()),
            ("ANDROID_HOME".into(), "/sdk".into())
        ]
    );
    assert_eq!(stage_env(&stages, "cargo test -q x"), []);
}
/// A home with project "redp" on a one-commit repo, main red at its head, and merge-queued lanes (oldest first).
async fn red_main_fixture(tag: &str, lanes: &[(&str, &str)]) -> (crate::app::Shared, String) {
    let root = std::env::temp_dir().join(format!("tirith-redmain-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (home, repo) = (root.join("home"), root.join("repo"));
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(repo.join("f"), "x").unwrap();
    for a in [
        "init -q",
        "config user.email t@t",
        "config user.name t",
        "add .",
        "commit -qm i",
    ] {
        crate::app::git(&repo, &a.split(' ').collect::<Vec<_>>())
            .await
            .unwrap();
    }
    let cfg = format!("[network]\nsandbox = false\n\n[[projects]]\nid = \"redp\"\nrepo = \"{}\"\n\n[[projects]]\nid = \"other\"\nrepo = \"{}\"\n", repo.display(), repo.display());
    std::fs::write(home.join("config.toml"), cfg).unwrap();
    let app = crate::app::App::open_at(home);
    let head = crate::app::git(&repo, &["rev-parse", "HEAD"])
        .await
        .unwrap();
    app.set_kv("main_red:redp", Some(&format!("test at {}", &head[..8])));
    for (i, (id, project)) in lanes.iter().enumerate() {
        app.save_lane(&crate::app::Lane {
            id: id.to_string(),
            project: project.to_string(),
            status: "merge-queued".into(),
            ended_at: Some(format!("2026-11-24T00:00:0{i}Z")),
            ..Default::default()
        });
    }
    (app, head[..8].to_string())
}
#[tokio::test]
async fn red_main_still_tries_its_oldest_lane_alone() {
    use super::batch::{next_batch, waits_note};
    let ids = |b: Vec<crate::app::Lane>| b.into_iter().map(|l| (l.id, l.solo)).collect::<Vec<_>>();
    // The lane that fixes main waited forever: every lane of a red project was held back.
    let (app, head8) = red_main_fixture("trial", &[("fix", "redp"), ("next", "redp")]).await;
    assert_eq!(
        ids(next_batch(&app).await),
        [("fix".to_string(), true)],
        "the oldest lane rides alone"
    );
    // One that already failed alone on this red main waits; the next lane gets its trial.
    app.update_lane("fix", |l| l.note = waits_note(&head8));
    assert_eq!(ids(next_batch(&app).await), [("next".to_string(), true)]);
    // Main moves (still red): the waiting lane is tried again.
    assert!(
        super::batch::red_trial(next_queue(&app), "redp", "ffffffff")
            .iter()
            .any(|l| l.id == "fix" && l.solo)
    );
    // Green main: the lanes ride one train together as before.
    app.set_kv("main_red:redp", None);
    assert_eq!(
        ids(next_batch(&app).await),
        [("fix".to_string(), false), ("next".to_string(), false)]
    );
    let root = std::env::temp_dir().join(format!("tirith-redmain-trial-{}", std::process::id()));
    std::fs::remove_dir_all(root).unwrap();
}
fn next_queue(app: &crate::app::Shared) -> Vec<crate::app::Lane> {
    let mut q: Vec<_> = app.lanes.lock().unwrap().values().cloned().collect();
    q.sort_by(|a, b| a.ended_at.cmp(&b.ended_at));
    q
}
#[test]
fn red_trial_leaves_other_projects_and_holds_an_unknown_head() {
    use super::batch::red_trial;
    let lane = |id: &str, p: &str| crate::app::Lane {
        id: id.into(),
        project: p.into(),
        ..Default::default()
    };
    let q = || vec![lane("a", "redp"), lane("b", "other"), lane("c", "redp")];
    let got: Vec<_> = red_trial(q(), "redp", "abcd1234")
        .into_iter()
        .map(|l| (l.id, l.solo))
        .collect();
    assert_eq!(got, [("a".into(), true), ("b".into(), false)]);
    let held: Vec<_> = red_trial(q(), "redp", "")
        .into_iter()
        .map(|l| l.id)
        .collect();
    assert_eq!(
        held,
        ["b"],
        "a red main at an unknown head still holds its lanes back"
    );
}
