//! `tirith browser login <url>`: OpenBot's own browser shown on the owner's screen at that page, with its own
//! profile (`<home>/browser/profile`), so they sign in once and the agents find the session there. It keeps the
//! keychain and password-store switches of the browser the agents drive (tools/browser/launch.mjs, through
//! Playwright), or the cookies it saves would not read there. LaunchServices starts it (`open -n`), so it is no
//! child of the daemon; when an agent holds the profile, Chromium shows the page in that window instead.
use std::path::Path;
use std::process::{Command, Stdio};

/// The browser application: Edge where it is installed, else Chrome.
fn application() -> Option<&'static str> {
    [
        "/Applications/Microsoft Edge.app",
        "/Applications/Google Chrome.app",
    ]
    .into_iter()
    .find(|p| Path::new(p).exists())
}

/// `open`'s arguments for the window: a new instance of `app` on the profile, at `url`.
pub fn login_args(app: &str, profile: &Path, url: &str) -> Vec<String> {
    let mut args: Vec<String> = ["-n", "-a", app, "--args"].map(String::from).to_vec();
    args.push(format!("--user-data-dir={}", profile.display()));
    for s in [
        "--use-mock-keychain",
        "--password-store=basic",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-sync",
        "--new-window",
    ] {
        args.push(s.into());
    }
    args.push(url.into());
    args
}

/// Shows the page; says what to tell the owner.
pub fn open(home: &Path, url: &str) -> Result<String, String> {
    let web = url.starts_with("https://") || url.starts_with("http://");
    let clean = !url.chars().any(|c| c.is_control() || c.is_whitespace());
    (web && clean)
        .then_some(())
        .ok_or("tirith browser login <url>: an http(s) address")?;
    let app = application().ok_or("neither Microsoft Edge nor Google Chrome is installed")?;
    let profile = home.join("browser/profile");
    std::fs::create_dir_all(&profile).map_err(|e| e.to_string())?;
    let done = Command::new("/usr/bin/open")
        .args(login_args(app, &profile, url))
        .stdin(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    done.success()
        .then(|| format!("shown in OpenBot's browser: {url} — ask the owner to sign in there and to close that window when done; the session stays saved"))
        .ok_or_else(|| "OpenBot's browser did not open".into())
}
