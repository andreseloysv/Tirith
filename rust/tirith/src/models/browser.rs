//! Where the browser MCP's server runs from: node, the launcher of this repo and its profile mode.
use crate::app::Shared;
use crate::config::get;
use std::path::Path;
/// The browser MCP server's (node, args) for a lane or another run; None without node or the launcher. See `browser_args_for` for how each agent carries it.
pub(super) fn browser_server(app: &Shared, lane: bool) -> Option<(String, Vec<String>)> {
    let launch = crate::bundle::launcher(&app.home)?;
    let node = node_bin(app)?;
    let mode = profile_mode(app, lane);
    Some((
        node,
        [launch.display().to_string()]
            .into_iter()
            .chain(mode.map(String::from))
            .collect(),
    ))
}
/// Whether NeoBot's own browser can run here at all (its helper is in this home and there is a Node).
pub fn own_browser(app: &Shared) -> bool {
    browser_server(app, false).is_some()
}
/// Lanes always `--isolated`; the others NeoBot's persistent profile, `--isolated` while another live session holds it, or `--extension` with `[browser] use = "mine"`.
fn profile_mode(app: &Shared, lane: bool) -> Option<&'static str> {
    let busy = profile_busy(app);
    (!lane && get::<String>(&app.cfg().raw, &["browser", "use"]).as_deref() == Some("mine"))
        .then_some("--extension")
        .or((lane || busy).then_some("--isolated"))
}
/// True while a live process holds `<home>/browser/profile` (its SingletonLock names host-pid).
fn profile_busy(app: &Shared) -> bool {
    std::fs::read_link(app.home.join("browser/profile/SingletonLock"))
        .unwrap_or_default()
        .to_string_lossy()
        .rsplit('-')
        .next()
        .and_then(|p| p.parse().ok())
        .is_some_and(crate::app::alive)
}
pub(super) fn add_browser(app: &Shared, argv: &mut Vec<String>, alias: &str, lane: bool) {
    let at = if alias == "muse" { argv.len() } else { 1 }; // Muse's -c only works after its subcommand, the others' flags belong right after the program
    drop(argv.splice(
        at..at,
        super::browser_args::browser_args_for(app, alias, lane),
    ));
}
/// Node for the browser launcher: on the children's PATH, else the newest `~/.nvm` install (the daemon's own PATH may have neither).
fn node_bin(app: &Shared) -> Option<String> {
    let on_path = app
        .child_path()
        .split(':')
        .map(|d| Path::new(d).join("node"))
        .find(|p| p.is_file());
    on_path
        .or_else(|| {
            std::fs::read_dir(crate::config::home_dir().join(".nvm/versions/node"))
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path().join("bin/node"))
                .filter(|p| p.is_file())
                .max()
        })
        .map(|p| p.display().to_string())
}
