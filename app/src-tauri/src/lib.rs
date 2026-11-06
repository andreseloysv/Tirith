//! The desktop app is a window on the local `tirith serve` daemon: it starts the daemon when nothing
//! answers on its port and then shows the page the daemon serves. All state and actions live in the daemon.

use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

fn tirith_home() -> PathBuf {
    std::env::var_os("TIRITH_HOME").map(PathBuf::from).unwrap_or_else(|| home_dir().join(".tirith"))
}

/// `TIRITH_PORT`, else `[server] port` of `config.toml`, else 7717 (the daemon reads the same places).
fn port() -> u16 {
    if let Some(p) = std::env::var("TIRITH_PORT").ok().and_then(|v| v.parse().ok()) {
        return p;
    }
    std::fs::read_to_string(tirith_home().join("config.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
        .and_then(|cfg| cfg.get("server")?.get("port")?.as_integer())
        .and_then(|p| u16::try_from(p).ok())
        .unwrap_or(7717)
}

fn answers(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

/// The `tirith` binary: next to the app's executable, then `~/.local/bin`, then `PATH`.
fn tirith_bin() -> PathBuf {
    let beside = std::env::current_exe().ok().and_then(|exe| Some(exe.parent()?.join("tirith")));
    let local = home_dir().join(".local/bin/tirith");
    beside.filter(|p| p.is_file()).or_else(|| Some(local).filter(|p| p.is_file())).unwrap_or_else(|| "tirith".into())
}

/// Starts the daemon in its own process group (it outlives the window) and waits up to 10 s for it.
fn ensure_daemon(port: u16) {
    if answers(port) {
        return;
    }
    let logs = tirith_home().join("logs");
    let _ = std::fs::create_dir_all(&logs);
    let log = std::fs::OpenOptions::new().create(true).append(true).open(logs.join("tirithd.log"));
    let mut cmd = Command::new(tirith_bin());
    cmd.arg("serve").stdin(Stdio::null());
    if let Ok(file) = log {
        if let Ok(copy) = file.try_clone() {
            cmd.stdout(copy).stderr(file);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    if cmd.spawn().is_err() {
        return;
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !answers(port) {
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let port = port();
            ensure_daemon(port);
            let url = format!("http://127.0.0.1:{port}/").parse().expect("valid daemon url");
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::External(url))
                .title("Tirith")
                .inner_size(1440.0, 900.0)
                .disable_drag_drop_handler()
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the Tirith app");
}
