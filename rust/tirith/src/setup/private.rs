//! What never reaches a log, an event or an answer: the values kept for the agents (`auth.rs`), their tokens and an enterprise's GitHub host.
use regex::Regex;
use std::path::Path;

const HOST: &str = "[private host]";
const SECRET: &str = "[redacted]";
/// The public one is no secret.
pub(super) const PUBLIC: &str = "github.com";

#[derive(Default)]
pub struct Hidden {
    secrets: Vec<String>,
    /// The host with whatever precedes it, and an enterprise's user names (`name_<its first label>`).
    hosts: Vec<(Regex, String)>,
}

impl Hidden {
    /// What to hide for `vars`: a variable that names a host (`…_HOST`), else a secret.
    pub fn of(vars: &[(String, String)]) -> Hidden {
        let mut hidden = Hidden::default();
        for (var, value) in vars {
            if var.ends_with("_HOST") {
                hidden.host(value);
            } else if value.len() >= 12 {
                hidden.secrets.push(value.clone());
            }
        }
        hidden
    }

    /// What to hide for everything kept in this Tirith home.
    pub fn in_home(home: &Path) -> Hidden {
        let files = std::fs::read_dir(home.join("auth")).into_iter().flatten();
        let mut vars = vec![];
        for file in files.flatten() {
            let path = file.path();
            let alias = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if path.extension().is_some_and(|e| e == "env") {
                vars.extend(super::auth::env(home, alias));
            }
        }
        Hidden::of(&vars)
    }

    fn host(&mut self, host: &str) {
        if host.len() < 4 || host.eq_ignore_ascii_case(PUBLIC) {
            return;
        }
        let mut add = |shape: String, shown: String| {
            let made = Regex::new(&shape).map(|re| (re, shown));
            self.hosts.extend(made.ok());
        };
        add(format!("(?i){}", regex::escape(host)), HOST.into());
        let label = regex::escape(host.split('.').next().unwrap_or_default());
        add(format!(r"(?i)_{label}\b"), format!("_{HOST}"));
    }

    /// True when nothing is private.
    pub fn none(&self) -> bool {
        self.secrets.is_empty() && self.hosts.is_empty()
    }

    pub fn hide(&self, text: &str) -> String {
        let mut out = text.to_string();
        for secret in &self.secrets {
            out = out.replace(secret, SECRET);
        }
        for (shape, shown) in &self.hosts {
            out = shape.replace_all(&out, shown.as_str()).into_owned();
        }
        out
    }
}
