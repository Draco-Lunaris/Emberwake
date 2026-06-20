# Quickstart: Emberwake

**Branch**: `001-greenfield` | **Date**: 2026-06-19

Developer and operator workflow. Doubles as the `quickstart.md` validation in the Polish phase.

## Prerequisites

- Rust stable 1.95+ (pinned by `rust-toolchain.toml`), edition 2024
- `cargo-leptos` (`cargo install cargo-leptos`) and the `wasm32-unknown-unknown` target
- Docker with buildx (multi-arch images)
- Dev-only tooling: `sqlx-cli`, `cargo-deny`, `cargo-audit`, `cargo-fuzz`, a WebDriver
  (geckodriver/chromedriver) for E2E

## Local development

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos sqlx-cli --locked

# create the dev database + run migrations
export DATABASE_URL="sqlite://./data/app.db"
sqlx database create
sqlx migrate run

# run with hot reload (SSR server + WASM rebuild) on :5005
DATA_DIR=./data cargo leptos watch
```

First load shows the **first-run setup** flow; complete it to create the admin account (the
setup route closes afterward). Then sign in and start adding services/bookmarks.

## Test & security gates

```bash
cargo clippy --all-targets --all-features -- -D warnings   # Principle I/II gate
cargo leptos test                                          # server-fn + integration tests
cargo deny check                                           # advisories, licenses, bans
cargo audit                                                # RUSTSEC advisories
cargo +nightly fuzz run import_html -- -max_total_time=60  # parser fuzz smoke (also json, opml)

# end-to-end (login/create/search) against a headless browser
cargo test -p e2e
```

`#[sqlx::test]` gives each data test an isolated database; the server-function tests drive the
real Axum router so auth/CSRF/headers are exercised, not bypassed.

## Build the container

```bash
# local single-arch
docker build -t emberwake -f .docker/Dockerfile .

# multi-arch (amd64 + arm64)
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f .docker/Dockerfile \
  -t ghcr.io/draco-lunaris/emberwake:dev .
```

The Dockerfile runs `cargo-leptos build --release`, copies the server binary plus the hashed
asset/WASM bundle into a digest-pinned `ubuntu:26.04` runtime, and runs as a non-root user.

## Run the container

```bash
docker run -p 5005:5005 \
  -v /path/to/data:/var/lib/emberwake \
  -e DATA_DIR=/var/lib/emberwake \
  -e RUST_LOG=info \
  --user 10001:10001 \
  --read-only --tmpfs /tmp \
  ghcr.io/draco-lunaris/emberwake:latest
```

Secrets via the `*_FILE` convention (file value wins, fail-loud if unreadable):

```yaml
services:
  emberwake:
    image: ghcr.io/draco-lunaris/emberwake:latest
    container_name: emberwake
    user: "10001:10001"
    read_only: true
    tmpfs: [/tmp]
    volumes:
      - emberwake-data:/var/lib/emberwake
      - /var/run/docker.sock:/var/run/docker.sock:ro   # only if Docker discovery is enabled
    ports: ["5005:5005"]
    environment:
      - DATA_DIR=/var/lib/emberwake
      - SESSION_SECRET_FILE=/run/secrets/session_secret
      - WEATHER_API_KEY_FILE=/run/secrets/weather_key   # optional
    secrets: [session_secret, weather_key]
    restart: unless-stopped
volumes: { emberwake-data: {} }
secrets:
  session_secret: { file: ./secrets/session_secret }
  weather_key:    { file: ./secrets/weather_key }
```

Optional built-in HTTPS (no reverse proxy): set the ACME env (domain + email + cache dir) and
the server obtains/renews certs via rustls-acme; otherwise terminate TLS at your proxy.

## Acceptance walkthrough (smoke the MVP + key slices)

1. Fresh volume → first-run setup creates the admin; setup route then 404s (US3).
2. Sign in → secure session cookie set; wrong password is throttled (US3).
3. Create a category, add a service + bookmark, drag to reorder, pin → optimistic UI, persists
   across reload (US2); page first-paint already contains pinned items (US1, view source).
4. Type a misspelled service name → fuzzy match ranks it first, no network call (US1).
5. Save a theme + custom CSS → applied on reload with no flash of default (US5).
6. (Optional) Enable status monitoring on a service, open `/events` in the UI → tile flips
   live when the target goes down (US6); same machinery powers weather (US7).
7. Export all data → import into a fresh instance → equivalent data set (US9); feed a
   malformed bookmarks file → rejected before any write.

## Release

Tagging `vX.Y.Z` triggers CI: clippy + tests + `cargo-deny` + `cargo-audit` + fuzz smoke must
pass on amd64 and arm64; the release job builds the multi-arch image, generates an SBOM,
pushes to GHCR, and signs the image with cosign. An unsigned or advisory-flagged build does
not publish.
