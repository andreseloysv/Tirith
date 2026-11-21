//! What a process Tirith starts runs as: the program, its arguments and the environment the sandbox adds.
use crate::app::Shared;

pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl Launch {
    pub fn command(&self, app: &Shared) -> tokio::process::Command {
        let mut c = tokio::process::Command::new(&self.program);
        c.args(&self.args);
        let _ =
            (!self.env.iter().any(|(k, _)| k == "PATH")).then(|| c.env("PATH", app.child_path()));
        c.envs(self.env.iter().map(|(k, v)| (k, v)))
            .process_group(0); // its own process group, so a stop kills all it started
        c
    }
}
