//! The system browser, for the provider's sign-in page (never the app's own window: the provider's page may need
//! the person's saved passwords, passkeys and single sign-on, and RFC 8252 asks for the browser).
use std::process::Command;

pub fn open(url: &str) -> Result<(), String> {
    let mut cmd = if cfg!(target_os = "macos") {
        Command::new("open")
    } else if cfg!(windows) {
        let mut c = Command::new("rundll32"); // `start` would break on the & of the address
        c.arg("url.dll,FileProtocolHandler");
        c
    } else {
        Command::new("xdg-open")
    };
    cmd.arg(url).spawn().map(drop).map_err(|e| format!("cannot open the browser: {e}"))
}
