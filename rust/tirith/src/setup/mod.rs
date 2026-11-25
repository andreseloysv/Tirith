//! The guide: `neobot setup`, `neobot connect`, `neobot doctor` (and `neobot service`, server mode, beside it). It takes a bare Mac or Linux machine to agents that are installed, signed in, held by the rules and proven with a real turn.
mod api;
mod as_user;
mod ask;
mod auth;
mod catalog;
mod choose;
mod configure;
mod connect;
mod context;
pub mod daemon;
mod doctor;
mod edit;
pub mod find;
mod guide;
mod hook_lines;
mod hooks;
mod host;
mod install;
mod login;
mod needs;
mod npm;
mod one;
mod opts;
mod packages;
mod private;
mod program;
mod prove;
pub mod rehook;
pub mod root;
pub mod run;
mod secret;
mod signin;
mod summary;
#[cfg(target_os = "linux")]
mod terminal;
mod trust;
mod trusted;
pub mod unhook;
mod web;
mod why;
mod winget;

pub use api::routes;
pub use as_user::{as_service_user, in_anothers_terminal};
pub use ask::Ask;
pub use auth::for_program;
pub use catalog::{known as known_agent, model_hosts, owner_of_model};
pub use hook_lines::others;
pub use opts::Opts;
pub use private::Hidden;
pub use secret::read as read_secret;
pub use web::lang::told as owner_language;

/// Whether this process is an agent's, by what NeoBot names it.
pub fn from_agent() -> bool {
    ["TIRITH_LANE", "TIRITH_ROLE"]
        .iter()
        .any(|k| std::env::var(k).is_ok_and(|v| !v.is_empty()))
}

/// These commands take options with no value, so they are read before the command line's `--flag value` pairs: the exit code, or none for another command.
pub fn cli(args: &[String]) -> Option<i32> {
    let (command, rest) = args.split_first()?;
    let command = command.as_str();
    matches!(command, "setup" | "connect" | "service" | "doctor").then_some(())?;
    if rest.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", opts::USE);
        return Some(0);
    }
    let looks = matches!(rest.first().map(String::as_str), Some("status" | "logs"));
    if from_agent() && command != "doctor" && !(command == "service" && looks) {
        eprintln!(
            "neobot {command} is the owner's: run it in your own terminal, not from an agent"
        );
        return Some(1);
    }
    let done = opts::parse(rest).and_then(|o| match command {
        "setup" => guide::setup(&o),
        "connect" => one::run(&o),
        "service" => crate::service::cli(&o),
        _ => doctor::doctor(),
    });
    Some(match done {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(why) => {
            eprintln!("{why}");
            1
        }
    })
}

#[cfg(test)]
#[path = "../../checks/setup.rs"]
mod tests;
