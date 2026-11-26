//! What `neobot doctor` says of server mode (`[server] mode = "server"`): it fails closed, so a missing network sandbox is a machine that starts nothing.
use crate::config::{get, Config};

/// A line for server mode, when this home is a server's; none otherwise.
pub(super) fn line(cfg: &Config, up: bool, line: &mut impl FnMut(bool, &str, &str)) {
    let own_edge = get::<String>(&cfg.raw, &["browser", "use"]).as_deref() == Some("mine");
    if own_edge && cfg!(target_os = "linux") {
        let what = "[browser] use = \"mine\" (the owner's own Edge)";
        line(
            false,
            what,
            "this needs the Mac: take it out of config.toml",
        );
    }
    if get::<String>(&cfg.raw, &["server", "mode"]).as_deref() != Some("server") {
        return;
    }
    let said = up
        .then(|| crate::client::call_json("GET", "/api/net/status", None).ok())
        .flatten();
    let closed = said
        .as_ref()
        .and_then(|s| s["closed"].as_str().map(String::from));
    let what = "server mode: no lane or bot starts without the network sandbox";
    match (said, closed) {
        (None, _) => line(
            true,
            &format!("{what} (the daemon is not answering, so its sandbox is not checked)"),
            "",
        ),
        (Some(_), None) => line(true, what, ""),
        (Some(_), Some(why)) => line(false, what, why.trim_start_matches("server mode: ")),
    }
}
