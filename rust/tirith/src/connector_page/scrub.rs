//! An error's text with no secret in it: each value the bot holds, a Bearer token, and the value of any `key=value` or `"key": "value"` whose key names a secret (token, secret, password, key, auth, cookie, credential, code).
use regex::Regex;
use std::sync::LazyLock;

const HIDDEN: &str = "(hidden)";

static BEARER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(bearer|basic)\s+[^\s,;)]+").unwrap());
static PAIR: LazyLock<Regex> = LazyLock::new(|| {
    let key = r"[A-Za-z0-9_-]*(?:token|secret|password|passwd|key|auth|cookie|credential|code)[A-Za-z0-9_-]*";
    Regex::new(&format!(r#"(?i)("?{key}"?\s*[=:]\s*"?)[^\s"&,;}}]+"#)).unwrap()
});
static URL_QUERY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(https?://[^\s?"]+)\?[^\s"]*"#).unwrap());

/// `text` with every value of `values` (4 characters or more), every Bearer token, every query string and every secret-named pair's value as `(hidden)`.
pub fn scrub(text: &str, values: &[String]) -> String {
    let mut out = text.to_string();
    for v in values.iter().filter(|v| v.len() >= 4) {
        out = out.replace(v.as_str(), HIDDEN);
    }
    let out = BEARER.replace_all(&out, format!("$1 {HIDDEN}"));
    let out = URL_QUERY.replace_all(&out, format!("$1?{HIDDEN}"));
    PAIR.replace_all(&out, format!("${{1}}{HIDDEN}"))
        .into_owned()
}
