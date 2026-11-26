//! The command line that starts one exec turn, and how the agent gets its message and gives its usage.
use super::{argv, copilot::copilot_argv, kind::Exec, scratch::Scratch, setup::Setup};
use crate::{app::Shared, models};
use std::path::PathBuf;

/// One exec turn's process: its argv, what goes on its stdin, the file its usage arrives in, and its small files.
pub(super) struct Line {
    pub argv: Vec<String>,
    pub stdin: Option<String>,
    pub usage: Option<PathBuf>,
    pub scratch: Scratch,
}

/// The agent's command line. Muse reads its message from a file, Codex and Copilot on stdin. A fresh Muse or Copilot session has never seen the role's instructions: they ride in its first message.
pub(super) fn command_line(
    app: &Shared,
    setup: &Setup,
    model: (Option<&str>, Option<&str>),
    resume: Option<&str>,
    (instructions, input): (&str, &str),
) -> Result<Line, String> {
    let (agent, read_only, mut scratch) = (&setup.agent, setup.read_only, Scratch::default());
    let first = match resume {
        Some(_) => input.to_string(),
        None => format!("{instructions}\n\n{input}"),
    };
    let mut browser = models::browser_args_for(app, agent.kind.alias(), false);
    let gateway = models::gateway_args_for(app, agent.kind.alias(), &setup.role); // a bot's role alone
    browser.extend(gateway.iter().cloned());
    browser.extend(crate::lab::replay_args(app, agent.kind.alias()));
    let (argv, stdin, usage) = match agent.kind {
        Exec::Codex => {
            let cfg = app.cfg();
            let a = argv::codex_argv(
                &agent.command,
                read_only,
                model,
                &cfg,
                instructions,
                resume,
                &browser,
            );
            (a, Some(input.to_string()), None)
        }
        Exec::Copilot => {
            let usage = scratch.path(&setup.dir, "usage.json");
            let a = copilot_argv(&agent.command, read_only, model, resume, &usage, &gateway);
            (a, Some(first), Some(usage))
        }
        Exec::Muse => {
            let file = scratch.path(&setup.dir, "prompt.txt");
            std::fs::write(&file, first).map_err(|e| e.to_string())?;
            let a = argv::muse_argv(
                &agent.command,
                read_only,
                model,
                &setup.dir.display().to_string(),
                resume,
                &file.display().to_string(),
                &browser,
            );
            (a, None, None)
        }
    };
    Ok(Line {
        argv,
        stdin,
        usage,
        scratch,
    })
}
