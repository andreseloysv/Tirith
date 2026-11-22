//! An answer that says in the first person that the task could not be done: not someone's words that it quotes or tells, and not the failure of someone or something that the task reports.
use super::told::own_words;
use regex::Regex;
use std::sync::LazyLock;

/// Saying of oneself, or of what one started, that it could not.
static SELF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)\bno (pude|puedo|pudimos|he podido|hemos podido|logré|conseguí|he logrado|he conseguido|fui capaz|he sido capaz)\b",
        r"|\bno me (fue|ha sido) posible\b|\bme (fue|ha sido|resultó) imposible\b|\bno (tuve|tengo) (acceso|conexión|permisos?|sesión)\b",
        r"|\b(i|we) (could not|couldn['’]t|cannot|can['’]t|was not able to|wasn['’]t able to|was unable to|am unable to|were unable to|failed to|didn['’]t manage to)\b",
        r"|^\W*(sorry\W+)?(could not|couldn['’]t|cannot|can['’]t|was unable to|unable to) ",
        r"|\bfalló (y |, )?no (pudo|se pudo)\b|\bfailed and (could not|couldn['’]t)\b",
        r"|\bich konnte\b[^.\n]{0,60}\bnicht\b",
    ))
    .unwrap()
});
/// Saying of what one tried that it could not be done.
static TRIED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)\bno (pude|pudimos|he podido|hemos podido|logré|conseguí|he logrado|he conseguido|fui capaz|he sido capaz)\b",
        r"|\bno me (fue|ha sido) posible\b|\bme (fue|ha sido|resultó) imposible\b",
        r"|\b(i|we) (could not|couldn['’]t|was not able to|wasn['’]t able to|was unable to|were unable to|failed to|didn['’]t manage to)\b",
    ))
    .unwrap()
});
/// Words of trouble: they count only where no judge read the run.
static TROUBLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)\bfall(ó|o|aron|ado|a)\b|\bfracas|\berror(es)?\b|\bproblema|\bsin hacer\b",
        r"|\bno se (hizo|pudo|pudieron|completó)|\bno funcion|\bbloque|\bno (pud|puedo)",
        r"|\bfail(ed|ure|s)?\b|\b(did not|didn['’]t) (work|go through|run|finish)|\bblocked\b|\bdenied\b",
        r"|\bcrash|\bunable\b|\bnot possible\b|\bcould ?n[o'’]t\b|\btrouble\b|\bbroken?\b",
    ))
    .unwrap()
});

/// True when it admits that it could not. Code takes that for certain only on a «∅»'s line, and otherwise only where no judge reads the run.
pub(super) fn admits(answer: &str) -> bool {
    let mine = own_words(answer);
    mine.lines()
        .any(|said| SELF.is_match(&super::claims::but_found_none(said)))
}

/// The same of a scheduled pass, which reports on others and may propose nothing: it says so where it begins, or of what it tried. Further on, "so I can't start an experiment there" is its reason to propose nothing.
pub(super) fn pass_admits(answer: &str) -> bool {
    let opening = answer.trim().split(['.', ':', '\n']).next().unwrap_or("");
    let tried = |said: &str| TRIED.is_match(&super::claims::but_found_none(said));
    admits(opening) || own_words(answer).lines().any(tried)
}

/// True when it speaks of trouble.
pub(super) fn trouble(answer: &str) -> bool {
    TROUBLE.is_match(&own_words(answer))
}
