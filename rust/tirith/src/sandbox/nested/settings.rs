//! The limit that holds for one `codex exec`: what it asked, else what its settings say, else Codex's own default.
use super::{asked::Asked, layers::Layers, mode::Mode, roots::roots, told::said, Request};
use std::path::Path;

/// The proxy's variables: a limited Codex's commands get them empty, as they got no network.
const NO_NET: [&str; 7] = [
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "ALL_PROXY",
    "https_proxy",
    "http_proxy",
    "all_proxy",
    "GRADLE_OPTS",
];
const BYPASS: &str = "--dangerously-bypass-approvals-and-sandbox";
/// With its sandbox off there Codex would tell its model that it has full access.
const UNTOLD: &str = "include_permissions_instructions=false";
/// Settings of an administrator's, which are not read here.
const MANAGED: [&str; 2] = [
    "/etc/codex/managed_config.toml",
    "/etc/codex/requirements.toml",
];

/// Where Codex runs: its folder, its home and the caller's TMPDIR.
pub(super) struct Where<'a> {
    pub cwd: &'a Path,
    pub home: &'a Path,
    pub tmp: Option<&'a Path>,
    /// Tirith's own read-only role.
    pub role: bool,
}

/// The request for the daemon, with what is said about it; none for full access, which asks for no limit.
pub(super) fn limit(a: &Asked, at: &Where) -> Result<Option<(Request, String)>, String> {
    let dir =
        a.cd.as_deref()
            .map_or(at.cwd.to_path_buf(), |d| at.cwd.join(d));
    let set = Layers::of(a, at.home, &dir);
    let named = || {
        set.get("sandbox_mode")
            .and_then(|v| Mode::named(v.as_str()?))
    };
    // a permission profile or an administrator's settings are not read here: the strictest limit, never a looser one
    let unread = MANAGED.iter().any(|f| Path::new(f).exists())
        || a.mode.is_none() && (a.permissions || set.get("default_permissions").is_some());
    let default = if set.known_project {
        Mode::Write
    } else {
        Mode::ReadOnly
    };
    let mode = if unread {
        Mode::ReadOnly
    } else {
        a.mode.or_else(named).unwrap_or(default)
    };
    if mode == Mode::Full {
        return Ok(None);
    }
    if a.worktree {
        return Err("codex --worktree cannot keep its limit inside Tirith's sandbox: make the worktree first and pass -C <folder>".into());
    }
    let write = mode == Mode::Write;
    let net = write && set.is("sandbox_workspace_write.network_access");
    let mut args = [BYPASS, "-c", UNTOLD].map(String::from).to_vec();
    for name in NO_NET.iter().filter(|_| !net) {
        args.extend([
            "-c".into(),
            format!("shell_environment_policy.set.{name}=\"\""),
        ]);
    }
    let mut line = a.args.clone();
    line.splice(a.at..a.at, args);
    let request = Request {
        args: line,
        cwd: at.cwd.display().to_string(),
        write,
        roots: if write {
            roots(a, &set, &dir, at)
        } else {
            vec![]
        },
        state: at.home.display().to_string(),
        out: a
            .out
            .as_ref()
            .map(|o| at.cwd.join(o).display().to_string())
            .unwrap_or_default(),
        net,
        agent: at.role,
        ..Request::default()
    };
    let note = said(mode, net, unread, at.role, &set.stdio());
    Ok(Some((request, note)))
}
