//! The systemd user unit.
use super::spec::Spec;

/// `StartLimitIntervalSec=0` because systemd otherwise gives up after five quick failures, whatever `Restart` says. `KillMode=process` because the lanes' agents outlive a restart and the new daemon picks them up by pid.
pub(super) fn unit(spec: &Spec) -> String {
    let env: String = spec
        .env
        .iter()
        .map(|(k, v)| format!("Environment={}\n", quoted(&format!("{k}={v}"))))
        .collect();
    format!(
        "[Unit]
Description=Tirith daemon
StartLimitIntervalSec=0

[Service]
ExecStart={exe} serve
{env}Restart=always
RestartSec=5
KillMode=process
StandardOutput=append:{log}
StandardError=append:{log}

[Install]
WantedBy=default.target
",
        exe = quoted(&spec.exe),
        log = literal(&spec.log),
    )
}

/// `%` starts a specifier in every one of these settings.
fn literal(text: &str) -> String {
    text.replace('%', "%%")
}

fn quoted(text: &str) -> String {
    let escaped = literal(text).replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
