//! Two small helps for an agent that looks at a page: a `find` given only an address means that page, and a read
//! that shows almost no text says where the rest is.

/// Under this many characters of text a page is taken to draw itself where `read` does not look.
const THIN: usize = 200;
const HINT: &str = "(little text here: a page drawn inside frames or as pictures has none to read. neobot safari find lists what can be pressed and typed into there)";

/// The words and the address a command asked for: `find <address>` alone is the page at that address, not words.
pub(super) fn asked<'a>(cmd: &str, target: &'a str, url: &'a str) -> (&'a str, &'a str) {
    let address = target.starts_with("https://") || target.starts_with("http://");
    match cmd == "find" && url.is_empty() && address {
        true => ("", target),
        false => (target, url),
    }
}

/// A read (URL, title, text) with the hint after it when the text is that short.
pub(super) fn hinted(page: String) -> String {
    let text = page.splitn(3, '\n').nth(2).unwrap_or("");
    let shown = text.chars().filter(|c| !c.is_whitespace()).count();
    match shown < THIN {
        true => format!("{}\n{HINT}", page.trim_end()),
        false => page,
    }
}
