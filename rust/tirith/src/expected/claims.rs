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
        r"|\bsin (acceso|permisos?|conexión|red|cuota|sesión)( al? \S+)?\s*([.,;:)!]|$)|\bsin permisos? (para|de (lectura|acceso))\b",
        r"|\bno (tengo|tuve|había|hay) (acceso|permiso|permisos|conexión|red|sesión)\b",
        r"|\b(could not|couldn['’]t|cannot|can['’]t|was not able to|wasn['’]t able to|unable to|failed to)\b",
        r"|\bme (fue|ha sido|resultó) imposible\b|\bno access\b|\bnicht (erreichen|zugreifen|öffnen)\b|\bkonnte\b[^.\n]{0,60}\bnicht\b",
    ))
    .unwrap()
});
/// But not finding anything new is exactly «∅».
static FOUND_NONE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(no (pude|he podido|logré|conseguí|he conseguido)|could not|couldn['’]t|was not able to|unable to|failed to) (encontrar|hallar|find|spot) (nada|ningun\w*|ningún|any\w*|nothing|(\S+ ){0,3}(nuev\w*|new\b))(?P<rest>[^.;\n]*)").unwrap()
});
/// Unless it says why it found none, or it is the way in that it could not find.
static CAUSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(porque|ya que|debido|because|due to|as the|since the|way|manera|forma de|botón|button|login)\b").unwrap()
});
/// And waiting for the owner's yes, or promising nothing, is no failure.
static WAITS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(no puedo|no he podido|cannot|can['’]t)( \S+){0,6}? ((sin|without) (tu|your) (sí|ok|yes|visto bueno|aprobación|confirmación|approval|confirmation)|(hasta que|until) (lo |la )?(apruebes|confirmes|you approve|you confirm))|(no puedo|cannot|can['’]t) (garantizar|asegurar|prometer|guarantee|promise|wait)").unwrap()
});

/// True when `text` says that something could not be done.
pub(super) fn could_not(text: &str) -> bool {
    let mine = super::told::own_words(text);
    mine.lines()
        .any(|said| COULD_NOT.is_match(&but_found_none(said)))
}

/// `text` without what is no failure: finding nothing new, and waiting for the owner's yes.
pub(super) fn but_found_none(text: &str) -> String {
    let text = WAITS.replace_all(text, "");
    let kept = FOUND_NONE.replace_all(&text, |c: &regex::Captures| {
        let said = if CAUSE.is_match(&c[0]) {
            &c[0]
        } else {
            &c["rest"]
        };
        said.to_string()
    });
    kept.into_owned()
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
