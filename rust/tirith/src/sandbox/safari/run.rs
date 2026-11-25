//! Running the Safari script outside the sandbox, and the browser the owner chose (`choice`).
use super::act::{act_args, show_script};
use super::script::{chat_role, safari_script};
use crate::app::Shared;

/// Runs `safari_script` (open, read), the fixed Accessibility script (click, type, find, wait) or a browser command
/// (browser, use, login); a read's page text is cut at 20,000 characters.
pub async fn safari(
    app: &Shared,
    lane: &str,
    role: &str,
    cmd: &str,
    url: &str,
    target: &str,
    text: &str,
) -> Result<String, String> {
    match cmd {
        "click" | "type" | "find" | "wait" => act(app, lane, role, cmd, url, target, text).await,
        "browser" | "use" | "login" => super::choice::browser(app, lane, role, cmd, target, url),
        _ => {
            let script = safari_script(app, lane, role, cmd, url)?;
            let out = osascript(&["-e".into(), script]).await?;
            Ok(match cmd {
                "open" => format!("opened {url}"),
                _ => super::thin::hinted(cut_page(&out)),
            })
        }
    }
}

/// A click, typing, `find` or `wait`: the tab brought forward when a URL names it, then the fixed script; its `ERROR …` is an error.
async fn act(
    app: &Shared,
    lane: &str,
    role: &str,
    cmd: &str,
    url: &str,
    target: &str,
    text: &str,
) -> Result<String, String> {
    chat_role(app, lane, role)?;
    let (target, url) = super::thin::asked(cmd, target, url);
    let args = act_args(cmd, target, text, url)?;
    if !url.is_empty() {
        let shown = osascript(&["-e".into(), show_script(url)]).await;
        // a page still on its way has no tab at that address yet: `wait` is what looks again
        shown
            .map(drop)
            .or_else(|e| (cmd == "wait").then_some(()).ok_or(e))?;
    }
    let out = osascript(&args).await?;
    match out.strip_prefix("ERROR ") {
        Some(e) => Err(format!("Safari: {e}")),
        None => Ok(out),
    }
}

/// osascript's output for `args`, or its error as `Safari: …`.
async fn osascript(args: &[String]) -> Result<String, String> {
    let out = tokio::process::Command::new("/usr/bin/osascript")
        .args(args)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(match out.status.success() {
        true => &out.stdout,
        false => &out.stderr,
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
pub(super) fn cut_page(text: &str) -> String {
    let cut = |s: &str| s.chars().take(20_000).collect::<String>();
    text.split_once('\n')
        .and_then(|(u, rest)| {
            rest.split_once('\n')
                .map(|(t, body)| format!("{u}\n{t}\n{}", cut(body)))
        })
        .unwrap_or_else(|| cut(text))
}
