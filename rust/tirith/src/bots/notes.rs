//! Numbered memory notes: every line that starts with `-` or `*` carries `[m<N>]` (a bot's own memory) or `[s<N>]` (the shared one), so an answer can cite the note it used.
use std::path::Path;
/// The note's text after its bullet (`- ` or `* `, indented or not), or None for a line that is no note.
fn note(line: &str) -> Option<(usize, &str)> {
    let lead = line.len() - line.trim_start().len();
    let rest = &line[lead..];
    (rest.starts_with('-') || rest.starts_with('*'))
        .then(|| (lead + 1, &rest[1..]))
        .filter(|(_, r)| r.starts_with(' ') && !r.trim().is_empty())
}
/// The number of a note that has one: `[<tag><N>]` right after its bullet.
fn number(text: &str, tag: char) -> Option<u64> {
    let t = text.trim_start().strip_prefix('[')?.strip_prefix(tag)?;
    let end = t.find(']')?;
    t[..end].parse().ok()
}
/// `text` with `[<tag><N>]` before every note that has none yet, numbered after the highest already there; headings, numbered notes and every other line stay as they are.
pub(super) fn numbered(text: &str, tag: char) -> String {
    let mut next = text
        .lines()
        .filter_map(|l| note(l).and_then(|(_, r)| number(r, tag)))
        .max()
        .unwrap_or(0);
    let mut out: Vec<String> = text
        .lines()
        .map(|l| match note(l) {
            Some((at, r)) if number(r, tag).is_none() => {
                next += 1;
                format!("{} [{tag}{next}]{r}", &l[..at])
            }
            _ => l.to_string(),
        })
        .collect();
    if text.ends_with('\n') {
        out.push(String::new());
    }
    out.join("\n")
}
/// The memory file at `file` numbered (and saved so when something changed), as it is now; empty when there is none.
pub(super) fn numbered_file(file: &Path, tag: char) -> String {
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let done = numbered(&text, tag);
    if done != text {
        let _ = std::fs::write(file, &done);
    }
    done
}
