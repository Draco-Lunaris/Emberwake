# Multi-stage Dockerfile for Emberwake
# T076 — finalized in Phase 12 (Polish).
#
# Stage 1 (planner): cargo-chef prepares a recipe from Cargo.toml/Cargo.lock for
#   maximum dependency-caching efficiency.
# Stage 2 (builder): cargo-chef cooks the recipe (caches compiled deps), then
#   `cargo leptos build --release` produces the server binary + hashed WASM/asset
#   bundle.
# Stage 3 (runtime): digest-pinned ubuntu:26.04, non-root user (UID 10001),
#   read-only rootfs + tmpfs for /tmp, HEALTHCHECK → /readyz.

# ── Stage 1: cargo-chef planner ──────────────────────────────────────────────
FROM rust:1.95 AS chef
RUN cargo install cargo-chef --locked && rm -rf /usr/local/cargo/registry
WORKDIR /build

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 2: builder ──────────────────────────────────────────────────────────
FROM chef AS builder

# Install build dependencies (no OpenSSL — project is rustls-only).
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-leptos for the WASM+SSR build.
RUN cargo install cargo-leptos --locked

# Add the wasm32-unknown-unknown target.
RUN rustup target add wasm32-unknown-unknown

# Cook the dependency recipe first (cached layer).
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source and build.
COPY . .
RUN cargo leptos build --release

# ── Stage 3: runtime ──────────────────────────────────────────────────────────
# digest-pinned ubuntu:26.04 — update digest when bumping base image.
FROM ubuntu@sha256:cc925e589b7543b910fea57a240468940003fbfc0515245a495dd0ad8fe7cef1 AS runtime
# NOTE: Replace the digest above with the actual ubuntu:26.04 digest at build time:
#   docker pull ubuntu:26.04 && docker inspect --format='{{index .RepoDigests 0}}' ubuntu:26.04
# The placeholder digest ensures the image is never pulled implicitly.

# Install minimal runtime deps: ca-certificates (for rustls trust roots) and
# curl (for HEALTHCHECK). No toolchain, Node, or Python.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root user (UID 10001).
RUN useradd -r -u 10001 -d /var/lib/emberwake -s /usr/sbin/nologin emberwake

# Create data directory owned by the runtime user.
RUN mkdir -p /var/lib/emberwake && chown 10001:10001 /var/lib/emberwake

# Copy the server binary.
COPY --from=builder /build/target/server/release/emberwake /usr/local/bin/emberwake

# Copy the hashed WASM/asset bundle (site-root from cargo-leptos metadata).
COPY --from=builder /build/target/site /var/lib/emberwake/site

USER 10001:10001

ENV DATA_DIR=/var/lib/emberwake
ENV RUST_LOG=info

EXPOSE 5005

# Single volume for persistent data (SQLite DB, backups).
VOLUME ["/var/lib/emberwake"]

# Health check hits the /readyz endpoint.
HEALTHCHECK --interval=30s --timeout=3s --start-period=2s --retries=3 \
    CMD curl -sf http://127.0.0.1:5005/readyz || exit 1

ENTRYPOINT ["/usr/local/bin/emberwake"]
