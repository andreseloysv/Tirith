//! `tirith-rs`: the Rust `tirith` while the Python one is live. Same commands, same files, so it never
//! shadows `tirith`. Only `collect` exists so far; see `docs/RUST-PORT.md`.

use std::path::PathBuf;
use std::process::ExitCode;
use tirith_core::collect::Collector;
use tirith_core::home::tirith_home;

const USAGE: &str = "usage: tirith-rs [--home DIR] collect --once";

fn main() -> ExitCode {
    let mut home: Option<PathBuf> = None;
    let mut command: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--home" => match args.next() {
                Some(dir) => home = Some(PathBuf::from(dir)),
                None => return usage("--home needs a folder"),
            },
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => match other.strip_prefix("--home=") {
                Some(dir) => home = Some(PathBuf::from(dir)),
                None => command.push(arg),
            },
        }
    }
    let home = tirith_home(home.as_deref());
    match command.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["collect", "--once"] => match Collector::new(&home).run_once() {
            Ok(_) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("tirith-rs collect: {err}");
                ExitCode::FAILURE
            }
        },
        ["collect"] => usage("only `collect --once` is ported so far"),
        [] => usage("no command"),
        [other, ..] => usage(&format!("not ported yet: {other}")),
    }
}

fn usage(message: &str) -> ExitCode {
    eprintln!("tirith-rs: {message}\n{USAGE}");
    ExitCode::from(2)
}
