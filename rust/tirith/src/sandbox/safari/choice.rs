//! Which browser the agents act in, as the owner chose by talking to a bot (`neobot browser use neobot|safari`):
//! NeoBot's own (`neobot`: nothing to give macOS, they sign in once in its window, `neobot browser login <url>`)
//! or their Safari (`safari`: their own sessions; the first click asks macOS for Accessibility). Kept in kv
//! `browser`; NeoBot's own until they choose.
use super::script::chat_role;
use crate::app::Shared;

/// How the browser commands are written.
pub const USAGE: &str =
    "neobot browser | neobot browser use neobot|safari | neobot browser login <url>";

/// The browser the agents act in.
pub fn chosen(app: &Shared) -> &'static str {
    match app.kv("browser").as_deref() {
        Some("safari") => "safari",
        _ => "neobot",
    }
}

/// What an agent does in each browser, in one line.
pub fn how(which: &str) -> &'static str {
    match which {
        "safari" => "safari: act in the owner's Safari with neobot safari open|read|click|type; a sign-in is theirs, in their Safari: open the page with neobot safari open <url> and give them its link",
        _ => "neobot: act in NeoBot's own browser with your browser tools; a sign-in is theirs, in a window you show them with neobot browser login <url>",
    }
}

/// `neobot browser` (which one, and how), `use neobot|safari` (the owner's choice) and `login <url>`.
pub fn browser(
    app: &Shared,
    lane: &str,
    role: &str,
    cmd: &str,
    which: &str,
    url: &str,
) -> Result<String, String> {
    chat_role(app, lane, role)?;
    match (cmd, which) {
        ("browser", _) => Ok(how(chosen(app)).into()),
        ("use", "neobot" | "openbot" | "safari") => {
            let which = if which == "openbot" { "neobot" } else { which };
            app.set_kv("browser", Some(which));
            Ok(format!("from now on the agents act in {}", how(which)))
        }
        ("login", _) => super::login::open(&app.home, url),
        _ => Err(USAGE.into()),
    }
}
