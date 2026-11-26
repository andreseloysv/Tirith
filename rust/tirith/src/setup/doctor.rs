//! `neobot doctor`: every part this machine needs, each with its fix.
use super::{context::Context, daemon, find, needs, signin};

pub(super) fn doctor() -> Result<bool, String> {
    let cx = Context::new(true)?;
    let mut ok = true;
    let mut line = |good: bool, what: &str, fix: &str| {
        ok &= good;
        let mark = if good { "✓" } else { "✗" };
        let fix = if good || fix.is_empty() {
            String::new()
        } else {
            format!(": {fix}")
        };
        println!("{mark} {what}{fix}");
    };
    let gone = needs::missing();
    line(
        gone.is_empty(),
        needs::PROGRAMS_LABEL,
        &format!("{} missing", gone.join(" ")),
    );
    line(
        needs::git_identity(),
        "a git identity for commits",
        "git config --global user.name / user.email",
    );
    let up = daemon::answering().is_some_and(|v| daemon::is_of(&v, &cx.home));
    line(up, "the daemon answers", "neobot service start");
    if up {
        let off = needs::sandbox_off();
        line(
            off.is_none(),
            "the network sandbox",
            off.as_deref().unwrap_or(""),
        );
    }
    let cfg = cx.cfg();
    super::server_line::line(&cfg, &cx.home, up, &mut line);
    if let Some(error) = &cfg.error {
        line(false, "config.toml", error);
    }
    let mut any = false;
    for known in super::catalog::all() {
        let Some(command) = cfg.factory.command(&known.alias) else {
            continue;
        };
        let Some(path) = find::program(&command) else {
            // Codex has a command without any config; it counts only where it is installed or set
            let set = crate::config::get::<String>(&cfg.raw, &["agents", &known.alias, "command"]);
            if set.is_some() {
                line(
                    false,
                    &known.name,
                    &format!("{command} does not run: neobot connect {}", known.alias),
                );
            }
            continue;
        };
        any = true;
        let fix = format!("neobot connect {}", known.alias);
        let signed = signin::signed_in(&cx, &known, &path);
        let what = match signed {
            Some(_) => "is signed in",
            None => "is installed (only a real turn tells its sign-in: neobot connect)",
        };
        line(
            signed != Some(false),
            &format!("{} {what}", known.name),
            &fix,
        );
        let hooked = super::hook_lines::hooked(&cx, &known);
        line(hooked, &format!("{}'s hooks call NeoBot", known.name), &fix);
    }
    line(any, "an agent is connected", "neobot setup");
    for role in cfg.factory.roles() {
        let Some(agent) = cfg.factory.role::<String>(&role, "agent") else {
            continue;
        };
        let runs = cfg
            .factory
            .command(&agent)
            .as_deref()
            .and_then(find::program)
            .is_some();
        if role != super::prove::ROLE {
            line(
                runs,
                &format!("role {role} runs on {agent}"),
                "neobot setup",
            );
        }
    }
    Ok(ok)
}
