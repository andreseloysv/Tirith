//! The bridge's way back: what runs a limited Codex's commands says it, a message a line.
use super::carry::{send, Out};
use super::{bridge::Seen, frames, too_big::refusal};
use std::io::{BufRead, BufReader, Read};
use std::process::ChildStdout;
use std::sync::atomic::Ordering::SeqCst;

/// What runs the commands says, a message a line, until it is gone.
pub(super) fn answers(from: ChildStdout, out: &Out, seen: &Seen) -> Option<()> {
    let mut lines = BufReader::new(from);
    let mut line = vec![];
    loop {
        line.clear();
        let got = (&mut lines)
            .take(frames::MOST + 1)
            .read_until(b'\n', &mut line)
            .ok()?;
        (got > 0).then_some(())?;
        let message = if got as u64 > frames::MOST && line.last() != Some(&b'\n') {
            lines.skip_until(b'\n').ok()?;
            seen.cut.store(true, SeqCst);
            refusal(&line)? // one that answers no call cannot be refused: the run is lost
        } else {
            line.trim_ascii().to_vec()
        };
        if !message.is_empty() {
            send(out, frames::TEXT, &message).ok()?;
        }
    }
}
