//! The command line of one exec turn, for each agent that runs one (Muse and Codex).

/// The argv of one Muse turn. It runs unattended inside Tirith's own sandbox, so its approvals and inner sandbox are off (`--yolo`, the parallel of the Codex bypass below); a read-only role's file-writing tools are off too.
pub(super) fn muse_argv(
    command: &str,
    read_only: bool,
    model: (Option<&str>, Option<&str>),
    workspace: &str,
    session: Option<&str>,
    prompt_file: &str,
    browser: &[String],
) -> Vec<String> {
    let (model, effort) = model;
    let mut a: Vec<String> = [
        command,
        "exec",
        "--json",
        "--yolo",
        "--workspace",
        workspace,
    ]
    .map(String::from)
    .to_vec();
    a.extend(
        browser
            .iter()
            .cloned()
            .chain(read_only.then(|| "--disable-write".to_string())),
    );
    a.extend(
        [
            ("--model", model),
            ("--reasoning-effort", effort),
            ("--session-id", session),
            ("--prompt-file", Some(prompt_file)),
        ]
        .into_iter()
        .filter_map(|(f, v)| v.map(|v| [f.to_string(), v.to_string()]))
        .flatten(),
    );
    a
}

pub(super) fn codex_argv(
    command: &str,
    read_only: bool,
    model: (Option<&str>, Option<&str>),
    cfg: &crate::config::Config,
    instructions: &str,
    resume: Option<&str>,
    browser: &[String],
) -> Vec<String> {
    let mut a: Vec<String> = [command, "exec", "--json", "--skip-git-repo-check"]
        .map(String::from)
        .to_vec();
    a.extend(if read_only {
        vec!["--sandbox".into(), "read-only".into(), "--ephemeral".into()]
    } else {
        vec!["--dangerously-bypass-approvals-and-sandbox".into()]
    });
    a.extend(
        browser.iter().cloned().chain(
            model
                .0
                .into_iter()
                .flat_map(|m| ["--model".to_string(), m.to_string()]),
        ),
    );
    super::options(cfg, &mut a, model.1);
    a.extend(
        [
            "-c".to_string(),
            format!(
                "developer_instructions={}",
                toml::Value::String(instructions.into())
            ),
        ]
        .into_iter()
        .chain(
            resume
                .into_iter()
                .flat_map(|id| ["resume".to_string(), id.to_string()]),
        )
        .chain(["-".to_string()]),
    );
    a
}

#[cfg(test)]
#[path = "../../checks/codex.rs"]
mod tests;
