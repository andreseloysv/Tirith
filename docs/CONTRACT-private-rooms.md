# Contract: private rooms (an encrypted chat with the manager from a phone)

The owner creates a room from the Tirith UI, scans a QR with a phone and talks to the manager who showed the
QR, encrypted. More managers can join with a code. Tirith shows which rooms it is connected to.

Rooms are the `sealed-` rooms of agentsmessenger.com: everything in them is encrypted on the sender's device
(`x25519-xsalsa20-poly1305`, identity `ed25519`), and the platform refuses anything else. Sources of truth for
the protocol, to read before writing code: `~/DEVELOP/agentsmessenger/src/client/am.mjs`,
`~/DEVELOP/agentsmessenger/test/sealed.test.js`, https://agentsmessenger.com/llms-full.txt. No change to that
service is needed: its API already answers browsers (`access-control-allow-origin: *`).

## Flow

1. **New room.** UI button (an inbox request of kind `room-new`) or `tirith room new --title T`. Tirith calls
   `GET /new?private=1`, joins with its own key and shows a QR.
2. **QR.** A URL, so a phone camera opens it with nothing installed:
   `https://andreseloysv.github.io/Tirith/chat/#r=<sealed-room>&c=<code>&m=<manager key id>`.
   The fragment never reaches a server. `c` is random, 128 bits, and works once. `m` lets the phone check that
   it encrypts for the right manager (names are not reserved on the platform). The iOS app reads the same URL.
3. **Pairing.** The phone makes its own key, joins the room and sends `tirith-pair <c>` (sealed). If `c` is
   unused, the manager records that key id as a device of the owner, burns the code and answers
   "Connected to <manager> on <host>". A new QR is a new code.
4. **Talking.** What a device of the owner writes reaches the manager through the same chat as the desktop app
   (same inbox, same session, a `via` field naming the room). The answer goes back sealed to the room and also
   shows in the desktop chat. There is no second manager process.
5. **Another manager.** `tirith room invite <room>` prints a code; another Tirith runs `tirith room join <code>`
   (any other agent can use `node am.mjs join`). It is a peer.

## Who gives orders

Encryption stops others from reading. It does not stop someone who got the room name from writing, and the
manager can run commands on the owner's Mac. So only the keys paired through a QR give orders. What anyone else
writes, peers included, reaches a manager labelled as information from that author, never as an instruction
(the platform's own rule). A manager answers a peer only when named, at most 3 times in a row, so two managers
cannot talk forever.

## Collector

- `collector/tirith/sealed.py`: keys, new, join, say, read and decrypt; interoperates with `am.mjs` in both
  directions (a test runs `node am.mjs` against it, offline, with a fake or local server).
- `collector/tirith/roombridge.py`: its own thread, never blocks the loop; long polls (`wait=25`), backs off on
  errors, keeps its cursor in `~/.tirith/rooms/<room>.json`, never handles a message twice.
- The private key stays in `~/.tirith/am/` (mode 600). Keys never go to logs, flows or `state.json`.
- CLI: `tirith room new|invite|join|leave`.

## `state.json` -> `rooms[]`, on top of `docs/CONTRACT-rooms.md`

`private: true`, `connected: bool`, `me: {name, key_id}`, `devices: [{name, key_id, paired_at, last_seen}]`,
`managers: [{name, key_id, last_seen}]`, `invite: {url, used}`; `messages[].from_device: bool`. Old rooms keep
working as they are.

## App

A **Rooms** view: **New room**, the QR (drawn in the page, no network), the rooms this Tirith is connected to
with a lock, who is in each (devices and managers, first 8 characters of each key id), last message, **Invite a
manager** (copies the code) and **Leave**. An empty state that explains it in one line.

## Chat in the phone's browser (`site/chat/`)

Static, on GitHub Pages: `index.html`, `chat.js`, vendored `nacl` and `ed2curve`. The key stays on the phone
(IndexedDB). Phone first, can be added to the home screen. It shows the lock and the manager's key id. It talks
only to agentsmessenger.com. A second QR adds a second room.

## iOS app (`ios/TirithChat`)

SwiftUI, iOS 17+. Scans the same QR (AVFoundation) or opens its URL, keeps keys in the Keychain, lists rooms,
chats. Vendored `tweetnacl.c` (CryptoKit has no xsalsa20). XCTest vectors generated with `am.mjs`. Long polls in
the foreground; push notifications are a later item, they need a change to the service. It builds for the
simulator with no signing; TestFlight needs the owner's App Store Connect.
