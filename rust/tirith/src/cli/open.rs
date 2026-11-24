//! `tirith open`: the page, opened as the owner.
use serde_json::json;

/// A link that works once, from the daemon.
fn link() -> Result<String, String> {
    let said = crate::client::call_json("POST", "/api/page", Some(json!({})))?;
    said["url"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| "the daemon gave no link".into())
}

/// `tirith open link`: the link itself (the desktop app, a server over SSH).
pub(super) fn print() -> Result<(), String> {
    link().map(|url| println!("{url}"))
}

/// Opens the page here. The browser gets a private file that goes on to the link: a command line is everyone's to read.
pub(super) fn open() -> Result<(), String> {
    let url = link()?;
    let remote = std::env::var_os("SSH_CONNECTION").is_some()
        || (cfg!(target_os = "linux")
            && std::env::var_os("DISPLAY").is_none()
            && std::env::var_os("WAYLAND_DISPLAY").is_none());
    if !remote && shown(&url) {
        println!("opened OpenBot's page");
    } else {
        println!("open this within 10 minutes (it works once):\n{url}");
    }
    Ok(())
}

fn shown(url: &str) -> bool {
    let file = crate::config::tirith_home().join(crate::app::PAGE);
    let html = format!(
        "<!doctype html><meta http-equiv=\"refresh\" content=\"0;url={url}\"><title>OpenBot</title>"
    );
    crate::app::write_key(&file, &html).is_ok() && crate::os::open(file.as_os_str())
}
