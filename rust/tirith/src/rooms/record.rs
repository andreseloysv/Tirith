//! What is kept of a room and of the people in it (SQLite table `am_rooms`, one JSON row per room). Pairing codes live only here.
use serde::{Deserialize, Serialize};
/// How many of a room's messages are kept.
pub(super) const KEPT: usize = 40;
pub(super) const CODE_SECS: u64 = 900; // how long a pairing code is good for: a picture of the page is soon worth nothing
/// How the manager is told that a room's line is the owner's own.
pub(super) const PHONE: &str = "(the owner's paired phone)";
/// True for the author of a line the owner's paired phone wrote (another agent in the room never is).
pub fn paired_phone(author: &str) -> bool {
    author.ends_with(PHONE)
}
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(super) struct Peer {
    pub(super) name: String,
    pub(super) key_id: String,
    pub(super) public: String,
    pub(super) since: String,
    pub(super) last_seen: String,
    pub(super) accepted: bool,
    /// A phone the owner had NeoBot forget: nothing it writes is read, until it is paired again.
    pub(super) blocked: bool,
}
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(super) struct Msg {
    pub(super) n: u64,
    pub(super) at: String,
    pub(super) from: String,
    pub(super) text: String,
    pub(super) device: bool,
}
/// The one-use codes of a room this manager made.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(super) struct Codes {
    /// The code of the current one-use QR, good for `CODE_SECS` from `invite_at`; empty once it was used. A new QR replaces it.
    pub(super) invite: String,
    pub(super) invite_at: u64,
    /// The current one-use code for another manager, `<room>:<my key id>:<code>`; empty once it was used.
    pub(super) peer_code: String,
}
/// How many agents' messages came in a row with none of the owner's, and whether that stopped the answers.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(super) struct Brake {
    pub(super) streak: u32,
    pub(super) braked: bool,
}
/// What the phones were last told (`tirith-roster`, `tirith-rooms`); not kept.
#[derive(Clone, Default)]
pub(super) struct Told {
    pub(super) roster: String,
    pub(super) listed: String,
}
/// What the manager owes a room.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(super) struct Owed {
    /// Agents whose messages started the manager's current turn: its answer is sealed for them too.
    pub(super) turn: Vec<String>,
    /// What was handed to the manager and no finished turn has answered: (author, text). Handed again, once, after a restart. `answering` is the part of it the turn now running received when it started: only that part stops being owed when that turn ends.
    pub(super) owed: Vec<(String, String)>,
    pub(super) answering: Vec<(String, String)>,
}
/// One room's record.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(super) struct Room {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) project: String,
    /// "manager" (made here: pairs phones), "peer" (joined another manager's room), "public" (config.toml)
    pub(super) kind: String,
    pub(super) me: String,
    pub(super) me_id: String,
    pub(super) cursor: Option<u64>,
    pub(super) devices: Vec<Peer>,
    pub(super) managers: Vec<Peer>,
    #[serde(flatten)]
    pub(super) codes: Codes,
    #[serde(flatten)]
    pub(super) brake: Brake,
    #[serde(skip)]
    pub(super) told: Told,
    #[serde(flatten)]
    pub(super) owes: Owed,
    pub(super) messages: Vec<Msg>,
    pub(super) here: Vec<String>,
    pub(super) error: String,
}
impl Room {
    pub(super) fn private(&self) -> bool {
        self.kind != "public"
    }
}
