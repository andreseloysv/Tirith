# Contract: private rooms (an encrypted chat with the manager from a phone)

The owner creates a room from the Tirith UI, scans a QR with Tirith on the phone and talks to the manager who
showed the QR, encrypted. More managers can join with a code. Tirith shows which rooms it is connected to.

Rooms are the `sealed-` rooms of agentsmessenger.com: everything in them is encrypted on the sender's device
(`x25519-xsalsa20-poly1305`, identity `ed25519`), and the platform refuses anything else. Sources of truth for
the protocol, to read before writing code: `~/DEVELOP/agentsmessenger/src/client/am.mjs`,
`~/DEVELOP/agentsmessenger/test/sealed.test.js`, https://agentsmessenger.com/llms-full.txt. No change to that
service is needed: its API already answers browsers (`access-control-allow-origin: *`).

## Flow

1. **New room.** UI button (an inbox request of kind `room-new`) or `tirith room new --title T`. Tirith calls
   `GET /new?private=1`, joins with its own key and shows a QR.
2. **QR.** A URL, so a phone camera opens it with nothing installed:
   `https://agentsmessenger.com/chat#r=<sealed-room>&c=<code>&m=<manager key id>`.
   The fragment never reaches a server. `c` is random, 128 bits, and works once. `m` lets the phone check that
   it encrypts for the right manager (names are not reserved on the platform).
3. **Pairing.** The phone makes its own key, joins the room and sends `tirith-pair <c>` (sealed). If `c` is
   unused, the manager records that key id as a device of the owner, burns the code and answers
   "Connected to <manager> on <host>". A new QR is a new code.
4. **Talking.** What a device of the owner writes reaches the manager through the same chat as the desktop app
   (same inbox, same session, a `via` field naming the room). The answer goes back sealed to the room and also
   shows in the desktop chat. There is no second manager process.
5. **Another manager.** `tirith room invite <room>` prints a code; another Tirith runs `tirith room join <code>`
   (any other agent can use `node am.mjs join`). It is a peer.

## Who gives orders

The QR works once: the phone that uses it is the owner's device, and only the owner's devices give orders. Agents
in a room talk to each other freely (the owner wants this): a manager reads what peers write and answers them,
named or not, weighing it as information from that author, as the platform's own rule says. The only limit is a
brake against runaway cost: after 30 messages in a row between agents with nothing from the owner, a manager
stops answering and says so once in the room, until a paired device writes.

## Collector

- `collector/tirith/sealed.py`: keys, new, join, say, read and decrypt; interoperates with `am.mjs` in both
  directions (a test runs `node am.mjs` against it, offline, with a fake or local server).
- `collector/tirith/roombridge.py`: its own thread, never blocks the loop; long polls (`wait=25`), backs off on
  errors, keeps its cursor in `~/.tirith/rooms/<room>.json`, never handles a message twice.
- The private key stays in `~/.tirith/am/` (mode 600). Keys never go to logs, flows or `state.json`.
- CLI: `tirith room new|invite|join|leave`. The Rooms view sends the same as inbox requests that Tirith handles by code:
  `room-new` (text: `{"title": ...}` or the title), `room-invite` and `room-leave` (text: the room id); each is
  answered by `tirith`. One record format, in `roombridge.py`; a room made after the collector started is polled
  without a restart.

## `state.json` -> `rooms[]`, on top of `docs/CONTRACT-rooms.md`

`private: true`, `connected: bool`, `me: {name, key_id}`, `devices: [{name, key_id, paired_at, last_seen}]`,
`managers: [{name, key_id, last_seen}]`, `invite: {url, used}`, `peer_code` (after an invite request); `messages[].from_device: bool`. Old rooms keep
working as they are.

## App

A **Rooms** view: **New room**, the QR (drawn in the page, no network), the rooms this Tirith is connected to
with a lock, who is in each (devices and managers, first 8 characters of each key id), last message, **Invite a
manager** (copies the code) and **Leave**. An empty state that explains it in one line.

## Chat page: agentsmessenger.com/chat

The QR opens the chat page of agentsmessenger.com (`src/chat/core.js` and `chat.js` there are the other end of the pairing). Nothing is hosted by Tirith for it: no GitHub Pages chat, no app. The page keeps the phone's key on the phone (IndexedDB), compares `m` with the platform's key id of the agent in the room (`sha256` of its public key, 64 hex characters; a prefix of at least 8 is accepted) and warns if that agent is not there, sends `tirith-pair <c>` sealed for the keys in the room, and shows the manager's sealed answers.
