//! `tirith connect [<agent>]`: one agent connected, or the list of them.
use super::connect::{connect, State};
use super::{catalog, configure, context::Context, find, opts::Opts, signin};
use crate::service;

pub(super) fn run(o: &Opts) -> Result<bool, String> {
    let cx = Context::new(o.yes)?;
    let Some(alias) = o.words.first() else {
        list(&cx);
        return Ok(true);
    };
    let known = catalog::known(alias).ok_or_else(|| {
        let all: Vec<String> = catalog::all().into_iter().map(|k| k.alias).collect();
        format!("no agent '{alias}': {}", all.join(", "))
    })?;
    let unit = service::Unit::here()?;
    service::start(&unit, &cx.ask)?; // the check is the daemon's turn
    match connect(&cx, &known, o.token) {
        State::Pending(_) => Ok(false),
        State::Connected => {
            // what a role cannot run on here moves to an agent it can
            let cfg = cx.cfg();
            let agents: Vec<String> = cfg.factory.commands().into_iter().map(|(a, _)| a).collect();
            let usable: Vec<String> = agents
                .into_iter()
                .filter(|a| {
                    cfg.factory
                        .command(a)
                        .as_deref()
                        .and_then(find::program)
                        .is_some()
                })
                .collect();
            for (role, was) in configure::roles(&cx.home, &usable, &known.alias)? {
                println!(
                    "  {role}: runs on {} now ({was} is not connected here)",
                    known.alias
                );
            }
            let _ = super::daemon::answering();
            Ok(true)
        }
    }
}

fn list(cx: &Context) {
    let cfg = cx.cfg();
    for known in catalog::all() {
        let set = cfg.factory.command(&known.alias);
        let path = find::agent(&known, set.as_deref());
        let signed = path
            .as_deref()
            .and_then(|p| signin::signed_in(cx, &known, p));
        let state = match (&path, signed) {
            (None, _) => "not installed".to_string(),
            (Some(_), Some(false)) => "installed, not signed in".to_string(),
            (Some(p), Some(true)) => format!("signed in ({})", p.display()),
            (Some(p), None) => format!("installed ({})", p.display()),
        };
        println!("{:<10} {:<18} {state}", known.alias, known.name);
    }
    println!("\nConnect one:  tirith connect <agent>");
}
