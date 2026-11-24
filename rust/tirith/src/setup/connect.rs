//! Connecting one agent, in order: installed, signed in, its hooks at NeoBot, then one real turn as the proof.
use super::{catalog::Known, context::Context, hooks, host, program, signin, trust};

/// Connected, or what is still to do.
pub enum State {
    Connected,
    Pending(String),
}

pub fn connect(cx: &Context, known: &Known, token: bool) -> State {
    println!("\n== {}", known.name);
    let done = steps(cx, known, token);
    match &done {
        Ok(model) => println!(
            "  ✓ connected: it answered and NeoBot's rules heard its tool call{}",
            suffix(model)
        ),
        Err(why) => println!("  ✗ {why}"),
    }
    match done {
        Ok(_) => State::Connected,
        Err(why) => State::Pending(why),
    }
}

fn suffix(model: &str) -> String {
    if model.is_empty() {
        String::new()
    } else {
        format!(" ({model})")
    }
}

fn steps(cx: &Context, known: &Known, token: bool) -> Result<String, String> {
    host::choose(cx, known)?;
    if token {
        signin::token(cx, known)?;
    }
    let path = program::ready(cx, known)?;
    wire(cx, known, &path)?;
    signin::signed_then_checked(cx, known, &path)
}

/// Its command in config.toml and its hooks at this NeoBot.
pub(super) fn wire(cx: &Context, known: &Known, path: &std::path::Path) -> Result<(), String> {
    super::configure::agent(&cx.home, &known.alias, path, &known.settings)?;
    let file = cx.settings(known).ok_or_else(|| {
        format!(
            "where are {}'s settings? Put `settings = \"<file>\"` under [agents.{}] in {}/config.toml, then run neobot connect {}",
            known.name,
            known.alias,
            cx.home.display(),
            known.alias
        )
    })?;
    if hooks::install(known, &file, &cx.exe, cx.named_home())? {
        println!("  rules: its hooks call NeoBot now ({})", file.display());
    }
    if known.hooks == "codex" {
        let count = trust::trust(path)?;
        if count > 0 {
            println!("  rules: {count} of NeoBot's hooks trusted in Codex");
        }
    }
    Ok(())
}
