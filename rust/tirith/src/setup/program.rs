//! An agent's program on this machine: found, or installed now with its maker's installer.
use super::{catalog::Known, context::Context, find, install};
use std::path::PathBuf;

pub(super) fn ready(cx: &Context, known: &Known) -> Result<PathBuf, String> {
    let set = cx.cfg().factory.command(&known.alias);
    if let Some(path) = find::agent(known, set.as_deref()) {
        println!("  found {}", path.display());
        return Ok(path);
    }
    if known.bin.is_empty() {
        let hint = if cfg!(windows) {
            " (its full path, such as C:\\Users\\you\\.local\\bin\\agent.exe)"
        } else {
            ""
        };
        let said = cx.ask.text(
            &format!("  The command that starts the {}{hint}", known.name),
            "",
        );
        return find::program(&said).ok_or_else(|| {
            format!(
                "{} is not known here: install it, put `command = \"<it>\"` under [agents.{}] in {}/config.toml, then run neobot connect {}",
                known.name,
                known.alias,
                cx.home.display(),
                known.alias
            )
        });
    }
    let question = format!(
        "  {} is not installed. Install it now with its official installer ({})?",
        known.name,
        if cfg!(windows) {
            &known.install_ps1
        } else {
            &known.install
        }
    );
    let again = format!("install it, then run neobot connect {}", known.alias);
    if !cx.ask.yes_no(&question, true) {
        return Err(format!("{} is not installed: {again}", known.name));
    }
    println!(
        "  installing from {}",
        if cfg!(windows) {
            &known.install_ps1
        } else {
            &known.install
        }
    );
    install::install(&cx.ask, known, &cx.home).map_err(|e| format!("{e}: {again}"))?;
    let path = find::agent(known, None)
        .ok_or_else(|| format!("{} was installed but is not found: {again}", known.bin))?;
    println!("  installed {}", path.display());
    Ok(path)
}
