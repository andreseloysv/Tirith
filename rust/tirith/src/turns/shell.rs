//! A shell call as the agent asked for it: Codex shows the command inside its shell (`/bin/bash -lc '…'`), while its hooks and the other agents name the command itself.
use regex::Regex;
use std::sync::LazyLock;

static WRAPPED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)^(?:\S*/)?(?:ba|z)?sh\s+-l?c\s+(.+)$").unwrap());
static PIECE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)'([^']*)'|"((?:[^"\\]|\\.)*)"|\\(.)|([^\s'"\\]+)"#).unwrap()
});
static ESCAPED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\\([$`"\\])"#).unwrap());

/// The command a `sh -c` wrapper runs, unquoted; any other command as it is.
pub(super) fn inner(command: &str) -> String {
    let word = WRAPPED.captures(command).and_then(|c| unquoted(&c[1]));
    word.unwrap_or_else(|| command.to_string())
}

/// One shell word without its quotes; none when it is more than one word.
fn unquoted(word: &str) -> Option<String> {
    let (mut out, mut at) = (String::new(), 0);
    for part in PIECE.captures_iter(word) {
        let whole = part.get(0)?;
        (whole.start() == at).then_some(())?;
        at = whole.end();
        if let Some(double) = part.get(2) {
            out += &ESCAPED.replace_all(double.as_str(), "$1");
        } else if let Some(plain) = part.get(1).or(part.get(3)).or(part.get(4)) {
            out += plain.as_str();
        }
    }
    (at == word.len()).then_some(out)
}
