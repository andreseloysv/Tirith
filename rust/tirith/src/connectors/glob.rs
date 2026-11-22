//! The globs of `tools`, `deny` and `approval`: `*` is any run of characters, `?` one; anything else matches itself.

/// True when `name` matches `pattern` as a whole.
pub(super) fn matches(pattern: &str, name: &str) -> bool {
    let (p, n): (Vec<char>, Vec<char>) = (pattern.chars().collect(), name.chars().collect());
    let (mut pi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            (pi, ni) = (pi + 1, ni + 1);
        } else if pi < p.len() && p[pi] == '*' {
            (star, mark, pi) = (Some(pi), ni, pi + 1);
        } else if let Some(s) = star {
            (pi, mark) = (s + 1, mark + 1);
            ni = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

/// True when any of the globs matches `name`.
pub(super) fn any(globs: &[String], name: &str) -> bool {
    globs.iter().any(|g| matches(g, name))
}
