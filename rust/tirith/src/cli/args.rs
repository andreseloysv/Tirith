//! Reading the arguments: a flag's value, the project a command is about, and the positional words.
use crate::client::Flag;

/// The value after `--name`.
pub(super) fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// The project a command is about: `--project`, else the project of the session NeoBot started it in.
pub(super) fn project(flag: Flag) -> String {
    flag("--project")
        .or_else(|| std::env::var("TIRITH_PROJECT").ok())
        .unwrap_or_default()
}

/// The words that are no flag nor a flag's value.
pub(super) fn positional(args: &[String]) -> Vec<&str> {
    let (mut pos, mut rest): (Vec<&str>, _) = (vec![], args.iter());
    while let Some(a) = rest.next() {
        if a.starts_with("--") {
            rest.next(); // a flag and its value
        } else {
            pos.push(a);
        }
    }
    pos
}
