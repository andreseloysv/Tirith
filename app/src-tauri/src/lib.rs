//! The desktop app is a window on the local `neobot serve` daemon: it starts the daemon when nothing
//! answers on its port and then shows the page the daemon serves. All state and actions live in the daemon.

#[cfg(unix)]
mod cli_link;
#[cfg(windows)]
mod cli_path;
mod daemon;
#[cfg(windows)]
mod guide;
mod page;

use daemon::{ensure_daemon, port, tirith_bin};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let port = port();
            #[cfg(unix)] // not in development: there the binary beside the app is a build, not a bundle
            if let Some(Err(e)) = daemon::beside().filter(|_| !cfg!(dev)).map(|bin| cli_link::link(&bin, &daemon::home_dir())) {
                eprintln!("neobot is not linked in ~/.local/bin: {e}");
            }
            #[cfg(windows)] // the same, for Windows: the app's folder on the user's PATH
            if let Some(Err(e)) = daemon::beside().filter(|_| !cfg!(dev)).and_then(|b| b.parent().map(|d| { cli_path::alias(d); cli_path::add(d) })) {
                eprintln!("neobot is not on PATH: {e}");
            }
            ensure_daemon(port);
            #[cfg(windows)] // no agent connected yet: the guide opens beside the window
            if !cfg!(dev) {
                guide::offer(&daemon::tirith_home(), &tirith_bin());
            }
            let url = page::link(port, &tirith_bin()).parse().expect("valid daemon url");
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::External(url))
                .title("NeoBot")
                .inner_size(1440.0, 900.0)
                .disable_drag_drop_handler()
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the NeoBot app");
}
