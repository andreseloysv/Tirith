//! Access-audit tags of one tool call, recorded by the hook handler as `access` events.
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

static CRED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"[^\s'";|&<>]*/\.(?:ssh|aws|gnupg|kube|docker)(?:/[^\s'";|&<>]*)?|[^\s'";|&<>]*\.config/gh(?:/[^\s'";|&<>]*)?|(?:^|[\s/'"=])\.env(?:\.[\w-]+)*\b|[^\s'";|&<>]*\.(?:netrc|git-credentials|npmrc|pypirc)\b|\bsecurity\s+(?:find-[a-z-]+|dump-keychain)\b[^;&|\n]*"#).unwrap()
});
static GIT_PUSH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bgit\b[^;&|\n]*?\b(push\b[^;&|\n]*|remote\s+(?:add|set-url)\b[^;&|\n]*)").unwrap()
});
static GH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[;&|(\n]\s*|\bxargs\s+)gh\s+([a-z-]+(?:\s+[a-z-]+)?)").unwrap()
});
static ENV: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$\{?([A-Z][A-Z0-9_]{2,})|\bprintenv\s+([A-Z][A-Z0-9_]{2,})").unwrap()
});
const ENV_SKIP: [&str; 8] = [
    "PATH", "HOME", "USER", "SHELL", "PWD", "TMPDIR", "LANG", "TERM",
];

type Tag = (&'static str, String);

/// The items with repeats removed, in first-seen order.
fn unique<T: PartialEq>(items: impl IntoIterator<Item = T>) -> Vec<T> {
    items.into_iter().fold(vec![], |mut v, x| {
        let _ = (!v.contains(&x)).then(|| v.push(x));
        v
    })
}

/// `(category, target)` with category one of "credential", "git_push", "gh", "env" (target = variable names, never values).
pub fn tags(tool: &str, input: &Value) -> Vec<Tag> {
    if tool != "Bash" {
        return file_tags(input);
    }
    let cmd = input["command"].as_str().unwrap_or("");
    let mut out = credential_tags(cmd);
    out.extend(
        GIT_PUSH
            .captures_iter(cmd)
            .map(|c| ("git_push", format!("git {}", c[1].trim()))),
    );
    out.extend(
        GH.captures_iter(cmd)
            .map(|c| ("gh", format!("gh {}", c[1].trim()))),
    );
    out.extend(env_tag(cmd));
    out
}

/// The credential files a non-Bash tool call names.
fn file_tags(input: &Value) -> Vec<Tag> {
    input["file_paths"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .filter(|f| CRED.is_match(f) && !f.ends_with(".example"))
        .map(|f| ("credential", f.to_string()))
        .collect()
}

/// The credential files and keychain reads a command names.
fn credential_tags(cmd: &str) -> Vec<Tag> {
    unique(
        CRED.find_iter(cmd)
            .map(|m| {
                m.as_str()
                    .trim_start_matches([' ', '\'', '"', '='])
                    .trim()
                    .to_string()
            })
            .filter(|t| !t.ends_with(".example")),
    )
    .into_iter()
    .map(|t| ("credential", t))
    .collect()
}

/// The environment variables a command reads, by name.
fn env_tag(cmd: &str) -> Option<Tag> {
    let names: Vec<&str> = unique(
        ENV.captures_iter(cmd)
            .filter_map(|c| c.get(1).or(c.get(2)))
            .map(|m| m.as_str())
            .filter(|n| !ENV_SKIP.contains(n) && !n.starts_with("TIRITH")),
    );
    (!names.is_empty()).then(|| ("env", names.join(", ")))
}
