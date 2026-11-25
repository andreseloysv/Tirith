//! The local daemon the window shows: where its binary is, its port, and starting it when nothing answers.

use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The binary's file name: `neobot`, `neobot.exe` on Windows (`tirith` is its alias).
const BIN: &str = if cfg!(windows) { "neobot.exe" } else { "neobot" };

/// HOME; on Windows, where it is seldom set, USERPROFILE (the daemon reads the same).
pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE").filter(|_| cfg!(windows)))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn tirith_home() -> PathBuf {
    std::env::var_os("TIRITH_HOME").map(PathBuf::from).unwrap_or_else(|| home_dir().join(".tirith"))
}

/// `TIRITH_PORT`, else `[server] port` of `config.toml`, else 7717 (the daemon reads the same places).
pub fn port() -> u16 {
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

/// `neobot` next to the app's executable: where a bundle carries it.
pub fn beside() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join(BIN)).filter(|p| p.is_file())
}

/// The `neobot` binary: next to the app's executable, then `~/.local/bin`, then `PATH`.
pub fn tirith_bin() -> PathBuf {
    let local = home_dir().join(".local/bin").join(BIN);
    beside().or_else(|| Some(local).filter(|p| p.is_file())).unwrap_or_else(|| "neobot".into())
}

/// A console program started from the window: on Windows without a console window of its own (the daemon's hidden one is shared by all it starts); elsewhere as is.
pub fn quiet(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }
    cmd
}

/// Starts the daemon in its own process group (it outlives the window) and waits up to 10 s for it.
pub fn ensure_daemon(port: u16) {
    if answers(port) {
        return;
    }
    let logs = tirith_home().join("logs");
    let _ = std::fs::create_dir_all(&logs);
    let log = std::fs::OpenOptions::new().create(true).append(true).open(logs.join("tirithd.log"));
    let mut cmd = Command::new(tirith_bin());
    crate::clean_env::clean(&mut cmd).arg("serve").stdin(Stdio::null());
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
    if quiet(&mut cmd).spawn().is_err() {
        return;
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !answers(port) {
        std::thread::sleep(Duration::from_millis(100));
    }
}
