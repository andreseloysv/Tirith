//! Putting a process Tirith starts inside the sandbox.
use super::{env::sandboxed_env, launch::Launch, owner::owner, profile::enabled};
use crate::app::Shared;

/// Wraps `argv` for a process Tirith starts on behalf of `who` (a lane id, a role such as "manager" or "ceo", or "merge:<lane>"): confined (`confine.rs`) with the proxy environment and the PATH with the tool wrappers first.
pub fn wrap(app: &Shared, who: &str, argv: Vec<String>) -> Launch {
    let mut it = argv.into_iter();
    let (program, args): (String, Vec<String>) = (it.next().unwrap_or_default(), it.collect());
    let signed = crate::setup::for_program(app, &program); // a token of this agent's own (tirith connect --token)
    if let Err(why) = enabled(app) {
        let mut launch = unsandboxed(app, who, &why, program, args);
        launch.env.extend(signed);
        return launch;
    }
    let mut env = sandboxed_env(app, &crate::lab::proxy_caller(who)); // a replay's turn reaches nothing outside
    env.extend(signed);
    let (program, args) = super::confine::argv(app, program, args);
    Launch { program, args, env }
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
