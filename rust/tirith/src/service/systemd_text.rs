//! The text of OpenBot's systemd user unit.
use super::unit::Unit;
use std::path::Path;

/// A value in a unit file, quoted: no specifier and no variable is read in it.
fn quoted(text: &str, runs: bool) -> Result<String, String> {
    let plain = !text.contains(['\n', '\r']);
    plain
        .then_some(())
        .ok_or_else(|| format!("a path with a line break cannot be a service: {text:?}"))?;
    let mut out = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%");
    if runs {
        out = out.replace('$', "$$");
    }
    Ok(format!("\"{out}\""))
}

pub(super) fn text(u: &Unit, log: &Path) -> Result<String, String> {
    let exe = quoted(&u.exe.display().to_string(), true)?;
    let home = quoted(&format!("TIRITH_HOME={}", u.home.display()), false)?;
    let path = quoted(&format!("PATH={}", u.path()), false)?;
    let log = log.display().to_string().replace('%', "%%");
    let port = u.port.map_or(String::new(), |p| {
        format!("Environment=\"TIRITH_PORT={p}\"\n")
    });
    Ok(format!(
        "# Written by `tirith service install`.
[Unit]
Description=OpenBot
After=network-online.target
StartLimitIntervalSec=0

[Service]
ExecStart={exe} serve
Environment={home}
Environment={path}
{port}Restart=always
RestartSec=3
# A restart of the daemon leaves the running lanes alone: it takes them up again.
KillMode=process
OOMPolicy=continue
LimitNOFILE=65536
StandardOutput=append:{log}
StandardError=append:{log}

[Install]
WantedBy=default.target
"
    ))
}
