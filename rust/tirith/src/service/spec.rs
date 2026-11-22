//! What the service runs: this binary, for this Tirith home.
use std::path::{Path, PathBuf};

pub(super) struct Spec {
    pub exe: String,
    pub log: String,
    pub env: Vec<(&'static str, String)>,
}

impl Spec {
    /// This binary, this Tirith home and, when it is set, this `TIRITH_PORT`: a service inherits none of the shell's variables.
    pub fn current() -> Result<Spec, String> {
        let exe = std::env::current_exe()
            .map_err(|e| format!("cannot tell which binary this is: {e}"))?;
        let home = absolute(&crate::config::tirith_home())?;
        let mut env = vec![("TIRITH_HOME", text(&home)?)];
        env.extend(
            std::env::var("TIRITH_PORT")
                .ok()
                .filter(|port| port.parse::<u16>().is_ok())
                .map(|port| ("TIRITH_PORT", port)),
        );
        Ok(Spec {
            exe: text(&absolute(&exe)?)?,
            log: text(&home.join("logs/tirithd.log"))?,
            env,
        })
    }
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
    std::path::absolute(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// A path as a service file can carry it: UTF-8 on one line.
fn text(path: &Path) -> Result<String, String> {
    path.to_str()
        .filter(|p| !p.chars().any(char::is_control))
        .map(String::from)
        .ok_or_else(|| format!("a service file cannot name the path {path:?}"))
}
