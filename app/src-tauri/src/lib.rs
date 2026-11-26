//! The desktop app is a window on the local `neobot serve` daemon: it starts the daemon when nothing
//! answers on its port and then shows the page the daemon serves. All state and actions live in the daemon.

mod clean_env;
#[cfg(unix)]
mod cli_link;
#[cfg(windows)]
mod cli_path;
mod daemon;
mod page;
mod team;

use daemon::{ensure_daemon, port, tirith_bin};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            team::commands::team_state,
            team::commands::team_add,
            team::commands::team_remove,
            team::commands::team_use,
            team::commands::team_sign_in
        ])
        .setup(|app| {
            team::menu(app.handle())?;
            #[cfg(unix)] // not in development: there the binary beside the app is a build, not a bundle
            if let Some(Err(e)) = daemon::beside().filter(|_| !cfg!(dev)).map(|bin| cli_link::link(&bin, &daemon::home_dir())) {
                eprintln!("neobot is not linked in ~/.local/bin: {e}");
            }
            #[cfg(windows)] // the same, for Windows: the app's folder on the user's PATH
            if let Some(Err(e)) = daemon::beside().filter(|_| !cfg!(dev)).and_then(|b| b.parent().map(|d| { cli_path::alias(d); cli_path::add(d) })) {
                eprintln!("neobot is not on PATH: {e}");
            }
            // A team server chosen: its page, no daemon of this Mac started or needed. Else this Mac's own.
            let url = team::start_url(app.handle()).unwrap_or_else(|| {
                let port = port();
                ensure_daemon(port); // no agent connected yet: the page itself sets them up, never a terminal
                page::link(port, &tirith_bin()).parse().expect("valid daemon url")
            });
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::External(url))
                .title(team::title(app.handle()))
                .inner_size(1440.0, 900.0)
                .disable_drag_drop_handler()
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the NeoBot app");
}
