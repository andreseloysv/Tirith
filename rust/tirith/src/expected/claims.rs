//! What an answer claims. «∅» is "I looked and nothing is new"; «✗» is "I could not"; a «∅» that says it could not is a «✗».
use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

pub(super) const NOTHING: char = '∅';
pub(super) const FAILED: char = '✗';

/// Saying one could not.
static COULD_NOT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)\bno (pude|puedo|pudo|pudimos|he podido|ha podido|he logrado|logré|he conseguido|conseguí|fue posible|ha sido posible|me fue posible)\b",
        r"|\bimposible (acceder|abrir|leer|comprobar|entrar|revisar|buscar|mirar)",
        r"|\bsin (acceso|permiso|permisos|conexión|red|cuota|sesión)\b",
        r"|\bno (tengo|tuve|había|hay) (acceso|permiso|permisos|conexión|red|sesión)\b",
        r"|\b(could not|couldn['’]t|cannot|can['’]t|was not able to|wasn['’]t able to|unable to|failed to)\b",
        r"|\bno access\b|\bnicht (erreichen|zugreifen|öffnen)\b|\bkonnte\b[^.\n]{0,60}\bnicht\b",
    ))
    .unwrap()
});
/// But not finding anything new is exactly «∅».
static FOUND_NONE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(no (pude|he podido|logré|conseguí|he conseguido)|could not|couldn['’]t|was not able to|unable to|failed to) (encontrar|hallar|find|spot) (nada|ningun\w*|ningún|nuev\w*|ofertas nuevas|any\w*|new)").unwrap()
});

/// True when `text` says that something could not be done.
pub(super) fn could_not(text: &str) -> bool {
    COULD_NOT.is_match(&FOUND_NONE.replace_all(text, ""))
}

/// True when the first line of `text` begins by saying so.
pub(super) fn begins_could_not(text: &str) -> bool {
    let line = text.trim_start().lines().next().unwrap_or("");
    let head: String = line.chars().take(32).collect();
    could_not(&head) && could_not(line)
}

/// What follows `mark` where a paragraph of the answer begins with it, to the answer's end.
pub(super) fn after(answer: &str, mark: char) -> Option<&str> {
    let text = answer.trim();
    let mut begins = std::iter::once(0).chain(text.match_indices("\n\n").map(|(at, _)| at + 2));
    begins
        .find_map(|at| text[at..].trim_start().strip_prefix(mark))
        .map(str::trim)
}

/// The text as it is kept and sent: a «∅» that says it could not begins with «✗» instead, so nothing treats it as silence.
pub fn honest(text: &str) -> Cow<'_, str> {
    match text.trim_start().strip_prefix(NOTHING) {
        Some(rest) if could_not(rest) => Cow::Owned(format!("{FAILED}{rest}")),
        _ => Cow::Borrowed(text),
    }
}

/// True for an answer that stays off the phones: it truly says that nothing is new.
pub fn quiet(text: &str) -> bool {
    honest(text).trim_start().starts_with(NOTHING)
}
