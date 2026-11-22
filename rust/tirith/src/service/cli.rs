//! `tirith service install|uninstall|status`.
use super::{lifecycle, manager::Manager, status};

/// `tirith service …` commands; `None` when `pos` is not one of this module's commands.
pub fn cli(pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    Some(match pos {
        ["service", "install"] => owner_only().and_then(|()| lifecycle::install(&Manager::here()?)),
        ["service", "uninstall"] => {
            owner_only().and_then(|()| lifecycle::uninstall(&Manager::here()?))
        }
        ["service", "status"] => Manager::here().and_then(|m| status::status(&m)),
        ["service", ..] => Err("tirith service install|uninstall|status".into()),
        _ => return None,
    })
}

/// Both changes restart or stop the daemon, which would cut the session of the agent that asked.
fn owner_only() -> Result<(), String> {
    let agent = ["TIRITH_LANE", "TIRITH_ROLE"]
        .iter()
        .any(|k| std::env::var(k).is_ok_and(|v| !v.is_empty()));
    (!agent).then_some(()).ok_or_else(|| {
        "an agent does not install or remove the service (it restarts the daemon): the owner runs it in a terminal".into()
    })
}
