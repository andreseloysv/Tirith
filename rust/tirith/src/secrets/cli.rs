//! `tirith secret set|list|remove`. It reads its own argv, where the shared reader would drop a `--value x` unseen: anything but the words it knows is refused, and never repeated.
use super::{input, names};
use crate::client::{call, call_json};
use serde_json::json;

const USE: &str =
    "tirith secret set <bot> <NAME>      the value comes on stdin, never as an argument
tirith secret list <bot>            the names, never the values
tirith secret remove <bot> <NAME>";

#[derive(Debug, PartialEq)]
pub(super) enum Command<'a> {
    Set(&'a str, &'a str),
    List(&'a str),
    Remove(&'a str, &'a str),
}

pub(super) fn parsed<'a>(words: &[&'a str]) -> Result<Command<'a>, String> {
    let command = match *words {
        ["set", bot, name] => Command::Set(bot, name),
        ["list", bot] => Command::List(bot),
        ["remove", bot, name] => Command::Remove(bot, name),
        _ => return Err(USE.into()),
    };
    let (Command::Set(bot, _) | Command::List(bot) | Command::Remove(bot, _)) = command;
    names::bot(bot)?;
    if let Command::Set(_, name) | Command::Remove(_, name) = command {
        names::secret(name)?;
    }
    if let Command::Set(_, name) = command {
        names::owners(name)?;
    }
    Ok(command)
}

/// The exit code, or none for another command.
pub fn cli(args: &[String]) -> Option<i32> {
    let (word, rest) = args.split_first()?;
    (word == "secret").then_some(())?;
    if rest.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USE}");
        return Some(0);
    }
    if crate::setup::from_agent() {
        eprintln!("tirith secret is the owner's: run it in your own terminal, not from an agent");
        return Some(1);
    }
    let words: Vec<&str> = rest.iter().map(String::as_str).collect();
    match parsed(&words).and_then(run) {
        Ok(()) => Some(0),
        Err(why) => {
            eprintln!("{why}");
            Some(1)
        }
    }
}

fn run(command: Command) -> Result<(), String> {
    match command {
        Command::Set(bot, name) => {
            let body = json!({"name": name, "value": input::read(name)?});
            call("POST", &format!("/api/secrets/{bot}"), Some(body))?;
            println!("{name} kept for {bot}");
        }
        Command::List(bot) => {
            let names = call_json("GET", &format!("/api/secrets/{bot}"), None)?;
            let names = names.as_array().into_iter().flatten();
            names
                .filter_map(|name| name.as_str())
                .for_each(|name| println!("{name}"));
        }
        Command::Remove(bot, name) => {
            call("DELETE", &format!("/api/secrets/{bot}/{name}"), None)?;
            println!("{name} removed from {bot}");
        }
    }
    Ok(())
}
