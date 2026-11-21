//! Running the Safari script outside the sandbox.
use super::script::safari_script;
use crate::app::Shared;

/// Runs `safari_script`; a read's page text is cut at 20,000 characters.
pub async fn safari(
    app: &Shared,
    lane: &str,
    role: &str,
    cmd: &str,
    url: &str,
) -> Result<String, String> {
    let script = safari_script(app, lane, role, cmd, url)?;
    let text = osascript(&script).await?;
    let Some(()) = (cmd != "open").then_some(()) else {
        return Ok(format!("opened {url}"));
    };
    Ok(cut_page(&text))
}

/// The script's output, or its error as `Safari: …`.
async fn osascript(script: &str) -> Result<String, String> {
    let out = tokio::process::Command::new("/usr/bin/osascript")
        .args(["-e", script])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(if out.status.success() {
        &out.stdout
    } else {
        &out.stderr
    })
    .trim_end()
    .to_string();
    out.status
        .success()
        .then_some(())
        .ok_or_else(|| format!("Safari: {}", text.trim()))?;
    Ok(text)
}

/// URL and title as they are, the page text cut at 20,000 characters.
fn cut_page(text: &str) -> String {
    let cut = |s: &str| s.chars().take(20_000).collect::<String>();
    text.split_once('\n')
        .and_then(|(u, rest)| {
            rest.split_once('\n')
                .map(|(t, body)| format!("{u}\n{t}\n{}", cut(body)))
        })
        .unwrap_or_else(|| cut(text))
}
