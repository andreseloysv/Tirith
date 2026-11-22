//! `tirith service install [--boot] | status | start | stop | restart | uninstall | logs`.
use super::{Kind, Unit};
use crate::setup::{daemon, Ask, Opts};

const USE: &str =
    "tirith service install [--boot] | status | start | stop | restart | uninstall | logs";

pub fn cli(opts: &Opts) -> Result<bool, String> {
    let (u, ask) = (Unit::here()?, Ask::new(opts.yes));
    match opts.words.first().map(String::as_str) {
        Some("install") => {
            let (kind, notes) = super::install(&u, &ask, opts.boot)?;
            println!("Tirith runs as {}.", kind.said());
            notes.iter().for_each(|n| println!("  {n}"));
        }
        Some("start") => println!("Tirith runs: {}.", super::start(&u, &ask)?.said()),
        Some("stop") => {
            super::stop(&u, &ask)?;
            println!("Tirith stopped. Lanes already running go on.");
        }
        Some("restart") => {
            super::stop(&u, &ask)?;
            println!("Tirith runs again: {}.", super::start(&u, &ask)?.said());
        }
        Some("uninstall") => {
            super::uninstall(&u, &ask)?;
            println!("The service is gone. {} stays as it is.", u.home.display());
        }
        Some("status") => return Ok(status(&u)),
        Some("logs") => println!("{}", crate::app::tail(&u.log()?, 60)),
        _ => return Err(USE.into()),
    }
    Ok(true)
}

/// Prints how it runs; whether it answers.
fn status(u: &Unit) -> bool {
    println!("service: {}", Kind::installed(u).said());
    let said = daemon::answering();
    let ours = said.as_ref().filter(|v| daemon::is_of(v, &u.home));
    match (ours, &said) {
        (Some(v), _) => println!(
            "daemon:  answers (pid {}, {})",
            v["pid"],
            v["version"].as_str().unwrap_or("")
        ),
        (None, Some(_)) => {
            println!("daemon:  not this home's: another Tirith answers on this port")
        }
        (None, None) => println!("daemon:  does not answer"),
    }
    ours.is_some()
}
