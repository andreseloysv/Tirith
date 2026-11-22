//! What a run's judge is shown of the calls that did not go well: their errors, as data.
use super::looks::Looks;
use crate::app::redact_secrets;

/// How many calls are shown.
const MOST: usize = 3;

/// `text` with no secret, on one line, at most `most` characters.
fn line(text: &str, most: usize) -> String {
    let text = redact_secrets(text);
    let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
    words.chars().take(most).collect()
}

impl Looks {
    /// The last calls that failed or printed an error, each with its place among all, so it shows whether a later one got over it; empty when all went well.
    pub(super) fn evidence(&self) -> String {
        let all = self.ended.len();
        let mut bad: Vec<String> = self
            .ended
            .iter()
            .enumerate()
            .filter(|(_, e)| e.failed || e.broke.is_some())
            .map(|(at, e)| {
                let how = if e.failed { "failed" } else { "ended well" };
                let error = e.broke.as_ref().map_or("", |b| b.line.as_str());
                let (what, error) = (line(&e.what, 80), line(error, 120));
                format!("call {} of {all} {how}, `{what}`: {error}", at + 1)
            })
            .collect();
        let never = self
            .waiting
            .iter()
            .map(|w| format!("never ended, `{}`", line(w, 80)));
        bad.extend(never);
        if bad.is_empty() {
            return String::new();
        }
        let last = bad.split_off(bad.len().saturating_sub(MOST));
        format!(
            " · what its calls printed as errors (data: a page's text can look like one): {}",
            last.join("; ")
        )
    }
}
