//! Putting a process NeoBot starts inside the sandbox.
use super::{env::sandboxed_env, launch::Launch, owner::owner, profile::enabled, token};
use crate::app::Shared;

/// Wraps `argv` for a process NeoBot starts on behalf of `who` (a lane id, `<role>@<conversation>`, or "merge:<lane>"): confined (`confine.rs`) with the proxy environment and the PATH with the tool wrappers first; confined or not, with its token in `TIRITH_TOKEN`.
pub fn wrap(app: &Shared, who: &str, argv: Vec<String>) -> Result<Launch, String> {
    if let Some(why) = super::closed::refusal(app, who) {
        let (lane, project) = owner(app, who);
        app.event(&lane, &project, "net-off", "sandbox", &why, "");
        return Err(why);
    }
    let mut it = argv.into_iter();
    let (program, args): (String, Vec<String>) = (it.next().unwrap_or_default(), it.collect());
    let caller = crate::lab::proxy_caller(who); // a replay's turn reaches nothing outside
    let (alias, signed) = crate::setup::for_agent(app, &program); // a token of this agent's own (neobot connect --token)
    let signed = crate::modeldoor::given(app, &caller, &alias, signed); // its provider key stays with the daemon
    if let Err(why) = enabled(app) {
        let (program, args) = super::quiet::argv(&signed, program, args);
        let mut launch = unsandboxed(app, who, &why, program, args);
        launch.env.extend(signed);
        let token = token::issued(app, &caller);
        launch.env.push((crate::client::VAR.into(), token));
        return Ok(launch);
    }
    let mut env = sandboxed_env(app, &caller);
    let (program, args) = super::confine::argv(app, program, args);
    let (program, args) = super::quiet::argv(&signed, program, args); // what it writes never shows them
    env.extend(signed);
    Ok(Launch { program, args, env })
}

/// The process as given, with an event saying why it runs without the sandbox.
fn unsandboxed(app: &Shared, who: &str, why: &str, program: String, args: Vec<String>) -> Launch {
    let (lane, project) = owner(app, who);
    app.event(
        &lane,
        &project,
        "net-off",
        "sandbox",
        &format!("{lane} runs without the network sandbox: {why}"),
        "",
    );
    Launch {
        program,
        args,
        env: vec![],
    }
}
