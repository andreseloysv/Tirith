//! An answer that says, in its own words, that the task could not be done: not someone's words that it quotes or tells.
use super::told::own_words;
use regex::Regex;
use std::sync::LazyLock;

/// Saying of oneself, or of what one started, that it could not.
static SELF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)\bno (pude|puedo|pudimos|he podido|hemos podido|logré|conseguí|he logrado|he conseguido|fui capaz|he sido capaz)\b",
        r"|\bno me (fue|ha sido) posible\b|\bme (fue|ha sido|resultó) imposible\b|\bno (tuve|tengo) (acceso|conexión|permisos?|sesión)\b",
        r"|\b(i|we) (could not|couldn['’]t|cannot|can['’]t|was not able to|wasn['’]t able to|was unable to|am unable to|were unable to|failed to|didn['’]t manage to)\b",
        r"|\bno (se pudo|se pudieron|se ha podido|se han podido|pudo|ha podido|pudieron|logró|consiguió) (buscar|abrir|acceder|leer|entrar|comprobar|revisar|mirar|consultar|conectar|ejecutar|arrancar|completar|terminar|realizar|cargar|hacer)\w{0,5}\b",
        r"|\b(could not|couldn['’]t|cannot|can['’]t|was unable to|unable to|failed to) (search|open|access|read|sign in|log ?in|get in|check|connect|run|start|complete|finish|reach|look|fetch|load|be (searched|checked|done|opened|run)|do (it|that|the task))\b",
        r"|\bfalló (y |, )?no (pudo|se pudo)\b|\bfailed and (could not|couldn['’]t)\b",
        r"|\bich konnte\b[^.\n]{0,60}\bnicht\b",
    ))
    .unwrap()
});
/// Words of trouble: they count only beside the tool's own failure.
static TROUBLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)\bfall(ó|o|aron|ado|a)\b|\bfracas|\berror(es)?\b|\bproblema|\bsin hacer\b",
        r"|\bno se (hizo|pudo|pudieron|completó)|\bno funcion|\bbloque|\bno (pud|puedo)",
        r"|\bfail(ed|ure|s)?\b|\b(did not|didn['’]t) (work|go through|run|finish)|\bblocked\b|\bdenied\b",
        r"|\bcrash|\bunable\b|\bnot possible\b|\bcould ?n[o'’]t\b|\btrouble\b|\bbroken?\b",
    ))
    .unwrap()
});

/// True when it admits that it could not.
pub(super) fn admits(answer: &str) -> bool {
    let mine = own_words(answer);
    mine.lines()
        .any(|said| SELF.is_match(&super::claims::but_found_none(said)))
}

/// True when it speaks of trouble.
pub(super) fn trouble(answer: &str) -> bool {
    TROUBLE.is_match(&own_words(answer))
}

/// True when it repeats the tool's error.
pub(super) fn repeats(answer: &str, error: &str) -> bool {
    let words = |t: &str| {
        let lower = t.to_lowercase();
        lower.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let tail = words(error.rsplit(": ").next().unwrap_or(error));
    let tail = tail.trim_end_matches('.');
    tail.chars().count() >= 16 && words(answer).contains(tail)
}
