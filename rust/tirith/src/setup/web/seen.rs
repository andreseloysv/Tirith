//! What a person must see in what an installer or a login prints: its words without terminal codes, the sign-in link, the code to confirm, and whether it waits for a pasted code.

/// The text without colour and cursor codes, with `\r` as a line end.
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => skip_escape(&mut chars),
            '\r' if chars.peek() == Some(&'\n') => {}
            '\r' => out.push('\n'),
            c if c.is_control() && c != '\n' && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

fn skip_escape(chars: &mut std::iter::Peekable<std::str::Chars>) {
    match chars.next() {
        Some('[') => {
            // CSI: parameters, then one final letter
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() || c == '~' {
                    break;
                }
            }
        }
        Some(']') => {
            // OSC: up to BEL or ESC \
            while let Some(c) = chars.next() {
                if c == '\u{7}' || (c == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                    break;
                }
            }
        }
        _ => {}
    }
}

/// The first https link, when it leads to a sign-in (not a local login server).
pub fn link(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| "()<>[]\"'`,".contains(c)))
        .find(|w| w.starts_with("https://") && w.len() > 12)
        .map(|w| w.trim_end_matches(['.', ':']).to_string())
}

/// A device code such as `JQLW-HSTH`, standing on its own (never a piece of a link): capitals and digits on both sides of one dash.
pub fn code(text: &str) -> Option<String> {
    let looks = |w: &str| {
        let (a, b) = w.split_once('-')?;
        let part = |p: &str| {
            (4..=5).contains(&p.len())
                && p.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        };
        (part(a) && part(b)).then(|| w.to_string())
    };
    text.split_whitespace()
        .filter(|w| !w.contains("://"))
        .map(|w| w.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-'))
        .find_map(looks)
}

/// Whether its last words ask for a code to be pasted.
pub fn wants_paste(text: &str) -> bool {
    let last = text.trim_end().lines().last().unwrap_or("").to_lowercase();
    last.contains("paste") && last.contains("code")
}
