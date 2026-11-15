use super::*;
use std::process::Command;
fn git(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new("git").current_dir(dir).args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"]).args(args).output().unwrap()
}
#[test]
fn guard_merge_ignores_parent_content() {
    let w = ["op", "us"].concat();
    let dir = std::env::temp_dir().join(format!("tirith-guard-merge-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let save = |file: &str, text: &str, msg: &str| {
        std::fs::write(dir.join(file), text).unwrap();
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-q", "-m", msg]).status.success()
    };
    git(&dir, &["init", "-q", "-b", "main"]);
    assert!(save("a.txt", "one\n", "base"));
    git(&dir, &["checkout", "-q", "-b", "side"]);
    assert!(save("zoo.txt", &format!("see {w} here\n"), "side"));
    git(&dir, &["checkout", "-q", "-b", "other", "main"]);
    assert!(save("c.txt", "c\n", "other"));
    git(&dir, &["checkout", "-q", "main"]);
    assert!(save("a.txt", "one\ntwo\n", "main"));
    // Install the guard now that history exists.
    let hooks = dir.join(".git").join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("pre-commit");
    std::fs::write(&hook, render("pre-commit", &[w.clone()])).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    // The merge brings the parent's forbidden line: allowed.
    let m = git(&dir, &["merge", "-q", "side", "-m", "merge"]);
    assert!(m.status.success(), "merge with parent content must pass: {}", String::from_utf8_lossy(&m.stderr));
    // A plain commit adding the word is still blocked.
    assert!(!save("b.txt", &format!("{w}\n"), "x"));
    git(&dir, &["reset", "-q", "--hard", "HEAD"]);
    // A merge that adds a forbidden line of its own is still blocked.
    git(&dir, &["merge", "-q", "--no-commit", "--no-ff", "other"]);
    assert!(!save("c.txt", &format!("c\n{w}\n"), "merge2"), "merge-added content must still be blocked");
    let _ = std::fs::remove_dir_all(&dir);
}
