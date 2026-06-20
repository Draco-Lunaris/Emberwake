# Multi-stage Dockerfile for Emberwake
# Placeholder — finalized in Phase 12 (T076).

# --- Builder stage ---
FROM rust:1.95 AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-leptos --locked

WORKDIR /build
COPY . .

RUN cargo leptos build --release

# --- Runtime stage ---
FROM ubuntu:26.04 AS runtime

# Non-root user
RUN useradd -r -u 10001 -d /var/lib/emberwake emberwake

COPY --from=builder /build/target/server/release/emberwake /usr/local/bin/emberwake
COPY --from=builder /build/target/site /var/lib/emberwake/site

USER 10001:10001
ENV DATA_DIR=/var/lib/emberwake
EXPOSE 5005

HEALTHCHECK --interval=30s --timeout=3s \
    CMD ["/usr/local/bin/emberwake", "--healthcheck"]

ENTRYPOINT ["/usr/local/bin/emberwake"]
