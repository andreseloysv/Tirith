//! The command line: its arguments, the command they name, and the commands that are more than one call.
mod args;
mod ask;
mod dispatch;
mod hook;
mod hooks;
mod hooks_token;
mod login_path;
mod open;
mod plan;
mod serve;
mod spent;
mod status;

/// Runs the command of this process's arguments; a failed one prints why and exits 1.
pub fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = crate::sandbox::launcher(&args) {
        std::process::exit(code);
    }
    crate::setup::as_service_user(&args);
    crate::setup::in_anothers_terminal();
    if let Some(code) = crate::setup::cli(&args) {
        std::process::exit(code);
    }
    if let Some(code) = crate::secrets::cli(&args) {
        std::process::exit(code);
    }
    if let Some("--version" | "-V" | "version") = args.first().map(String::as_str) {
        return println!("neobot {}", crate::bundle::VERSION);
    }
    #[cfg(unix)]
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
