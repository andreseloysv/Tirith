//! Codex's options as a command line is read: which take a value, and one option's name and value.

/// The options of `codex` and `codex exec` that take a value.
const VALUED: [&str; 19] = [
    "--config",
    "--enable",
    "--disable",
    "--remote",
    "--remote-auth-token-env",
    "--image",
    "--model",
    "--local-provider",
    "--profile",
    "--sandbox",
    "--cd",
    "--add-dir",
    "--ask-for-approval",
    "--thread-source",
    "--cyber-access-program",
    "--output-schema",
    "--color",
    "--output-last-message",
    "--permission-profile",
];
const SHORT: [(&str, &str); 10] = [
    ("-c", "--config"),
    ("-i", "--image"),
    ("-m", "--model"),
    ("-p", "--profile"),
    ("-s", "--sandbox"),
    ("-C", "--cd"),
    ("-a", "--ask-for-approval"),
    ("-o", "--output-last-message"),
    ("-P", "--permission-profile"),
    ("-h", "--help"),
];

/// One option: its long name, and the value written with it.
pub(super) fn option(word: &str) -> Option<(&str, Option<&str>)> {
    if word.starts_with("--") {
        return Some(
            word.split_once('=')
                .map_or((word, None), |(n, v)| (n, Some(v))),
        );
    }
    let short = word.get(..2).filter(|s| s.starts_with('-'))?;
    let long = SHORT
        .iter()
        .find(|(s, _)| *s == short)
        .map(|(_, long)| *long)?;
    let rest = &word[2..];
    let value = (!rest.is_empty()).then(|| rest.strip_prefix('=').unwrap_or(rest));
    Some((long, value))
}

pub(super) fn valued(name: &str) -> bool {
    VALUED.contains(&name)
}
