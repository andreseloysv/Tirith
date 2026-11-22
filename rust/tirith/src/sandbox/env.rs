//! The environment of a sandboxed process: the proxy for every tool that reads one, ssh through the proxy and the tool wrappers first on PATH.
use super::{jvm::gradle_opts, profile::proxy_port, token};
use crate::app::Shared;

const NO_PROXY: &str = "localhost,127.0.0.1,::1";
const PROXY_VARS: [&str; 6] = [
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "ALL_PROXY",
    "https_proxy",
    "http_proxy",
    "all_proxy",
];

/// Everything a process `who` gets inside the sandbox.
pub fn sandboxed_env(app: &Shared, who: &str) -> Vec<(String, String)> {
    let (port, user) = (proxy_port(app), token::user(who));
    let token = token::issued(app, who);
    let mut env = proxy_vars(&format!("http://{user}:{token}@127.0.0.1:{port}"));
    env.push((crate::client::VAR.into(), token.clone()));
    env.push((
        "GRADLE_OPTS".into(),
        gradle_opts(&port.to_string(), &user, &token),
    ));
    env.extend(ssh_command());
    env.extend(shell_paths(app));
    env
}

/// The proxy variables, the hosts that skip the proxy and the mark of a sandboxed process.
fn proxy_vars(url: &str) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = PROXY_VARS
        .iter()
        .map(|k| (k.to_string(), url.to_string()))
        .collect();
    env.extend(
        [
            ("NO_PROXY", NO_PROXY),
            ("no_proxy", NO_PROXY),
            ("NODE_USE_ENV_PROXY", "1"),
            ("TIRITH_SANDBOXED", "1"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string())),
    );
    env
}

/// ssh ignores proxy variables: git over ssh goes through the proxy with `tirith net tunnel`.
fn ssh_command() -> Option<(String, String)> {
    crate::app::exe()
        .filter(|_| std::env::var_os("GIT_SSH_COMMAND").is_none())
        .map(|exe| {
            let (own, exe) = (own_config(), exe.display());
            (
                "GIT_SSH_COMMAND".into(),
                format!("ssh{own} -o ProxyCommand='{exe} net tunnel %h %p'"),
            )
        })
}

/// In a user namespace root's files look like nobody's, and ssh refuses the system's included ones: it reads only the owner's config there.
fn own_config() -> String {
    let Some(()) = cfg!(target_os = "linux").then_some(()) else {
        return String::new();
    };
    let file = crate::config::home_dir().join(".ssh/config");
    let file = if file.is_file() {
        file.display().to_string()
    } else {
        "/dev/null".into()
    };
    format!(" -F '{file}'")
}

/// PATH with the tool wrappers first, and the zsh startup folder that puts them first again.
fn shell_paths(app: &Shared) -> [(String, String); 2] {
    [
        (
            "PATH".into(),
            format!(
                "{}:{}",
                app.home.join("net/bin").display(),
                app.child_path()
            ),
        ),
        (
            "ZDOTDIR".into(),
            app.home.join("net/zdot").display().to_string(),
        ),
    ]
}
