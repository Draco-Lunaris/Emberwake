# Phase 0 Research: Emberwake

**Branch**: `001-greenfield` | **Date**: 2026-06-19

Decisions resolving the plan's Technical Context. Each states the choice, why, and the
rejected alternative. Versions reflect the current ecosystem as of mid-2026 (Rust 1.95 /
edition 2024; Leptos 0.8.x).

## Decision 1 — Full-stack framework: Leptos 0.8.x (SSR + hydrate) on Axum

**Decision**: Build the whole app in Leptos with server-side rendering and islands-style
hydration, using typed `#[server]` functions as the client/server boundary, served by Axum.

**Rationale**: Leptos is the most performance-focused, actively developed full-stack Rust
framework: fine-grained reactive signals (no virtual DOM), SSR for instant first paint,
hydration for interactivity, and server functions that erase the hand-maintained API layer —
a contract change becomes a compile error (Principle I). It integrates natively with Axum via
`cargo-leptos`, which also produces the hashed asset/WASM bundle for the single-image build.

**Alternatives rejected**:
- *Dioxus*: excellent and more React-like, with unique cross-platform (web/desktop/mobile)
  reach. Kept as the documented single-swap alternative if native targets ever become a goal;
  not chosen now because Leptos's fine-grained SSR is the stronger fit for a server-first web
  dashboard and its server-function ergonomics are more mature for this shape of app.
- *Yew*: CSR-centric, less momentum, weaker SSR story.
- *Axum + a JS SPA (React/Svelte)*: violates the full-Rust mandate (Principle I) and reintroduces
  a Node toolchain and a JSON contract to drift.

## Decision 2 — Persistence: SQLite (WAL) via SQLx

**Decision**: Embedded SQLite in WAL mode, accessed through SQLx with compile-time-checked
queries and SQL-file migrations (`sqlx migrate`). Wrap data access behind a repository trait.

**Rationale**: Zero external dependencies is a product value for a self-hosted dashboard —
one container, one volume. SQLx gives async access and, crucially, compile-time query
verification, so schema/type drift and many N+1s are build errors (Principles III, IV). WAL
mode plus a small pool gives ample concurrency for this workload. The repository trait keeps
a future Postgres backend a localized change.

**Alternatives rejected**:
- *SeaORM*: convenient relations/migrations, but the query surface here is simple and SQLx's
  compile-time checking is the stronger correctness/security guarantee for a security-first
  build. (SeaORM remains a reasonable swap behind the repository trait.)
- *Diesel*: synchronous core; awkward in the async stack.
- *Postgres by default*: contradicts the zero-dependency product value; deferred behind the
  trait.

## Decision 3 — Identity & sessions: Argon2id + server-side revocable sessions

**Decision**: Local accounts hashed with Argon2id (`argon2` crate, configurable cost).
Sessions are opaque, server-side, rotating, and revocable, stored in the database and carried
in HttpOnly/Secure/SameSite cookies via `tower-sessions` (SQLx store). CSRF protection via
origin checks plus a per-session token on state-changing requests.

**Rationale**: A multi-user, security-first product needs revocable identity, which stateless
JWTs do not give cleanly (no server-side invalidation without extra machinery). Server-side
sessions support immediate sign-out, revoke-all, and rotation on privilege change (mitigating
fixation) — all Principle II requirements. Argon2id is the current password-hashing default;
configurable cost keeps login within the latency budget on arm64.

**Alternatives rejected**:
- *JWT access tokens for browser auth*: revocation and rotation require a denylist/short TTLs
  that recreate server state anyway, with a larger footgun surface (alg confusion, leakage in
  storage). JWTs are used only, if at all, internally — not as the browser session.
- *bcrypt*: acceptable but Argon2id is the stronger default and there is no legacy constraint.

## Decision 4 — Extended auth: OIDC (auth code + PKCE), WebAuthn passkeys, scoped API tokens

**Decision**: Optional OIDC via `openidconnect` (auth code + PKCE, provider discovery);
optional passkeys via `webauthn-rs`; scoped, hashed, revocable API tokens for a small public
REST surface guarded by a bearer middleware.

**Rationale**: Modern identity is the headline upgrade over a single shared password and fits
real SSO/homelab deployments. Auth-code+PKCE is the correct browser OIDC flow; passkeys add
phishing-resistant passwordless login; scoped tokens enable safe automation (least privilege,
Principle II). All three are optional so the base product stays simple.

**Alternatives rejected**:
- *Implicit/hybrid OIDC flows*: deprecated/insecure; rejected.
- *Long-lived unscoped API keys*: violate least privilege; rejected in favor of scoped,
  expiring, revocable tokens.

## Decision 5 — Transport security & headers: rustls, strict CSP, optional ACME

**Decision**: rustls for all TLS (outbound and optional inbound); never link system OpenSSL.
Send a strict nonce/hash-based CSP, HSTS, `X-Content-Type-Options`, frame-deny, and a minimal
referrer policy on every response via a security-headers layer. Optional built-in HTTPS via
rustls + ACME (`rustls-acme`/`instant-acme`) for operators not behind a proxy.

**Rationale**: rustls keeps the artifact portable across the arch matrix and the ubuntu:26.04
runtime and shrinks native attack surface (Principle II/V). A strict CSP with per-response
nonces (Leptos supports nonce propagation) means no `unsafe-inline` and a real XSS backstop.

**Alternatives rejected**:
- *native-tls/OpenSSL*: reintroduces system-library coupling and CVE exposure the rustls path
  avoids.
- *Relaxed CSP with `unsafe-inline`*: defeats the purpose; rejected — first-party assets are
  nonce/hash-allowed instead.

## Decision 6 — Abuse resistance & untrusted input: rate limiting + fuzzed parsers

