//! An agent's command line flags from the factory's `[agents.<alias>]`, and setting them in an argv.
use crate::config::Config;
/// An agent command line: the alias it belongs to and the full argv (program first).
pub struct AgentCmd {
    pub alias: String,
    pub argv: Vec<String>,
}
/// Per alias: (model flag, effort flag, resume args, lane args) from the factory's `[agents.<alias>]`, with config.toml laid over it; an empty flag is no flag.
pub(super) fn flags(
    cfg: &Config,
    alias: &str,
) -> (Option<String>, Option<String>, Vec<String>, Vec<String>) {
    let key = |k: &str| {
        cfg.factory
            .agent::<String>(alias, k)
            .filter(|s| !s.is_empty())
    };
    (
        key("model_flag"),
        key("effort_flag"),
        cfg.factory.agent(alias, "resume_args").unwrap_or_default(),
        cfg.factory
            .agent::<Vec<String>>(alias, "args")
            .or_else(|| cfg.factory.agent(alias, "lane_args"))
            .unwrap_or_default(),
    )
}
/// Sets the model and the effort in argv with the agent's flags (or their usual spellings already there).
pub(super) fn set_model_and_effort(
    argv: &mut Vec<String>,
    (mflag, eflag): (&Option<String>, &Option<String>),
    model: &Option<String>,
    effort: &Option<String>,
) {
    set_flag(argv, mflag, &["--model", "-m"], model);
    set_flag(argv, eflag, &["--effort", "--reasoning-effort"], effort);
}
/// Sets `flag value` in argv: replaces the value of an existing `flag`/`alt` (also `flag=value`), else appends.
fn set_flag(argv: &mut Vec<String>, flag: &Option<String>, alts: &[&str], value: &Option<String>) {
    let (Some(flag), Some(value)) = (flag, value) else {
        return;
    };
    let is = |w: &str| w == flag || alts.contains(&w);
    if let Some(i) = argv
        .iter()
        .position(|w| is(w))
        .filter(|i| i + 1 < argv.len())
    {
        argv[i + 1] = value.clone();
    } else if let Some(w) = argv
        .iter_mut()
        .find(|w| w.split_once('=').is_some_and(|(f, _)| is(f)))
    {
        *w = format!("{}={value}", w.split_once('=').unwrap().0);
    } else {
        argv.extend([flag.clone(), value.clone()]);
    }
}
