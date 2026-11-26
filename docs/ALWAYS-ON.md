# NeoBot always on, on a Mac mini

A Mac mini on mains power can run NeoBot day and night, with no screen, and your phone reaches it from anywhere.
It needs five things:

1. [The daemon starts by itself](#1-the-daemon-starts-by-itself): `neobot service install`.
2. [The Mac does not sleep](#2-the-mac-does-not-sleep): `[power] keep_awake = true` and one switch in Energy.
3. [It starts after a power cut](#3-it-starts-after-a-power-cut): another setting in Energy.
4. [Someone is logged in](#4-someone-is-logged-in): automatic login, **which does not exist with FileVault: after
   a power cut you log in by hand**.
5. [The phone is paired](#5-the-phone-is-paired): one QR, no open port.

Names of settings are given in English, then as a Mac in Spanish shows them.

## 1. The daemon starts by itself

If NeoBot already runs (started by `./run` or by the app), stop it first, when no lane is working, because two
daemons cannot share the port (7717 unless `[server] port` says otherwise):

```bash
neobot status                                        # wait for "no lane is working"
kill $(lsof -t -iTCP:7717 -sTCP:LISTEN)
```

Then, as the user who will run NeoBot (no `sudo`):

```bash
neobot service install
neobot service status                                # how it runs, and alive: its own process answers
```

This writes `~/Library/LaunchAgents/com.tirith.serve.plist`: launchd starts the daemon at login and again whenever
it ends. Its log is `~/.tirith/logs/tirithd.log`. The service runs the binary you installed it with, so install it
again if that binary moves. `neobot service uninstall` undoes it.

It is a LaunchAgent, not a LaunchDaemon, because Safari, Edge and the keychain need your session. So it runs only
while you are logged in: a locked screen is fine, logging out stops it.

## 2. The Mac does not sleep

In `~/.tirith/config.toml`:

```toml
[power]
keep_awake = true
```

It is `false` by default and is read when the daemon starts, so restart it, when no lane is working:

```bash
launchctl kickstart -k gui/$(id -u)/com.tirith.serve
pmset -g assertions | grep caffeinate                # a line here: the Mac is held awake
```

The daemon runs `caffeinate -i -s -w <its pid>`: no sleep while it lives (`-s` counts on mains power only), and
normal sleep again when it ends. The display may still turn off.

That does not cover the time without a daemon (an update, the login window), so also turn this on in
**System Settings > Energy** (*Ajustes del Sistema > Energía*):

- **Prevent your Mac from automatically sleeping when the display is off** («Evitar el reposo automático», shown as
  *Impedir que el Mac entre en reposo automáticamente al apagar la pantalla*).

### A MacBook is not a Mac mini

**A MacBook with the lid closed and no external display goes to sleep**, with or without `keep_awake`: `caffeinate`
does not stop that. Leave the lid open, or connect an external display, the power adapter and a keyboard and mouse
before closing it. Its sleep switch is in System Settings > Battery > Options (*Batería > Opciones*).

## 3. It starts after a power cut

In the same **Energy** pane, turn on:

- **Start up automatically after a power failure** («Arrancar tras un corte de luz», shown as
  *Arrancar automáticamente tras un corte de suministro*). Where macOS offers a choice instead of a switch, pick
  **After power failure** (*Tras un corte de suministro*) or **Always** (*Siempre*).

From a terminal, and to check it:

```bash
sudo pmset -a autorestart 1
pmset -g custom | grep autorestart
```

This only turns the Mac on. NeoBot starts when you are logged in, which is the next step.

## 4. Someone is logged in

`fdesetup status` says whether FileVault is on. Choose one:

| | FileVault on | FileVault off |
|---|---|---|
| The disk | locked until someone types the password | readable by whoever has the Mac |
| Automatic login | does not exist | can be turned on |
| After a power cut | the Mac waits: log in by hand | NeoBot comes back by itself |

**With FileVault off:** System Settings > Users & Groups > **Automatically log in as** (*Usuarios y grupos >
Iniciar sesión automáticamente como*), and choose the user who installed the service. Anyone who can touch the Mac
can then get into that session, so this is for a Mac in a place you trust.

**With FileVault on:** after every power cut or restart, also one for a macOS update, type the password at the Mac.
Nothing of NeoBot runs until then and the phone gets no answer. Since macOS 26, with Remote Login on, SSH from your
own network can unlock the disk (`man apple_ssh_and_filevault`), but that opens no session: you still log in at the
Mac or by Screen Sharing on your own network. Do not open either of them to the internet for this.

## 5. The phone is paired

Open the **Rooms** page on the Mac and scan the QR with the phone, once. It gets a chat with the manager of every
project, also of those you add later, and works from any network, **with nothing on the Mac open to the internet**:

- The daemon listens on `127.0.0.1` only. Neither the internet nor your own network can connect to it.
- Phone and Mac both connect outwards to agentsmessenger.com over HTTPS, and the Mac waits there for new messages.
  It needs no port forwarded on the router, public IP, VPN or tunnel.
- The private rooms are end-to-end encrypted, and only a paired phone speaks as you. The QR works once and for
  fifteen minutes.
- The rooms and the pairing survive restarts: when the daemon is back it reads on from the last message it saw.

Do not forward port 7717 and do not put a reverse proxy in front of it: whatever reaches that port can give your
agents orders. If a phone is lost, `neobot room forget <phone key id>` cuts it off.

## A Mac older than macOS 13.5: in Docker

GitHub Copilot CLI's native binary needs macOS 13.5. On an older Intel Mac mini NeoBot runs, with Copilot as its
agent, in a Linux container of Docker Desktop; the image is `scripts/docker/` (its README has the commands).

- The agent is Copilot with BYOK: the model provider's key in the volume, no GitHub sign-in. State, config and the
  key live in the volume `neobot-home`, so an upgrade replaces the container and keeps them.
- Publish the page on the Mac's loopback only (`-p 127.0.0.1:7801:7802`) and reach it from your computer with
  `ssh -N -L 7801:127.0.0.1:7801 <you>@<mac>`. Nothing is opened to the network.
- `--restart unless-stopped` brings it back with Docker. Docker Desktop itself starts when someone is logged in
  (step 4 above), so this guide's steps 2 to 4 apply to it too.
- Docker's default seccomp profile blocks NeoBot's network sandbox; use `scripts/docker/seccomp.json`.
- Give Docker Desktop's VM enough memory and CPUs (Settings > Resources) for the container's limits.

## Try it before you leave

1. When no lane is working, restart the Mac and do not open NeoBot. With FileVault, log in.
2. `neobot service status` says alive, and `pmset -g assertions | grep caffeinate` prints a line.
3. Turn the phone's Wi-Fi off and send `/models` in a project's chat: the Mac answers without calling a model.

If the phone gets no answer, look in this order: the Mac has power and internet, someone is logged in,
`neobot service status`, then `tail -50 ~/.tirith/logs/tirithd.log` (`port 7717 is taken` means that another daemon
was started by hand: stop it as in step 1 and launchd starts its own).
