//! The command line that starts one exec turn, and the prompt file Muse reads.
use super::{argv, setup::Setup};
use crate::{app::Shared, models};
use std::path::PathBuf;

/// The agent's argv, and the prompt file it reads when it takes the message from a file (Muse). A fresh Muse session has never seen the role's instructions: they ride in its first message.
pub(super) fn command_line(
    app: &Shared,
    setup: &Setup,
    model: (Option<&str>, Option<&str>),
    resume: Option<&str>,
    prompt: (&str, &str),
) -> Result<(Vec<String>, Option<PathBuf>), String> {
    let (instructions, input) = prompt;
    let command = &setup.agent.command;
    if !setup.agent.muse {
        let browser = models::browser_args_for(app, "codex", false);
        let cfg = app.cfg();
        let argv = argv::codex_argv(
            command,
            setup.read_only,
            model,
            &cfg,
            instructions,
            resume,
            &browser,
        );
        return Ok((argv, None));
    }
    let file = setup.dir.join(".turn-prompt.txt");
    let _ = std::fs::create_dir_all(&setup.dir);
    let text = if resume.is_some() {
        input.to_string()
    } else {
        format!("{instructions}\n\n{input}")
    };
    std::fs::write(&file, text).map_err(|e| e.to_string())?;
    let argv = argv::muse_argv(
        command,
        setup.read_only,
        model,
        &setup.dir.display().to_string(),
        resume,
        &file.display().to_string(),
        &models::browser_args_for(app, "muse", false),
    );
    Ok((argv, Some(file)))
}
