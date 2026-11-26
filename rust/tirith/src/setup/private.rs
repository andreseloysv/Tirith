//! What never reaches a log, an event or an answer: the values kept for the agents (`auth.rs`), their tokens and an enterprise's GitHub host.
use super::kind::Kind;
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
    /// What to hide for `vars`: a variable that names a host (`…_HOST`) or holds a private URL, a secret (by its name or its look), nothing for a model id, a flag or a public provider's URL.
    pub fn of(vars: &[(String, String)]) -> Hidden {
        let mut hidden = Hidden::default();
        for (var, value) in vars {
            match super::kind::of(var, value) {
                Kind::Host(host) => hidden.host(&host),
                Kind::Secret => hidden.secrets.push(value.clone()),
                Kind::Open => {}
            }
        }
        hidden
    }

    /// What to hide for everything kept in this NeoBot home.
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
