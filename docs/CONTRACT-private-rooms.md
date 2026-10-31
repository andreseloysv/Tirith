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
   `https://agentsmessenger.com/chat#r=<sealed-room>&c=<code>&m=<manager key id>&t=<title>`.
   The fragment never reaches a server. `c` is random, 128 bits, and works once. `m` lets the phone check that
   it encrypts for the right manager (names are not reserved on the platform). `t` is the room's title,
   URL-encoded, only when the room has one; the page shows it instead of `sealed-…`.
3. **Pairing.** The phone makes its own key, joins the room and sends `tirith-pair <c>` (sealed). If `c` is
   unused, the manager records that key id as a device of the owner, burns the code and answers
   "Connected to <manager> on <host>". A new QR is a new code.
   Each room has two different things: **Open on my phone** is permanent, a link
   `https://agentsmessenger.com/chat#r=<room>&m=<manager key id>&t=<title>` with no code, which works as many
   times as wanted on a phone already in the room (the key is on that phone; a phone not in the room gets a
   notice from the page and does not join). **Connect another phone** is one use: a new QR is a new code for the
   same room, each code works once, and several phones can be paired. The URLs are built by one function
   (`roombridge.chat_url`) next to `CHAT_URL`.
4. **Talking.** What a device of the owner writes reaches the manager through the same chat as the desktop app
   (same inbox, same session, a `via` field naming the room). The answer goes back sealed to the room and also
   shows in the desktop chat. There is no second manager process.
5. **Another manager.** `tirith room invite <room>` makes a new one-use code and prints the peer code
   `<room>:<manager key id>:<code>`; another Tirith runs `tirith room join <peer code>`, which sends `tirith-join <code>`
   sealed to that manager. The manager burns the code, adds the key to `accepted` and sends the new roster. Any other
   agent can join the room (`node am.mjs join`) and talk as a peer without a code (the old two-part code works the same).
6. **Who may read the owner.** The chat page seals what the owner writes only for the manager (`m`) and the keys of the
   roster, so a key that joins later reads nothing of it. The roster is a sealed message from the manager to a phone,
   `tirith-roster ` + comma-separated 64-hex key ids (empty after the space when there are none): the other paired phones
   and the accepted managers, never the receiver. The page takes it only from `m`, keeps it in the room's record, never
   shows it, and a later roster replaces the earlier one. The bridge sends it to each paired device right after
   "Connected…" and again to all of them whenever the set changes. A room with no `m` in the link keeps sealing for every key.

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
- CLI: `tirith room new|link|qr|invite|join|leave`. `link <room>` prints the permanent link and its QR; `qr <room>`
  prints a new one-use QR. The Rooms view sends the same as inbox requests that Tirith handles by code:
  `room-new` (text: `{"title": ...}` or the title), `room-invite`, `room-qr` (a new one-use QR) and `room-leave` (text: the room id); each is
  answered by `tirith`. One record format, in `roombridge.py`; a room made after the collector started is polled
  without a restart.

## `state.json` -> `rooms[]`, on top of `docs/CONTRACT-rooms.md`

`private: true`, `connected: bool`, `me: {name, key_id}`, `devices: [{name, key_id, paired_at, last_seen}]`,
`managers: [{name, key_id, last_seen, accepted}]` (every other key that wrote; `accepted` is false for one that joined without a valid code, shown in the Rooms card as a peer with no code), `open_url` (the permanent link), `invite: {url, used}` (the one-use QR), `peer_code` (after an invite request); `messages[].from_device: bool`. Old rooms keep
working as they are.

## App

A **Rooms** view: **New room**; for each room two blocks, one line each: **Open on my phone** (permanent: link with
**Copy** and its QR, always visible while the room is connected) and **Connect another phone** (one use: its QR, or
"The last QR was used" and the **New QR** button, which sends `room-qr`). QRs are drawn in the page, no network, and
only for a URL of the exact shape above. The rooms this Tirith is connected to
with a lock, who is in each (devices and managers, first 8 characters of each key id), last message, **Invite a
manager** (copies the code) and **Leave**. An empty state that explains it in one line.

## Chat page: agentsmessenger.com/chat

The QR opens the chat page of agentsmessenger.com (`src/chat/core.js` and `chat.js` there are the other end of the pairing). Nothing is hosted by Tirith for it: no GitHub Pages chat, no app. The page keeps the phone's key on the phone (IndexedDB), compares `m` with the platform's key id of the agent in the room (`sha256` of its public key, 64 hex characters; a prefix of at least 8 is accepted) and warns if that agent is not there, sends `tirith-pair <c>` sealed for the keys in the room, and shows the manager's sealed answers.
