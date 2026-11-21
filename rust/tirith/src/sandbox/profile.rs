//! The sandbox profile, the proxy's port and whether processes run inside the sandbox.
use crate::{app::Shared, config};

/// The sandbox profile: outbound network only to this machine (the daemon's port, the proxy and local sockets) and no Apple Events (macOS refuses them from `sandbox-exec` anyway; managers and bots read the owner's Safari through the daemon, see `safari_script`).
pub const PROFILE: &str = "(version 1)\n(allow default)\n(deny network-outbound)\n(deny appleevent-send)\n(allow network-outbound (remote ip \"localhost:*\"))\n(allow network-outbound (remote unix-socket))\n";
pub const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";
const DEFAULT_PORT: u16 = 47812;

pub fn proxy_port(app: &Shared) -> u16 {
    config::get(&app.cfg().raw, &["network", "proxy_port"]).unwrap_or(DEFAULT_PORT)
}

pub fn enabled(app: &Shared) -> Result<(), String> {
    config::get::<bool>(&app.cfg().raw, &["network", "sandbox"])
        .unwrap_or(true)
        .then_some(())
        .ok_or_else(|| "[network] sandbox = false in config.toml".to_string())?;
    (cfg!(target_os = "macos") && std::path::Path::new(SANDBOX_EXEC).exists())
        .then_some(())
        .ok_or_else(|| "the sandbox needs macOS (sandbox-exec)".to_string())?;
    (std::env::var("TIRITH_SANDBOXED").as_deref() != Ok("1")).then_some(()).ok_or_else(|| "the daemon itself runs in a sandbox and macOS cannot nest them (its own proxy settings stay)".to_string())?;
    Ok(())
}

/// Whether a process Tirith starts runs inside the network sandbox: `wrap` puts it there, or the daemon itself already runs in one (macOS cannot nest them, so its children stay in the outer one and keep its proxy env). `[network] sandbox = false` means neither.
pub fn sandboxed(app: &Shared) -> bool {
    config::get::<bool>(&app.cfg().raw, &["network", "sandbox"]).unwrap_or(true)
        && (std::env::var("TIRITH_SANDBOXED").as_deref() == Ok("1")
            || (cfg!(target_os = "macos") && std::path::Path::new(SANDBOX_EXEC).exists()))
}
