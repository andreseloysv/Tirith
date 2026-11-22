//! A limited Codex is two processes. Its own keeps what is its alone (its state with the sign-in it renews, the proxy) and writes in none of the folders; what runs the model's commands and patches, `apply_patch` too, is its `exec-server`, under the limit and with nothing of that state. A patch cannot be told apart inside one process, so it is not in that process.
use super::bridge::Bridge;
use super::broken::broken;
use super::capable::{capable, URL};
use super::limits::{Around, Part};
use super::{ending::end, line::NO_NET, spawn::spawn, watch::Watch, Request};
use super::{notice::Notice, too_big};
use std::os::fd::OwnedFd;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Stdio};
use std::time::Instant;

pub(super) struct Pair {
    codex: Child,
    runs: Child,
    bridge: Bridge,
    notice: Notice,
    watches: Vec<Watch>,
    began: Instant,
    over: bool,
}

impl Pair {
    pub(super) fn start(mut r: Request, around: &Around, io: [OwnedFd; 3]) -> Result<Pair, String> {
        capable(&r.program, &r.state)?;
        let mut bridge = Bridge::open()?;
        let notice = Notice::to(&io[2]);
        r.env.retain(|(name, _)| name != URL);
        let mut theirs = r.clone();
        theirs.args = ["exec-server", "--listen", "stdio"]
            .map(String::from)
            .to_vec();
        theirs
            .env
            .retain(|(name, _)| r.net || !NO_NET.contains(&name.as_str()));
        let pipes = [Stdio::piped(), Stdio::piped(), Stdio::null()];
        let mut runs = spawn(&theirs, around, Part::Commands, pipes)?;
        r.env
            .push((URL.into(), format!("ws://127.0.0.1:{}", bridge.port)));
        let codex = match spawn(&r, around, Part::Codex, io.map(Stdio::from)) {
            Ok(codex) => codex,
            Err(why) => {
                end(&mut runs);
                return Err(why);
            }
        };
        if let (Some(to), Some(from)) = (runs.stdin.take(), runs.stdout.take()) {
            bridge.serve(codex.id() as libc::pid_t, to, from);
        }
        let mut pair = Pair {
            codex,
            runs,
            bridge,
            notice,
            watches: vec![],
            began: Instant::now(),
            over: false,
        };
        for pid in [pair.codex.id(), pair.runs.id()] {
            pair.watches.push(Watch::over(pid)?); // and an error ends both
        }
        Ok(pair)
    }

    /// Its exit code once it is over.
    pub(super) fn ended(&mut self) -> Result<Option<i32>, String> {
        if self.bridge.cut() {
            self.notice.say(&too_big::why());
        }
        if let Some(status) = self.codex.try_wait().map_err(|e| e.to_string())? {
            crate::app::kill_group(self.codex.id(), libc::SIGTERM); // nothing it left outlives it
            self.watches.clear();
            end(&mut self.runs);
            self.over = true;
            let code = status.code().unwrap_or(128 + status.signal().unwrap_or(0));
            return Ok(Some(code));
        }
        let Some(why) = broken(&self.bridge, self.runs.id(), self.began) else {
            return Ok(None);
        };
        self.end();
        Err(why.into())
    }

    pub(super) fn end(&mut self) {
        if !self.over {
            self.over = true;
            end(&mut self.codex);
            end(&mut self.runs);
            self.watches.clear();
        }
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        self.end();
    }
}
