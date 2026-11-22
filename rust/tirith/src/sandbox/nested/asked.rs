//! What a `codex exec` command line asks of Codex's sandbox, and that line without the words that ask it.
use super::flags::{option, valued};
use super::mode::Mode;

#[derive(Debug, Default)]
pub(super) struct Asked {
    pub mode: Option<Mode>,
    pub cd: Option<String>,
    pub adds: Vec<String>,
    pub out: Option<String>,
    pub profile: Option<String>,
    pub permissions: bool,
    pub ignore_config: bool,
    pub worktree: bool,
    /// The `-c` settings, the last of a key wins.
    pub sets: Vec<(String, String)>,
    /// The line without what asked for a sandbox, and where an option of `exec` goes into it.
    pub args: Vec<String>,
    pub at: usize,
}

/// None for a command that is no `exec` (nothing runs commands unattended there).
pub(super) fn read(args: &[String]) -> Option<Asked> {
    let mut a = Asked::default();
    let (mut exec, mut i) = (false, 0);
    while let Some(word) = args.get(i) {
        i += 1;
        if word == "--" {
            a.args.push(word.clone());
            a.args.extend(args[i..].iter().cloned());
            break;
        }
        let Some((name, written)) = option(word).filter(|_| word.len() > 1) else {
            if !exec && !word.starts_with('-') {
                matches!(word.as_str(), "exec" | "e").then_some(())?;
                exec = true;
                a.at = a.args.len() + 1;
            }
            a.args.push(word.clone());
            continue;
        };
        let mut kept = vec![word.clone()];
        let mut value = written.map(String::from);
        if value.is_none() && valued(name) {
            value = args.get(i).cloned();
            kept.extend(value.clone());
            i += 1;
        }
        if a.take(name, value.unwrap_or_default()) {
            a.args.extend(kept);
        }
    }
    exec.then_some(a)
}

impl Asked {
    /// Takes one option; false when it only asked for a sandbox.
    fn take(&mut self, name: &str, value: String) -> bool {
        match name {
            "--sandbox" => self.mode = Some(Mode::named(&value).unwrap_or(Mode::ReadOnly)),
            "--full-auto" | "--approve-for-me" => self.mode = self.mode.or(Some(Mode::Write)),
            "--dangerously-bypass-approvals-and-sandbox" | "--yolo" => self.mode = Some(Mode::Full),
            "--config" => return self.set(&value),
            "--cd" => self.cd = Some(value),
            "--add-dir" => self.adds.push(value),
            "--output-last-message" => self.out = Some(value),
            "--profile" => self.profile = Some(value),
            "--permission-profile" => self.permissions = true,
            "--ignore-user-config" => self.ignore_config = true,
            "--worktree" => self.worktree = true,
            _ => {}
        }
        !matches!(
            name,
            "--sandbox"
                | "--full-auto"
                | "--approve-for-me"
                | "--dangerously-bypass-approvals-and-sandbox"
                | "--yolo"
        )
    }

    fn set(&mut self, pair: &str) -> bool {
        let Some((key, value)) = pair.split_once('=') else {
            return true;
        };
        self.sets.push((key.trim().into(), value.trim().into()));
        key.trim() != "sandbox_mode"
    }
}
