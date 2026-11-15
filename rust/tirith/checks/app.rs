use super::*;
#[test]
fn lane_log_redacts_key_prefixes() {
    let log = "GOOGLE_API_KEY=AIzaSyABCDEFGHIJKLMNOP\n{\"GITHUB_TOKEN\":\"ghp_abcdefghijklmnop12\"}\nMY_KEY: sk-abcdef123456789\nused AIzaSyZZZZZZZZZZZZZZ here\nplain line";
    let out = redact_secrets(log);
    ["AIza", "ghp_", "sk-", "abcdefghijklmnop12"].into_iter().for_each(|leak| assert!(!out.contains(leak), "leaked {leak}: {out}"));
    assert!(out.contains("GOOGLE_API_KEY=[redacted]") && out.contains("plain line"));
}
#[test]
fn child_path_includes_cargo_bin() {
    let home = std::env::temp_dir().join(format!("tirith-cp-{}", std::process::id()));
    let cargo = home.join(".cargo/bin");
    std::fs::create_dir_all(&cargo).unwrap();
    let p = path_with_cargo(Some(Path::new("/x")), "/usr/bin", &home, &[]);
    assert_eq!(p, format!("/x:/usr/bin:{}", cargo.display()));
    assert_eq!(path_with_cargo(None, &format!("/a:{}", cargo.display()), &home, &[]), format!("/a:{}", cargo.display()));
    let _ = std::fs::remove_dir_all(&home);
    assert_eq!(path_with_cargo(None, "/usr/bin", &home, &[]), "/usr/bin");
}
#[test]
fn child_path_includes_homebrew_bin() {
    let base = std::env::temp_dir().join(format!("tirith-hb-{}", std::process::id()));
    let brew = base.join("brew/bin");
    let dirs = [brew.to_str().unwrap(), "/nonexistent/bin"];
    std::fs::create_dir_all(&brew).unwrap();
    let p = path_with_cargo(None, "/usr/bin", &base, &dirs);
    assert_eq!((p.clone(), path_with_cargo(None, &p, &base, &dirs)), (format!("/usr/bin:{}", brew.display()), p));
}
/// A repository with one commit holding `files` (each a folder-relative path and its text).
async fn committed(repo: &Path, files: &[(&str, &str)]) {
    for (file, text) in files {
        let path = repo.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).and_then(|_| std::fs::write(path, text)).unwrap();
    }
    for a in ["init -q", "config user.email t@t", "config user.name t", "add .", "commit -qm i"] {
        git(repo, &a.split(' ').collect::<Vec<_>>()).await.unwrap();
    }
}
#[tokio::test]
async fn worktree_local_properties() {
    let base = std::env::temp_dir().join(format!("tirith-lp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let repo = base.join("repo");
    committed(&repo, &[(".gitignore", "local.properties\n"), ("README", "x")]).await;
    std::fs::create_dir_all(repo.join("android")).unwrap();
    std::fs::write(repo.join("android/local.properties"), "sdk.dir=/sdk\n").unwrap();
    let wt = base.join("wt");
    add_worktree(&repo, &wt, None, &[]).await.unwrap();
    assert_eq!(std::fs::read_to_string(wt.join("android/local.properties")).unwrap(), "sdk.dir=/sdk\n");
    let _ = std::fs::remove_dir_all(&base);
}
#[tokio::test]
async fn sparse_zoo_lane() {
    let repo = std::env::temp_dir().join(format!("tirith-sp-{}", std::process::id()));
    let ex: Vec<String> = ["output/research", "tools/content", "ZooBundles"].map(String::from).into();
    let files: Vec<(String, &str)> = ex.iter().map(String::as_str).chain(["cms"]).map(|d| (format!("{d}/f"), "x")).collect();
    committed(&repo, &files.iter().map(|(f, t)| (f.as_str(), *t)).collect::<Vec<_>>()).await;
    let wt = repo.join("../wt-sp");
    add_worktree(&repo, &wt, Some("lane/z"), &ex).await.unwrap();
    assert!(wt.join("cms/f").exists() && ex.iter().all(|p| !wt.join(p).exists()));
    let _ = std::fs::remove_dir_all(&wt);
    let _ = std::fs::remove_dir_all(&repo);
}
