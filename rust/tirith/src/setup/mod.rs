//! The guide: `tirith setup`, `tirith connect`, `tirith doctor` (and `tirith service`, server mode, beside it). It takes a bare Mac or Linux machine to agents that are installed, signed in, held by the rules and proven with a real turn.
mod api;
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
mod install;
mod needs;
mod one;
mod opts;
mod packages;
mod program;
mod prove;
pub mod run;
mod secret;
mod signin;
mod summary;
mod trust;

pub use api::routes;
pub use ask::Ask;
pub use auth::for_program;
pub use hook_lines::others;
pub use opts::Opts;

/// These commands take options with no value, so they are read before the command line's `--flag value` pairs: the exit code, or none for another command.
pub fn cli(args: &[String]) -> Option<i32> {
    let (command, rest) = args.split_first()?;
    let command = command.as_str();
    matches!(command, "setup" | "connect" | "service" | "doctor").then_some(())?;
    if rest.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", opts::USE);
        return Some(0);
    }
    let agent = ["TIRITH_LANE", "TIRITH_ROLE"]
        .iter()
        .any(|k| std::env::var(k).is_ok_and(|v| !v.is_empty()));
    if agent && command != "doctor" {
        eprintln!(
            "tirith {command} is the owner's: run it in your own terminal, not from an agent"
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
