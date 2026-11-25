//! Which browser the agents act in, as the owner chose by talking to a bot (`neobot browser use <name>`):
//! NeoBot's own (`neobot`: nothing to give the system, they sign in once in its window, `neobot browser login
//! <url>`) or the owner's own, with their own sessions: Safari on a Mac (`safari`: the first click asks macOS
//! for Accessibility), Chrome or Edge on Windows (`chrome`, `edge`). Kept in kv `browser`; NeoBot's own until
//! they choose.
use super::script::chat_role;
use crate::app::Shared;

/// The owner's own browsers this system can act in.
pub const OWN: &[&str] = if cfg!(windows) {
    &["chrome", "edge"]
} else {
    &["safari"]
};

/// How the browser commands are written here.
pub fn usage() -> String {
    let own = OWN.join("|");
    format!("neobot browser | neobot browser use neobot|{own} | neobot browser login <url>")
}

/// The browser the agents act in.
pub fn chosen(app: &Shared) -> &'static str {
    let kept = app.kv("browser").unwrap_or_default();
    OWN.iter().copied().find(|n| *n == kept).unwrap_or("neobot")
}

/// What an agent does in each browser, in one line: an owner's own one by its name, anything else NeoBot's own.
pub fn how(which: &str) -> String {
    let name = match (which, super::uia::browser(which)) {
        ("safari", _) => "Safari",
        (_, Some(known)) => known.1,
        _ => return "neobot: act in NeoBot's own browser with your browser tools; a sign-in is theirs, in a window you show them with neobot browser login <url>".into(),
    };
    format!("{which}: act in the owner's {name} with neobot {which} open|read|find|click|type|wait; a sign-in is theirs, in their {name}: open the page with neobot {which} open <url> and give them its link")
}

/// `neobot browser` (which one, and how), `use <name>` (the owner's choice) and `login <url>`.
pub fn browser(
    app: &Shared,
    lane: &str,
    role: &str,
    cmd: &str,
    which: &str,
    url: &str,
) -> Result<String, String> {
    chat_role(app, lane, role)?;
    let which = if which == "openbot" { "neobot" } else { which };
    match (cmd, OWN.contains(&which) || which == "neobot") {
        ("browser", _) => Ok(how(chosen(app)) + &super::missing::note(app)),
        ("use", true) => {
            app.set_kv("browser", Some(which));
            Ok(format!("from now on the agents act in {}", how(which)))
        }
        ("login", _) => super::login::open(&app.home, url),
        _ => Err(usage()),
    }
}