**Decision**: Per-route rate limiting and login throttling via `tower_governor`. Import
parsers (HTML via `scraper`, JSON via `serde_json`, OPML) enforce size/derivation limits, run
under `spawn_blocking`, and have `cargo-fuzz` targets.

**Rationale**: Login throttling blunts credential stuffing; rate limits protect token/import
routes. Import handles untrusted files, so fuzzing is mandated (Principle IV) to prevent
panics/OOM from malformed or hostile input.

**Alternatives rejected**:
- *No rate limiting / trust-the-proxy*: leaves the app exposed when run directly; rejected.
- *Unbounded parsing*: zip-bomb-style nested bookmark trees → DoS; rejected via limits.

## Decision 7 — Live updates: Server-Sent Events (with WebSocket fallback)

**Decision**: Push status-monitoring and weather updates over SSE (`axum::response::sse`);
use a WebSocket only if bidirectional needs emerge.

**Rationale**: Pushes here are server→client and low-frequency; SSE is simpler, proxy-friendly,
auto-reconnecting, and cheaper than a WS for this pattern. It composes with the auth/session
layer the same way ordinary requests do.

**Alternatives rejected**:
- *WebSocket for everything*: more moving parts (heartbeats, framing) than one-way pushes need.
- *Client polling*: wastes cycles and adds latency the live widgets are meant to remove.

## Decision 8 — Integrations: bollard (Docker, incl. events), kube-rs (Kubernetes), read-only

**Decision**: `bollard` for Docker (list/inspect + the events stream for live discovery),
`kube` + `k8s-openapi` for Ingress enumeration. Both strictly read-only by construction.

**Rationale**: Mature async clients covering exactly the read paths needed. Subscribing to
Docker events modernizes discovery from poll to live (US8) while keeping access read-only
(Principle II). Wiring only list/inspect/watch calls makes mutation impossible by omission.

**Alternatives rejected**:
- *Shelling out to docker/kubectl*: brittle, bloats the image, harder to test.
- *Polling only*: misses the live-discovery upgrade.

## Decision 9 — Observability: tracing + OpenTelemetry + Prometheus

**Decision**: `tracing` with a JSON subscriber (env-configurable level), OTLP export via
`tracing-opentelemetry`/`opentelemetry-otlp`, and a Prometheus `/metrics` endpoint
(`metrics` + an Axum exporter). `/healthz` + `/readyz` back the container healthcheck.

**Rationale**: First-class observability (Principle V) suits a homelab/prod deployment with an
existing metrics/trace stack; structured logs + traces + metrics make the single binary
operable without guesswork.

**Alternatives rejected**:
- *Ad-hoc `println!`/unstructured logs*: not operable at scale; rejected.

## Decision 10 — Build, supply chain & artifact: cargo-leptos, ubuntu:26.04, signed + SBOM

**Decision**: Multi-stage Dockerfile. Builder runs `cargo-leptos build --release` with
`cargo-chef` caching; runtime stage is digest-pinned `ubuntu:26.04` running a non-root user
with a read-only root FS where feasible, containing the server binary + hashed asset/WASM
bundle. CI gates on `clippy`, `cargo test`, `cargo-deny`, `cargo-audit`, and fuzz smoke; the
release job builds `amd64`+`arm64` via buildx, generates an SBOM (`cargo-cyclonedx`), pushes
to GHCR, and signs the image (cosign/sigstore).

**Rationale**: Delivers Principle V end to end — one reproducible, signed, observable image
with no toolchain/Node/Python in it — and bakes supply-chain assurance into the pipeline
(Principle II), which matters for security-sensitive self-hosting.

**Alternatives rejected**:
- *distroless/scratch runtime*: smaller, but the spec mandates `ubuntu:26.04`, and ubuntu eases
  CA-cert/locale provisioning for the integrations and debugging.
- *Unsigned images / no SBOM*: fails the supply-chain bar; rejected.

## Decision 11 — Toolchain target: GNU on ubuntu, musl as conditional

**Decision**: Build `*-unknown-linux-gnu` for both arches against the ubuntu base. Evaluate
musl static linking only if every dependency (rustls/ring, bollard, kube, sqlite) builds clean
under musl on both arches.

**Rationale**: GNU-on-ubuntu is the low-risk default matching the runtime base; musl's fully
static appeal is real but needs validation with this dependency set before committing.

## Open Items Carried Into Design

None. All items resolved.

## Resolved Items

- **CSP nonce propagation** (resolved 2026-06-19): Leptos supports CSP nonces natively
  since v0.4.5 via the `nonce` feature and `use_nonce` attribute. The Axum integration calls
  `leptos::nonce::provide_nonce()` in the SSR handler. Enable the `nonce` feature on the
  `leptos` crate and call `provide_nonce()` in the Axum handler setup (T011). [Sources:
  github.com/leptos-rs/leptos (Axum integration source), docs.rs/leptos (Csp struct),
  newreleases.io/leptos-rs/leptos/v0.4.5 | Grade: B]

- **Argon2id parameters on arm64** (resolved 2026-06-19): M1 (aarch64) benchmark shows
  32 MiB / 6 iterations / 1 parallelism = ~82ms. Default `m=32 MiB, t=3, p=1` (~40ms on
  arm64) with a startup auto-tune if login exceeds 200ms. Parameters are configurable via
  config. [Sources: github.com/LoupVaillant/Monocypher/issues/274, ciphertools.org | Grade: B]

- **OIDC provisioning policy** (resolved 2026-06-19): Admin-approve is the default. External
  identities created via OIDC login require admin approval before they are mapped to a local
  account. This ensures least surprise for homelab operators and prevents unauthorized IdP
  users from gaining access. [Source: Kelly (operator persona) | Grade: A]
