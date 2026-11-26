//! The app's sign-in without the window: `cargo run -p tirith-signin --example sign_in -- <server address> [--print]`.
//! It prints the provider's address it opens (`--print`: only prints it, opens nothing), waits for the browser, and
//! shows who the server says you are. For trying a server, or a provider, before anyone opens the app.
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (server, print) = (args.first().expect("a server address"), args.iter().any(|a| a == "--print"));
    let browse = |url: &str| -> Result<(), String> {
        println!("{url}");
        if print {
            return Err("--print: not opened".into());
        }
        let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        std::process::Command::new(opener).arg(url).status().map(drop).map_err(|e| e.to_string())
    };
    match tirith_signin::sign_in(server.trim_end_matches('/'), &browse, Duration::from_secs(300)) {
        Ok(s) => println!("signed in: {} ({}), link {}{}", s.name, s.role, server, s.link),
        Err(e) => eprintln!("not signed in: {e}"),
    }
}
