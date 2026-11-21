//! Running git.
use std::path::Path;

/// `git -C dir args…`: stdout on success, the error text otherwise.
pub async fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = tokio::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let text = |b: &[u8]| String::from_utf8_lossy(b).trim().to_string();
    out.status
        .success()
        .then(|| text(&out.stdout))
        .ok_or_else(|| {
            format!(
                "git {}: {} {}",
                args.join(" "),
                text(&out.stderr),
                text(&out.stdout)
            )
            .trim()
            .to_string()
        })
}
