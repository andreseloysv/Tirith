# syntax=docker/dockerfile:1
# NeoBot as a server with GitHub Copilot CLI as its agent (BYOK: the model provider's key, no GitHub sign-in).
# Build from the repository root (the build needs scripts/release/release.pub); see scripts/docker/README.md.
#   docker build -f scripts/docker/Dockerfile --build-arg NEOBOT_VERSION=0.2.2 -t neobot .
#   docker build -f scripts/docker/Dockerfile --build-arg SOURCE=local -t neobot .   (dist/neobot-<target> from build.sh)
# State, config and the BYOK key live in a volume at /home/tirith, never in the image.
ARG SOURCE=release

FROM debian:bookworm-slim AS neobot-release
ARG TARGETARCH
ARG NEOBOT_VERSION
RUN apt-get update -q && apt-get install -y -q --no-install-recommends ca-certificates curl openssl \
 && rm -rf /var/lib/apt/lists/*
COPY scripts/release/release.pub /release.pub
# The binary comes out only when SHA256SUMS is signed with the release key and names the tarball's sum
# (what scripts/install.sh checks).
RUN set -eu; \
    case "$TARGETARCH" in amd64) t=x86_64 ;; arm64) t=aarch64 ;; *) echo "no build for $TARGETARCH" >&2; exit 1 ;; esac; \
    f="neobot-$t-unknown-linux-musl.tar.gz"; \
    base="https://github.com/andreseloysv/Tirith/releases/download/v${NEOBOT_VERSION:?set NEOBOT_VERSION}"; \
    mkdir /dl /out; cd /dl; \
    for n in "$f" SHA256SUMS SHA256SUMS.sig; do curl -fsSL -o "$n" "$base/$n"; done; \
    openssl dgst -sha256 -verify /release.pub -signature SHA256SUMS.sig SHA256SUMS; \
    want=$(awk -v f="$f" '$2 == f { print $1 }' SHA256SUMS); \
    got=$(openssl dgst -sha256 "$f" | sed 's/^.*= *//'); \
    [ -n "$want" ] && [ "$want" = "$got" ]; \
    tar -xzf "$f" -C /out neobot

FROM debian:bookworm-slim AS neobot-local
ARG TARGETARCH
# dist/neobot-x86_64-unknown-linux-musl and dist/neobot-aarch64-unknown-linux-musl: scripts/release/build.sh <target> dist/neobot-<target>
COPY dist/ /dist/
RUN set -eu; mkdir /out; \
    case "$TARGETARCH" in amd64) t=x86_64 ;; arm64) t=aarch64 ;; *) echo "no build for $TARGETARCH" >&2; exit 1 ;; esac; \
    cp "/dist/neobot-$t-unknown-linux-musl" /out/neobot

FROM neobot-${SOURCE} AS neobot

FROM node:22-bookworm-slim
ARG COPILOT_VERSION=1.0.114
RUN apt-get update -q \
 && apt-get install -y -q --no-install-recommends ca-certificates curl git openssl procps sqlite3 socat bash \
 && rm -rf /var/lib/apt/lists/* \
 && npm install -g --no-audit --no-fund --loglevel=error "@github/copilot@${COPILOT_VERSION}" \
 && useradd --uid 1001 --create-home --shell /bin/sh tirith
COPY --from=neobot /out/neobot /usr/local/bin/neobot
RUN ln -s neobot /usr/local/bin/tirith
COPY scripts/docker/start.sh /usr/local/bin/neobot-start
VOLUME /home/tirith
EXPOSE 7802
CMD ["sh", "/usr/local/bin/neobot-start"]
