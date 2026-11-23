//! The text of the guard's hooks, with the words in it.
pub(super) const MARKER: &str = "FORBIDDEN_WORDS_GUARD";
pub(super) const HOOKS: [&str; 2] = ["pre-commit", "commit-msg"];
const HEAD: &str = "#!/bin/bash\n# FORBIDDEN_WORDS_GUARD\n# {what}\n# Installed by OpenBot; words: {words}\nset -euo pipefail\nexport TIRITH_GUARD_RE='{re}'\n";
const PRE_COMMIT: &str = include_str!("../../assets/pre-commit.sh");
const COMMIT_MSG: &str = include_str!("../../assets/commit-msg.sh");
/// The repository's own hook, kept next to ours as `<hook>.local` when the guard was installed, still runs.
const LOCAL: &str = include_str!("../../assets/local-hook.sh");

pub(super) fn render(hook: &str, words: &[String]) -> String {
    let (what, body) = match hook {
        "pre-commit" => ("Blocks the commit when a staged file name or an added line contains a forbidden word (whole word, any case).", PRE_COMMIT),
        _ => ("Blocks the commit when the message contains a forbidden word (whole word, any case); # lines are ignored.", COMMIT_MSG),
    };
    let re = words.join("|").replace('\'', "'\\''");
    HEAD.replace("{what}", what)
        .replace("{words}", &words.join(","))
        .replace("{re}", &re)
        + body
        + LOCAL
}

#[cfg(test)]
#[path = "../../checks/guard.rs"]
mod tests;
