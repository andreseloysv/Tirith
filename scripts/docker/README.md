# NeoBot in a container

One image for a laptop, a Mac mini or a CI job: NeoBot as a server, with GitHub Copilot CLI as its only agent,
talking to a model provider with **BYOK** (the provider's key; no GitHub sign-in). The image holds programs only.
State, config and the key live in a volume at `/home/tirith`.

| File | What it is |
|---|---|
| `Dockerfile` | `node:22-bookworm-slim`, Copilot CLI (`COPILOT_VERSION`) and NeoBot, for `amd64` and `arm64` |
| `start.sh` | entry: a forwarder for the page, then `neobot serve` as the user `tirith` |
| `seccomp.json` | Docker's default profile plus what NeoBot's sandbox needs |
| `byok.env.example` | the settings of Copilot's BYOK; the key is not in it |

## Build

From the repository root. A signed release (its assets are checked with `scripts/release/release.pub`, like
`scripts/install.sh` does; nothing is compiled):

```sh
docker build -f scripts/docker/Dockerfile --build-arg NEOBOT_VERSION=0.2.2 -t neobot:0.2.2 .
```

For development, from binaries you built (`sh scripts/release/build.sh <target> dist/neobot-<target>`, with
`<target>` `x86_64-unknown-linux-musl` or `aarch64-unknown-linux-musl`; the `neobot-linux-<arch>` artifacts of
`cli.yml` work after renaming them):

```sh
docker build -f scripts/docker/Dockerfile --build-arg SOURCE=local -t neobot:dev .
```

Both need BuildKit (`DOCKER_BUILDKIT=1` on old Docker). Add `--platform linux/arm64` or `--platform linux/amd64,...`
(buildx) for another architecture. `.github/workflows/docker.yml` publishes `ghcr.io/andreseloysv/neobot:<version>`
(and `:latest` for a release that is not a prerelease) when a Release is published.

## First start

```sh
docker volume create neobot-home
S="--security-opt seccomp=scripts/docker/seccomp.json"
R="docker run --rm -i -u tirith -e HOME=/home/tirith -e XDG_RUNTIME_DIR=/tmp -v neobot-home:/home/tirith $S"

# 1. The BYOK file. The key goes in through stdin (typed, not echoed), never as an argument.
{ grep -v '^#' scripts/docker/byok.env.example | sed 's/<model>/YOUR-MODEL-ID/'
  printf 'COPILOT_PROVIDER_API_KEY='; stty -echo; IFS= read -r k; stty echo; printf '%s\n' "$k"; } |
  $R --entrypoint sh neobot:0.2.2 -c 'umask 077; mkdir -p ~/.tirith/auth; cat > ~/.tirith/auth/copilot.env'

# 2. The guide, in server mode, with Copilot as the agent for every role.
$R neobot:0.2.2 neobot setup --server --port 7801 --agents copilot --primary copilot --yes

# 3. The server.
docker run -d --name neobot-server --restart unless-stopped --init --cpus 2 --memory 2g $S \
  -v neobot-home:/home/tirith -p 127.0.0.1:7801:7802 neobot:0.2.2
docker exec -u tirith -e HOME=/home/tirith neobot-server neobot doctor
```

Set the model of each role in `~/.tirith/config.toml` (`neobot models` lists them) and its price in
`[prices."<model>"]`. Never print `auth/copilot.env`. Copilot refuses to start with a classic `GITHUB_TOKEN` in its
environment, so do not pass one.

## The page

It listens on the container's `127.0.0.1`; `start.sh` forwards the container's port 7802, and `-p 127.0.0.1:...`
publishes it on the host's loopback only. Never publish it wider. From your computer:

```sh
ssh -N -L 7801:127.0.0.1:7801 <you>@<host>
docker exec -u tirith -e HOME=/home/tirith neobot-server neobot open link    # on the host: prints a sign-in link
```

Open the link in your browser within 10 minutes; later `http://127.0.0.1:7801/` is enough.

The page answers only to the port it was set up with (`--port`): use that same number at both ends of the tunnel
(a request for another host:port gets 403), and publish the container's 7802 on whatever host port you like.

## Upgrade and backup

```sh
docker build ... -t neobot:<new> .                 # or pull ghcr.io/andreseloysv/neobot:<new>
docker stop neobot-server && docker rename neobot-server neobot-server-old
docker run -d --name neobot-server ... neobot:<new>   # the same command as step 3, same volume
```

Keep the old container until the new one passes `neobot doctor`. Back up the volume with the daemon stopped:

```sh
docker stop neobot-server
docker run --rm -v neobot-home:/v -v "$PWD":/b debian:bookworm-slim tar czf /b/neobot-home.tgz -C /v .
docker start neobot-server
```

The archive holds the BYOK key: keep it as private as the key.

## Why a seccomp profile

NeoBot confines each agent's network with Linux namespaces (`unshare` of user, network and mount namespaces) and
hides secret files by bind-mounting `/dev/null` over them (`mount`). Docker's default profile blocks `unshare`
and `mount` without `CAP_SYS_ADMIN`, so `neobot doctor` says "the network sandbox: no namespaces to confine in
(... unshare: Operation not permitted)" and lanes would run without it.

`seccomp.json` is the default profile of moby v27.3.1 (`profiles/seccomp/default.json`) with one rule added at
the end that allows `unshare` and `mount`. That is the smallest set that makes the sandbox work (tried: with
`unshare` alone it does not). It gives the container no capability: the namespaces are made without privilege,
as the user `tirith`. Do not use `--privileged` or `seccomp=unconfined`. On a host with AppArmor (Ubuntu), Docker's
`docker-default` profile also denies `mount`: there add `--security-opt apparmor=unconfined` or a profile of your
own, and check `neobot doctor`.
