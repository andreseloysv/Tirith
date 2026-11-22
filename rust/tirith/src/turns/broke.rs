//! A tool's own error in what it printed. An exit status alone does not tell: `codex exec` ends well when every command it ran was refused.
use regex::Regex;
use std::sync::LazyLock;

/// An error line, and whether it alone proves that nothing could be done there.
#[derive(Clone, Debug)]
pub(super) struct Broke {
    pub line: String,
    pub hard: bool,
}

/// macOS's refusal of a sandbox inside another, as a line or inside a line of JSON events.
static NESTING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)^(?:\{.*)?(?P<said>(?:sandbox-exec: )?sandbox_apply: [^"\\]{3,80})"#).unwrap()
});
/// Fatal wherever it stands at a line's beginning.
static FATAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^bwrap: |^tool failed: environment failure|^tirith: (codex could not be started|a codex already under a limit)|^(error: )?not logged in\b").unwrap()
});
/// Fatal as an output's end (a page that was read may say the same in its middle).
static FINAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^curl: \((5|6|7|28|35|52|56|60)\)|^(error: |api error: )?.{0,40}\b(econnrefused|enotfound|eai_again|etimedout)\b|^(error: |stream error: )?(stream disconnected before|error sending request|unable to connect to api)|^(api error: |error: )(401|402|403|429)\b").unwrap()
});
/// How an agent or a CLI begins the line that says its quota is spent: a page's text does not.
static SPENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(error|api error|fatal|you['’]ve (hit|reached)|you have (hit|reached|used))\b",
    )
    .unwrap()
});
/// An error, that proves a failure only with the call's own failure.
static ERROR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(\w+)?(error|exception): \S|^tool failed: |^curl: \(\d+\)|\bcommand not found\b|: (operation not permitted|permission denied|connection refused|access denied)\.?$").unwrap()
});
/// Nothing but the status.
static STATUS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\W*(exit(ed with exit)? ?(code|status) \d+\W*)?$").unwrap());
/// How many lines count as an output's end.
const END: usize = 4;
/// How much of what a call gave a judge is shown.
const GAVE: usize = 160;
/// A line longer than this is content, not an error.
const LONG: usize = 300;

/// The error in `output`: a fatal one first, else the last one; of a call that `failed`, else its last line.
pub(super) fn broke(output: &str, failed: bool) -> Option<Broke> {
    let (hard, soft) = (true, false);
    if let Some(found) = output.lines().find_map(|l| NESTING.captures(l.trim())) {
        let line = found["said"].trim().to_string();
        return Some(Broke { line, hard });
    }
    let lines: Vec<&str> = output
        .lines()
        .map(str::trim)
        .filter(|l| !STATUS.is_match(l) && l.len() <= LONG)
        .collect();
    let mut found = None;
    for (at, line) in lines.iter().copied().enumerate() {
        let end = at + END >= lines.len();
        let spent = || (failed || SPENT.is_match(line)) && crate::models::out_of_credits(line);
        if FATAL.is_match(line) || (end && (FINAL.is_match(line) || spent())) {
            return Some(Broke {
                line: line.into(),
                hard,
            });
        }
        if ERROR.is_match(line) {
            found = Some(line);
        }
    }
    let line = found.or_else(|| lines.last().copied().filter(|_| failed))?;
    Some(Broke {
        line: line.into(),
        hard: soft,
    })
}

/// What a call gave, short: all of it when that is short, else its start and its end.
pub(super) fn gave(output: &str) -> String {
    let said = output
        .lines()
        .map(str::trim)
        .filter(|l| !STATUS.is_match(l));
    let lines: Vec<&str> = said.collect();
    let whole = lines.join(" / ");
    if whole.chars().count() <= GAVE {
        return whole;
    }
    let cut = |line: &str| line.chars().take(GAVE / 2).collect::<String>();
    match (lines.first(), lines.last()) {
        (Some(first), Some(last)) if lines.len() > 1 => format!("{} … {}", cut(first), cut(last)),
        _ => whole.chars().take(GAVE).collect(),
    }
}

/// True when nothing was printed but the status.
pub(super) fn silent(output: &str) -> bool {
    output.lines().all(|l| STATUS.is_match(l.trim()))
}
