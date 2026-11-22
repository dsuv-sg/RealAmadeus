# syntax=docker/dockerfile:1
#
# RealAmadeus V2 — Web version image: amadeus-server + the Trunk-built WASM frontend.
#
#   docker build -t realamadeus-web .
#   docker build --build-arg AMADEUS_BASE_PATH=/amadeus-v2/ -t realamadeus-web:v2-preview .
#   docker run -d -p 127.0.0.1:9502:9502 --env-file .env -v amadeus-data:/data realamadeus-web
#   curl -fsS http://127.0.0.1:9502/amadeus/healthz      # -> ok
#
# Build context: the RealAmadeusRust workspace root. Published by .github/workflows/release.yml
# as ghcr.io/<owner>/realamadeus-web:<version> and :latest. See deploy/README.md.

ARG RUST_VERSION=1
ARG DEBIAN_SUITE=bookworm

# ------------------------------------------------------------------------------------------
# Builder
# ------------------------------------------------------------------------------------------
FROM rust:${RUST_VERSION}-${DEBIAN_SUITE} AS builder

ARG TRUNK_VERSION=0.22.1
# URL prefix the frontend is built for (Trunk public_url). The server's AMADEUS_BASE_PATH is
# set to the same value in the runtime stage. Leading/trailing slashes are optional.
ARG AMADEUS_BASE_PATH=/amadeus/

# .dockerignore drops rust-toolchain.toml (it only pins "stable"), so the image's stable
# toolchain is used instead of downloading a second one.
RUN rustup target add wasm32-unknown-unknown

# Prebuilt, statically linked Trunk, verified against the published checksum.
RUN set -eux; \
    case "$(uname -m)" in \
      x86_64)  triple=x86_64-unknown-linux-musl ;; \
      aarch64) triple=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported build architecture: $(uname -m)" >&2; exit 1 ;; \
    esac; \
    url="https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-${triple}.tar.gz"; \
    curl -fsSL --retry 3 -o /tmp/trunk.tar.gz "$url"; \
    expected="$(curl -fsSL --retry 3 "$url.sha256" | cut -d' ' -f1)"; \
    echo "${expected}  /tmp/trunk.tar.gz" | sha256sum -c -; \
    tar -xzf /tmp/trunk.tar.gz -C /usr/local/bin trunk; \
    rm -f /tmp/trunk.tar.gz; \
    trunk --version

WORKDIR /src
COPY . .

# Frontend (wasm-release profile, wasm-opt) and the server (release profile, LTO).
# Trunk downloads wasm-bindgen, wasm-opt and tailwindcss into its cache on first use.
RUN --mount=type=cache,id=realamadeus-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=realamadeus-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=realamadeus-trunk-cache,target=/root/.cache/trunk \
    --mount=type=cache,id=realamadeus-target,target=/src/target \
    set -eux; \
    base="/${AMADEUS_BASE_PATH#/}"; \
    public_url="${base%/}/"; \
    cd /src/apps/web; \
    trunk build --release --public-url "$public_url" --dist /out/dist; \
    cd /src; \
    cargo build --release --locked -p amadeus-server; \
    install -Dm755 target/release/amadeus-server /out/amadeus-server; \
    test -f /out/dist/index.html

# ------------------------------------------------------------------------------------------
# Runtime
# ------------------------------------------------------------------------------------------
FROM debian:${DEBIAN_SUITE}-slim AS runtime

ARG AMADEUS_BASE_PATH=/amadeus/

# ca-certificates: TLS to the LLM providers; curl: HEALTHCHECK.
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates curl; \
    rm -rf /var/lib/apt/lists/*; \
    groupadd --system --gid 10001 amadeus; \
    useradd --system --uid 10001 --gid amadeus --home-dir /app --no-create-home \
        --shell /usr/sbin/nologin amadeus; \
    mkdir -p /app /data; \
    chown amadeus:amadeus /data

WORKDIR /app
COPY --from=builder /out/amadeus-server /app/amadeus-server
COPY --from=builder /out/dist /app/dist
COPY LICENSE /app/LICENSE
COPY deploy/.env.example /app/.env.example

# The server normalizes AMADEUS_BASE_PATH itself (/amadeus/ -> /amadeus), so the Trunk
# spelling of the build argument can be passed through unchanged.
ENV AMADEUS_BIND=0.0.0.0:9502 \
    AMADEUS_BASE_PATH=${AMADEUS_BASE_PATH} \
    AMADEUS_DIST_DIR=/app/dist \
    AMADEUS_DATA_DIR=/data \
    RUST_LOG=info

VOLUME ["/data"]
EXPOSE 9502
USER amadeus:amadeus

# Follows AMADEUS_BIND / AMADEUS_BASE_PATH if they are overridden at `docker run` time; empty
# values mean the server defaults (port 9502, /amadeus), exactly as amadeus-server reads them.
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD bind="${AMADEUS_BIND:-127.0.0.1:9502}"; base="${AMADEUS_BASE_PATH:-/amadeus}"; base="/${base#/}"; \
        curl -fsS "http://127.0.0.1:${bind##*:}${base%/}/healthz" >/dev/null || exit 1

# amadeus-server runs as PID 1 and shuts down gracefully on SIGTERM (`docker stop`).
ENTRYPOINT ["/app/amadeus-server"]

LABEL org.opencontainers.image.title="RealAmadeus Web" \
      org.opencontainers.image.description="RealAmadeus V2 (Rust) web server — a non-commercial Steins;Gate 0 fan project" \
      org.opencontainers.image.source="https://github.com/dsuv-sg/RealAmadeus" \
      org.opencontainers.image.url="https://dsuv.elvelt.com/amadeus" \
      org.opencontainers.image.licenses="CC-BY-NC-4.0"
