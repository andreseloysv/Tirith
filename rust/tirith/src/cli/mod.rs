//! The command line: its arguments, the command they name, and the commands that are more than one call.
mod args;
mod ask;
mod dispatch;
mod hook;
mod hooks;
mod plan;
mod serve;
mod status;

/// Runs the command of this process's arguments; a failed one prints why and exits 1.
pub fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("serve") {
        // A CLI command piped into `head` ends quietly instead of panicking on the closed pipe.
        unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
    }
    let flag = |name: &str| args::flag(&args, name);
    let project = args::project(&flag);
    let pos = args::positional(&args);
    if let Err(e) = dispatch::run(&pos, &flag, &project) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
