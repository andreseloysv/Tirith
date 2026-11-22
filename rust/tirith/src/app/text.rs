//! Log text: its secrets hidden, its last lines.
use std::path::Path;

/// Hides whole secret values in log text: the value of any `*KEY*`/`*TOKEN*`/`*SECRET*`/`*PASSWORD*` variable (`NAME=value`, `NAME: value`, `"NAME":"value"`) and bare well-known key shapes. Nothing of the value is kept, not even its first characters.
pub fn redact_secrets(text: &str) -> String {
    use regex::Regex;
    use std::sync::LazyLock;
    static NAMED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)(\b[A-Za-z0-9_]*(?:KEY|TOKEN|SECRET|PASSWORD)[A-Za-z0-9_]*\\?["']?\s*[=:]\s*\\?["']?)([^\s"'\\,;}&]+)"#).unwrap()
    });
    static BARE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\bAIza[0-9A-Za-z_\-]{10,}|\bsk-[0-9A-Za-z_\-]{10,}|\bgh[pousr]_[0-9A-Za-z]{10,}|\bgithub_pat_[0-9A-Za-z_]{10,}|\bxox[abpr]-[0-9A-Za-z\-]{10,}|\bAKIA[0-9A-Z]{12,}").unwrap()
    });
    let step = NAMED.replace_all(text, "${1}[redacted]");
    BARE.replace_all(&step, "[redacted]").into_owned()
}

/// The last `lines` lines of a file.
pub fn tail(path: &Path, lines: usize) -> String {
    let text = std::fs::read(path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}
