//! `tirith setup`: the whole guide, from a bare machine to agents that really answer.
use super::connect::{connect, State};
use super::{choose, configure, context::Context, daemon, needs, opts::Opts, summary};
use crate::service;

pub(super) fn setup(o: &Opts) -> Result<bool, String> {
    let cx = Context::new(o.yes)?;
    println!("OpenBot {}", crate::bundle::VERSION);
    println!("Setting it up in {}.\n", cx.home.display());
    let mut pending = needs::machine(&cx);
    pending.iter().for_each(|p| println!("! {p}")); // at once, not only at the end
    super::trusted::keep(&cx.home)?;
    start(&cx, o, &mut pending)?;
    pending.extend(needs::sandbox(&cx));

    let mut connected = vec![];
    for known in choose::agents(&cx, o.agents.as_deref())? {
        match connect(&cx, &known, false) {
            State::Connected => connected.push(known.alias),
            State::Pending(why) => pending.push(why),
        }
    }
    if let Some(primary) = choose::primary(&cx, &connected, o.primary.as_deref()) {
        for (role, was) in configure::roles(&cx.home, &connected, &primary)? {
            println!("  {role}: runs on {primary} now ({was} is not connected here)");
        }
        let _ = daemon::answering(); // it reads the config again
    }
    summary::show(&cx, &connected, &pending);
    Ok(pending.is_empty() && !connected.is_empty())
}

/// The daemon runs, as a service with --server.
fn start(cx: &Context, o: &Opts, pending: &mut Vec<String>) -> Result<(), String> {
    let unit = service::Unit::here()?;
    if let Some(port) = o.port.filter(|p| *p != cx.cfg().port) {
        if daemon::answering().is_some_and(|v| daemon::is_of(&v, &cx.home)) {
            service::stop(&unit, &cx.ask)?; // before the port it answers on changes
        }
        configure::port(&cx.home, port)?;
    }
    let server = o.server
        || o.boot
        || cx.ask.yes_no(
            "Run OpenBot as a service of this machine (server mode: it starts by itself and stays up)?",
            cfg!(target_os = "linux"),
        ) && cx.ask.person();
    if !server {
        let older = daemon::answering()
            .filter(|v| daemon::is_of(v, &cx.home))
            .is_some_and(|v| v["version"] != crate::bundle::VERSION);
        if older {
            service::stop(&unit, &cx.ask)?; // installed over an older one: the new binary takes over
        }
        println!("OpenBot runs: {}.", service::start(&unit, &cx.ask)?.said());
        return Ok(());
    }
    let boot = o.boot
        || (cfg!(target_os = "macos")
            && cx.ask.yes_no(
                "Should it also run with nobody logged in on this Mac (needs sudo)?",
                false,
            ));
    let (kind, notes) = service::install(&unit, &cx.ask, boot)?;
    println!("OpenBot runs as {}.", kind.said());
    pending.extend(notes);
    Ok(())
}
