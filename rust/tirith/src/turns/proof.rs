//! What a turn's calls prove of a run: a look that did not work, and an error that nothing after it undid.
use super::look_lines::Ended;
use super::looks::Looks;
use regex::Regex;
use std::sync::LazyLock;

/// A call that looks at something outside.
static OUTSIDE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)safari|curl|wget|fetch|web_?search|browser|playwright|navigate|mcp__|https?://|\bcodex\b|\bgh |\bssh ").unwrap()
});
/// A call that only keeps books: it undoes no failure.
static BOOKS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^((bash|shell|exec_command): )?(tirith (turn|say|note)|echo|printf|cat|ls|pwd|date|true|head|tail|wc|which|command -v|mkdir|touch|sleep|codex (--version|--help|login status))\b|^(edit|write|todowrite|write_file|edit_file|add_memory|edit_memory|write_todos)\b").unwrap()
});
/// A tool that reads what is kept: what it shows is content, whatever it says.
static READS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(read|grep|glob|read_file|read_skill|read_memory|skill)\b").unwrap()
});
/// An agent's CLI, whose status does not tell.
static NESTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b(codex|muse|copilot)["']? (.{0,80} )?(exec|e|-p)\b"#).unwrap()
});

/// True of a call that looks outside.
pub(super) fn outside(what: &str) -> bool {
    OUTSIDE.is_match(what)
}

fn books(what: &str) -> bool {
    BOOKS.is_match(what) || READS.is_match(what)
}

/// An error nothing after it undid.
pub struct Proof<'a> {
    pub line: &'a str,
    /// It alone proves the failure.
    pub hard: bool,
}

/// Whether one of `after` undid `broken`: the same call worked, or other work did (for a hard error only another look outside).
fn undone(after: &[Ended], broken: &Ended, hard: bool) -> bool {
    after.iter().any(|e| {
        let worked = !e.failed && e.broke.is_none();
        let other = !books(&e.what) && (!hard || outside(&e.what));
        worked && (e.what == broken.what || other)
    })
}

impl Looks {
    /// The look that did not work: one outside that failed and never worked after, one that never ended, or the last call when none worked at all.
    pub fn blind(&self) -> Option<&str> {
        let retried = |at: usize, what: &str| {
            let mut after = self.ended[at + 1..].iter();
            after.any(|e| e.what == what && !e.failed && e.broke.is_none())
        };
        let mut all = self.ended.iter().enumerate();
        let failed = all.find(|(at, e)| e.failed && outside(&e.what) && !retried(*at, &e.what));
        let none = || {
            self.ended
                .last()
                .filter(|_| self.ended.iter().all(|e| e.failed))
        };
        failed
            .map(|(_, e)| e.what.as_str())
            .or_else(|| self.waiting.first().map(String::as_str))
            .or_else(|| none().map(|e| e.what.as_str()))
    }

    /// True when it tried to work and every try failed.
    pub fn nothing_worked(&self) -> bool {
        let mut work = self.ended.iter().filter(|e| !books(&e.what)).peekable();
        work.peek().is_some() && work.all(|e| e.failed)
    }

    /// The error that stands: a hard one first. A soft one counts only of a call that failed, or of an agent's CLI.
    pub fn proof(&self) -> Option<Proof<'_>> {
        let standing = self.ended.iter().enumerate().filter_map(|(at, e)| {
            let broke = e.broke.as_ref()?;
            let content = !e.failed && READS.is_match(&e.what);
            let counts = broke.hard || e.failed || NESTED.is_match(&e.what);
            let stands = !content && counts && !undone(&self.ended[at + 1..], e, broke.hard);
            stands.then_some(Proof {
                line: &broke.line,
                hard: broke.hard,
            })
        });
        let mut standing: Vec<Proof> = standing.collect();
        let hard = standing.iter().position(|p| p.hard);
        hard.map(|at| standing.remove(at))
            .or_else(|| standing.pop())
    }
}
