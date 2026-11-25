//! Whose words an answer holds: what it quotes, and what it tells of someone's, are theirs. What it tells of its own tool or helper is its own.
use regex::Regex;
use std::sync::LazyLock;

/// Quoted words.
static QUOTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)«[^»]*»|“[^”]*”|"[^"\n]*"|‘[^\n]*?’(\W|$)|(^|[\s(:])'[^\n]*?'([\s).,;:!?]|$)|`[^`]*`|^[ \t]*>[^\n]*$"#).unwrap()
});
/// Where a clause ends and the answer speaks for itself again.
static BREAK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)[\n!?;]|\. |,? (pero|así que|por eso|but) ").unwrap());
/// Someone's words begin.
static TOLD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)\b(dice|dicen|dijo|dijeron|escribe|escribió|escribieron|responde|respondió|respondieron|contestó|cuenta|contó|avisa de|avisan de)( ?:| que\b)",
        r"|\b(says|said|writes|wrote|replied|reports that|told me|told us)\b",
        r"|\b(correo|mensaje|respuesta|comentario|reseña|e-?mail|message|reply|comment)( \S+)? (de|del|from|by) [^:]{1,40}:",
    ))
    .unwrap()
});
/// Before those: itself, or what it started.
static OURS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(^|\W)(i|we|yo|codex|muse|copilot|agente|ayudante|herramienta|script|orden|comando|navegador|safari|chrome|browser|helper|agent|tool|command|cli|error|sandbox)(\W|$)").unwrap()
});

/// The answer's own words: a clause a line, without what it quotes or tells of someone.
pub(super) fn own_words(answer: &str) -> String {
    let unquoted = QUOTED.replace_all(answer, " ");
    let clauses: Vec<&str> = BREAK
        .split(&unquoted)
        .map(|clause| match TOLD.find(clause) {
            Some(told) if !OURS.is_match(&clause[..told.start()]) => &clause[..told.start()],
            _ => clause,
        })
        .collect();
    clauses.join("\n")
}
