//! What a client and the broker pass between them.

/// What the client asks the daemon for: a program as it is to run, and the limit on it. It names no profile: the daemon writes that.
#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub(in crate::sandbox) struct Request {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub env: Vec<(String, String)>,
    /// Workspace-write (else read-only).
    pub write: bool,
    pub roots: Vec<String>,
    /// Codex's home, of which the process itself must write some.
    pub state: String,
    /// The file `-o` names.
    pub out: String,
    /// The commands may use the network (as Tirith's sandbox lets it).
    pub net: bool,
    /// Tirith's own role: its commands keep the daemon, as that agent. No wider than what any confined caller has.
    #[serde(default)]
    pub agent: bool,
}
