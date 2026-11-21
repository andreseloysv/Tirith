//! A lane's worktree: added, its excluded paths left out, its ignored local files copied in.
use super::git;
use std::path::Path;

/// A worktree at `base` (a new branch when `branch` is given), without the manifest's `lane_exclude` paths.
pub async fn add_worktree(
    repo: &Path,
    dir: &Path,
    branch: Option<&str>,
    exclude: &[String],
) -> Result<(), String> {
    let _ = dir.parent().map(std::fs::create_dir_all);
    let d = dir.to_string_lossy().to_string();
    let mut args = vec!["worktree", "add"];
    args.extend((!exclude.is_empty()).then_some("--no-checkout"));
    args.extend(branch.map_or(vec!["--detach", &d, "HEAD"], |b| vec!["-b", b, &d, "HEAD"]));
    git(repo, &args).await?;
    if !exclude.is_empty() {
        check_out_without(dir, exclude).await?;
    }
    copy_local_properties(repo, dir).await;
    Ok(())
}

/// Checks out the new worktree with a sparse checkout that leaves the `exclude` folders out.
async fn check_out_without(dir: &Path, exclude: &[String]) -> Result<(), String> {
    let sparse: Vec<String> = ["sparse-checkout", "set", "--no-cone", "/*"]
        .into_iter()
        .map(String::from)
        .chain(
            exclude
                .iter()
                .map(|p| format!("!/{}/", p.trim_matches('/'))),
        )
        .collect();
    git(dir, &sparse.iter().map(String::as_str).collect::<Vec<_>>()).await?;
    git(dir, &["checkout", "-q"]).await?;
    Ok(())
}

/// Copy the git-ignored `local.properties` files (Android `sdk.dir`) from the main checkout into the new worktree.
async fn copy_local_properties(repo: &Path, dir: &Path) {
    let args = [
        "ls-files",
        "--others",
        "--ignored",
        "--exclude-standard",
        "-z",
        "--",
        ":(glob)**/local.properties",
    ];
    git(repo, &args)
        .await
        .unwrap_or_default()
        .split('\0')
        .filter(|r| !r.is_empty())
        .for_each(|rel| {
            drop(dir.join(rel).parent().map(|p| {
                (
                    std::fs::create_dir_all(p),
                    std::fs::copy(repo.join(rel), dir.join(rel)),
                )
            }))
        });
}
