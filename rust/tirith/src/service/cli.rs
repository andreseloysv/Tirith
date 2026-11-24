//! `neobot service install [--boot] | status | start | stop | restart | uninstall | logs`.
use super::{Kind, Unit};
use crate::setup::{daemon, Ask, Opts};

const USE: &str =
    "neobot service install [--boot] | status | start | stop | restart | uninstall | logs";

pub fn cli(opts: &Opts) -> Result<bool, String> {
    let (u, ask) = (Unit::here()?, Ask::new(opts.yes));
    match opts.words.first().map(String::as_str) {
        Some("install") => {
            let (kind, notes) = super::install(&u, &ask, opts.boot)?;
            println!("NeoBot runs as {}.", kind.said());
            notes.iter().for_each(|n| println!("  {n}"));
        }
        Some("start") => println!("NeoBot runs: {}.", super::start(&u, &ask)?.said()),
        Some("stop") => {
            super::stop(&u, &ask)?;
            println!("NeoBot stopped. Lanes already running go on.");
        }
        Some("restart") => {
            super::stop(&u, &ask)?;
            println!("NeoBot runs again: {}.", super::start(&u, &ask)?.said());
        }
        Some("uninstall") => {
            super::uninstall(&u, &ask)?;
            println!("The service is gone. {} stays as it is.", u.home.display());
        }
        Some("status") => return Ok(status(&u, &ask)),
        Some("logs") => println!("{}", crate::app::tail(&u.log()?, 60)),
        _ => return Err(USE.into()),
    }
    Ok(true)
}

/// Prints how it runs; whether it answers.
fn status(u: &Unit, ask: &Ask) -> bool {
    let kind = Kind::installed(u);
    println!("service: {}", kind.said());
    let said = daemon::answering();
    let ours = said.as_ref().filter(|v| daemon::is_of(v, &u.home));
    match (ours, &said) {
        (Some(v), _) => println!(
            "daemon:  answers (pid {}, {})",
            v["pid"],
            v["version"].as_str().unwrap_or("")
        ),
        (None, Some(_)) => {
            println!("daemon:  not this home's: another NeoBot answers on this port")
        }
        (None, None) => println!("daemon:  does not answer"),
    }
    if kind != Kind::Plain {
        println!(
            "alive:   {}",
            super::alive::said(super::alive::pid(u, ask, kind), said.as_ref())
        );
    }
    ours.is_some()
}
