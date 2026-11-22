//! The git the daemon itself runs, confined like everything it starts: a repository agents write in (a bot's, a project's with its lanes) can make git run a program, a hook, an fsmonitor or a filter. That program is then an agent's: known at the daemon's door, its network the proxy's.
use super::{confine, env::sandboxed_env, launch::Launch, token::load_secret};
use crate::app::Shared;
use crate::config;
use std::process::Command;
use std::sync::OnceLock;

/// Who the proxy and its log know the daemon's git as.
const WHO: &str = "git";
/// How git starts inside, kept from the daemon's start: its callers hold no app, and one may hold the config's lock.
static INSIDE: OnceLock<Launch> = OnceLock::new();

/// Kept before any of the daemon's work runs git; nothing when the owner turned the sandbox off or the daemon is in one already.
pub fn prepare(app: &Shared) {
    let wanted = config::get::<bool>(&app.cfg().raw, &["network", "sandbox"]).unwrap_or(true);
    if !wanted || std::env::var("TIRITH_SANDBOXED").as_deref() == Ok("1") {
        return;
    }
    load_secret(app); // the proxy's password of `git` derives from it
    let env = sandboxed_env(app, WHO);
    let (program, args) = confine::argv(app, WHO.into(), vec![]);
    let _ = INSIDE.set(Launch { program, args, env });
}

/// A git command for the daemon to add its arguments to.
pub fn git() -> Command {
    let Some(inside) = INSIDE.get().filter(|_| confine::ready().is_ok()) else {
        return Command::new("git");
    };
    let mut command = Command::new(&inside.program);
    command
        .args(&inside.args)
        .envs(inside.env.iter().map(|(k, v)| (k, v)));
    command
}
