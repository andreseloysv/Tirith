use super::*;
#[test]
fn stage_timeouts_per_project() {
    let kv = std::collections::HashMap::from([
        ("stage_learned:zoo:build".to_string(), "60".to_string()),
        ("stage_limit:tirith:test".to_string(), format!("99@{}", now_s())),
        ("stage_limit:tirith:old".to_string(), format!("99@{}", now_s() - MANAGER_LIMIT_SECS - 1)),
    ]);
    let get = |k: &str| kv.get(k).cloned();
    let got = [("zoo", "build"), ("tirith", "build"), ("tirith", "test"), ("tirith", "old")].map(|(p, st)| limit_from(get, p, st, 30));
    assert_eq!(got, [60, 30, 99, 30]);
}
#[tokio::test]
async fn project_test_stages_serialized() {
    assert!(std::sync::Arc::ptr_eq(&stage_lock("p1"), &stage_lock("p1")) && !std::sync::Arc::ptr_eq(&stage_lock("p1"), &stage_lock("p2")));
    let (lock, running) = (stage_lock("ser"), std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)));
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
    assert!(include_str!("merge.rs").contains("let _one_at_a_time = lock.lock().await;"));
}
#[test]
fn main_red_uses_latest_main() {
    let head = "0e89bb4aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    assert_eq!(red_stage_if_stale("test at d7fa7cd8", head), Some("test".into()));
    assert_eq!((red_stage_if_stale("cargo test at 0e89bb4a", head), red_stage_if_stale("test at d7fa7cd8", "")), (None, None));
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
    let st = |name: &str, run: &str| Stage { name: name.into(), run: run.into(), paths: vec![], kind: String::new() };
    let stages = [st("android", "cd android && JAVA_HOME=/jdk17 ANDROID_HOME=\"/sdk\" ./gradlew assembleDebug"), st("build", "cargo build")];
    let env = stage_env(&stages, "cd android && ./gradlew lint");
    assert_eq!(env, [("JAVA_HOME".into(), "/jdk17".into()), ("ANDROID_HOME".into(), "/sdk".into())]);
    assert_eq!(stage_env(&stages, "cargo test -q x"), []);
}
