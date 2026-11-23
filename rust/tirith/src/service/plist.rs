//! The text of OpenBot's launchd job.
use super::unit::Unit;
use std::path::Path;

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn key(name: &str, value: &str) -> String {
    format!(
        "  <key>{name}</key>\n  <string>{}</string>\n",
        escaped(value)
    )
}

/// The job; `user` for one that starts at boot as that user.
pub(super) fn text(u: &Unit, log: &Path, user: Option<&str>) -> String {
    let show = |p: &Path| p.display().to_string();
    let mut body = key("Label", &u.label());
    body += &format!(
        "  <key>ProgramArguments</key>\n  <array>\n    <string>{}</string>\n    <string>serve</string>\n  </array>\n",
        escaped(&show(&u.exe))
    );
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let mut env = key("TIRITH_HOME", &show(&u.home));
    env += &key("HOME", &show(&u.user_home));
    env += &key("PATH", &u.path());
    env += &key("SHELL", &shell);
    if let Some(port) = u.port {
        env += &key("TIRITH_PORT", &port.to_string());
    }
    body += &format!("  <key>EnvironmentVariables</key>\n  <dict>\n{env}  </dict>\n");
    body += &key("WorkingDirectory", &show(&u.user_home));
    body += &key("StandardOutPath", &show(log));
    body += &key("StandardErrorPath", &show(log));
    body += &key("ProcessType", "Interactive"); // the agents' work is not throttled
    for on in ["RunAtLoad", "KeepAlive", "AbandonProcessGroup"] {
        body += &format!("  <key>{on}</key>\n  <true/>\n"); // (the last: a restart leaves running lanes alone)
    }
    body += "  <key>SoftResourceLimits</key>\n  <dict>\n    <key>NumberOfFiles</key>\n    <integer>10240</integer>\n  </dict>\n";
    match user {
        Some(user) => body += &key("UserName", user),
        // It loads over SSH too, with nobody at the screen.
        None => body += "  <key>LimitLoadToSessionType</key>\n  <array>\n    <string>Aqua</string>\n    <string>Background</string>\n  </array>\n",
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n{body}</dict>\n</plist>\n"
    )
}
