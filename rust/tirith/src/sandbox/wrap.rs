//! Putting a process Tirith starts inside the sandbox.
use super::{
    env::sandboxed_env,
    launch::Launch,
    owner::owner,
    profile::{enabled, PROFILE, SANDBOX_EXEC},
};
use crate::app::Shared;

/// Wraps `argv` for a process Tirith starts on behalf of `who` (a lane id, a role such as "manager" or "ceo", or "merge:<lane>"): `sandbox-exec -p <profile> argv…` with the proxy environment and the PATH with the tool wrappers first.
pub fn wrap(app: &Shared, who: &str, argv: Vec<String>) -> Launch {
    let mut it = argv.into_iter();
    let (program, args): (String, Vec<String>) = (it.next().unwrap_or_default(), it.collect());
    if let Err(why) = enabled(app) {
        return unsandboxed(app, who, &why, program, args);
    }
    let env = sandboxed_env(app, who);
    Launch {
        program: SANDBOX_EXEC.into(),
        args: ["-p".to_string(), PROFILE.to_string(), program]
            .into_iter()
            .chain(args)
            .collect(),
        env,
    }
}

/// The process as given, with an event saying why it runs without the sandbox.
fn unsandboxed(app: &Shared, who: &str, why: &str, program: String, args: Vec<String>) -> Launch {
    let (lane, project) = owner(app, who);
    app.event(
        &lane,
        &project,
        "net-off",
        "sandbox",
        &format!("{who} runs without the network sandbox: {why}"),
        "",
    );
    Launch {
        program,
        args,
        env: vec![],
    }
}
