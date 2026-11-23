//! Which browser the agents act in, as the owner chose by talking to a bot (`tirith browser use openbot|safari`):
//! OpenBot's own (`openbot`: nothing to give macOS, they sign in once in its window, `tirith browser login <url>`)
//! or their Safari (`safari`: their own sessions; the first click asks macOS for Accessibility). Kept in kv
//! `browser`; OpenBot's own until they choose.
use super::script::chat_role;
use crate::app::Shared;

/// How the browser commands are written.
pub const USAGE: &str =
    "tirith browser | tirith browser use openbot|safari | tirith browser login <url>";

/// The browser the agents act in.
pub fn chosen(app: &Shared) -> &'static str {
    match app.kv("browser").as_deref() {
        Some("safari") => "safari",
        _ => "openbot",
    }
}

/// What an agent does in each browser, in one line.
pub fn how(which: &str) -> &'static str {
    match which {
        "safari" => "safari: act in the owner's Safari with tirith safari open|read|click|type; a sign-in is theirs, in their Safari: open the page with tirith safari open <url> and give them its link",
        _ => "openbot: act in OpenBot's own browser with your browser tools; a sign-in is theirs, in a window you show them with tirith browser login <url>",
    }
}

/// `tirith browser` (which one, and how), `use openbot|safari` (the owner's choice) and `login <url>`.
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
        ("use", "openbot" | "safari") => {
            app.set_kv("browser", Some(which));
            Ok(format!("from now on the agents act in {}", how(which)))
        }
        ("login", _) => super::login::open(&app.home, url),
        _ => Err(USAGE.into()),
    }
}
