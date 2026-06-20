# Implementation Plan: Emberwake

**Branch**: `001-greenfield` | **Date**: 2026-06-19 | **Spec**: ./spec.md

**Input**: Feature specification from `/specs/001-greenfield/spec.md`

## Summary

A clean-sheet, full-Rust self-hosted startpage: a Leptos (SSR + hydrate) application on
Axum/Tokio with SQLite (WAL) via SQLx, multi-user security-first identity (Argon2id,
server-side revocable sessions, optional OIDC/passkeys/scoped tokens), live status and weather
widgets over SSE, optional read-only Docker/Kubernetes discovery, modern theming, and fuzzed
import/export — delivered as a single signed multi-arch `ubuntu:26.04` image with first-class
observability. No legacy API/schema/config parity is attempted. Capabilities ship as
independent slices; the MVP is a fast, editable, multi-user dashboard (US1–US3).

## Technical Context

**Language/Version**: Rust stable 1.95+, edition 2024, pinned via `rust-toolchain.toml`.

**Primary Dependencies**: Leptos 0.8.x + `leptos_axum` + `leptos_router` (SSR/hydrate, server
functions), built with `cargo-leptos`; Axum/Tokio/Tower; SQLx (SQLite, compile-time queries,
migrations); `tower-sessions` (SQLx store) + `argon2`; `openidconnect`, `webauthn-rs`;
`tower_governor` (rate limit); security-headers layer (CSP nonces, HSTS); `reqwest` (rustls)
for weather/OIDC; `bollard` (Docker + events), `kube` + `k8s-openapi`; `scraper` (HTML import),
`serde`/`serde_json` (JSON/OPML); `tracing` + `tracing-opentelemetry` + `opentelemetry-otlp` +
a Prometheus exporter; `uuid` (v7); `validator`/`garde`; optional `rustls-acme` (built-in
HTTPS). Build/supply chain: `cargo-chef`, `cargo-deny`, `cargo-audit`, `cargo-cyclonedx`,
`cargo-fuzz`, cosign.

**Storage**: Embedded SQLite (WAL) at the data volume via SQLx; data access behind a
repository trait (future Postgres swap). UUIDv7 keys; forward SQL migrations.

**Testing**: `cargo test` (server functions + HTTP surface through the real router);
`#[sqlx::test]` isolated-DB data tests; `cargo-fuzz` on import parsers; end-to-end browser
tests (fantoccini/WebDriver) for login/create/search; CI on `amd64` + `arm64`.

**Target Platform**: Linux containers, `linux/amd64` + `linux/arm64`, runtime `ubuntu:26.04`.

**Project Type**: Full-stack Rust web application (Leptos SSR/hydrate + Axum server).

**Performance Goals**: SSR TTFB < 50 ms and interactive < 1 s at 200 services / 500 bookmarks;
CRUD server-fn p95 < 25 ms; hydration bundle < 350 KB compressed (SC-001/002/004).

**Constraints**: Idle RSS ≤ 48 MB; cold start to `readyz` < 1.5 s; rustls only (no system
OpenSSL); strict CSP, no `unsafe-inline`; read-only Docker/K8s; non-root, read-only rootfs
where feasible; signed image + SBOM (SC-003/006/008).

**Scale/Scope**: Single-instance self-hosted; one operator org; catalogs to low thousands of
items; small user counts.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan satisfies it |
|-----------|-----------------------------|
| I. Full-Rust, End-to-End Type Safety | Leptos SSR/hydrate with typed `#[server]` functions as the only client/server boundary; no JS runtime, no Python in the image; shared validated domain types. |
| II. Security by Design | Argon2id + server-side revocable sessions + CSRF + strict CSP + rate limiting + read-only integrations + audit log + supply-chain gates; full threat model in `security.md`. |
| III. Performance Is a Feature | Fine-grained SSR + island hydration (no VDOM); SQLite WAL + compile-time-checked SQLx; fully async; explicit, tested budgets (SC-001/002/003/004). |
| IV. Test-Backed & Verifiable | Server-fn/HTTP tests through the real router; `#[sqlx::test]` data tests; fuzzed parsers; E2E journeys; amd64/arm64 CI matrix. |
| V. Reproducible, Observable, Single Artifact | One `cargo-leptos`-built, signed, SBOM-bearing multi-arch `ubuntu:26.04` image; tracing/OTel/Prometheus; `/healthz`+`/readyz`+`HEALTHCHECK`. |

**Result**: PASS. No deviations require Complexity Tracking at plan time. Re-check after Phase
1 once CSP-nonce propagation through Leptos SSR and per-arch Argon2id parameters are confirmed
(both tracked as Open Items in `research.md`); neither is a principle deviation.

## Project Structure

### Documentation (this feature)

```text
specs/001-greenfield/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 decisions
├── data-model.md        # Phase 1 schema
├── security.md          # Threat model (Principle II gate input)
├── quickstart.md        # Phase 1 dev/build/run workflow
├── contracts/
│   ├── server-functions.md  # typed server-fn boundary (primary RPC)
│   └── public-api.yaml      # small public REST/SSE/OIDC surface
└── tasks.md             # Phase 2 output (/tasks)
```

### Source Code (repository root)

```text
Cargo.toml                 # workspace
rust-toolchain.toml        # 1.95+, edition 2024
Cargo.lock                 # committed (reproducible)
deny.toml                  # cargo-deny config (supply chain)
Cargo.toml [leptos] / Leptos.toml   # cargo-leptos config (output, bundle)
migrations/                # sqlx forward SQL migrations

crates/
├── app/                   # Leptos UI + server functions + shared domain (ssr+hydrate)
│   └── src/
│       ├── lib.rs
│       ├── domain/        # entities, DTOs, validation (shared types)
│       ├── components/    # islands: dashboard, editors, search, settings, widgets
│       ├── routes/        # leptos_router routes (dashboard, admin, setup, login)
│       ├── server/        # #[server] fns: content CRUD, auth, settings, import/export
│       └── error.rs       # typed app error
├── server/                # Axum binary (ssr): startup, state, wiring, middleware
│   └── src/
│       ├── main.rs        # config load, migrate, build router, serve (+ optional ACME)
│       ├── config.rs      # figment TOML+env, *_FILE secrets, validation
│       ├── db/            # pool (WAL, fk on), repository trait + sqlite impl
│       ├── auth/          # argon2, sessions, csrf, oidc, webauthn, api-token middleware
│       ├── security/      # CSP/nonce + headers layer, rate limiting
│       ├── integrations/  # docker (bollard+events), kubernetes (kube), weather client
│       ├── monitor/       # status-check scheduler
│       ├── sse/           # server-push hub (status, weather)
│       ├── importer/      # json, html, opml parsers (bounded; fuzzed)
│       ├── audit.rs       # append-only audit writer
│       ├── telemetry.rs   # tracing + OTLP + metrics; /healthz /readyz /metrics
│       └── public_api.rs  # scoped-token REST surface
│   └── tests/
│       ├── server_fn/     # contract tests through the real router
│       ├── integration/   # #[sqlx::test] data + journey tests
│       └── fixtures/
└── frontend/              # thin wasm hydrate entry (hydrate feature)
    └── src/lib.rs

fuzz/                      # cargo-fuzz targets (importer parsers)
e2e/                       # WebDriver end-to-end tests
.docker/Dockerfile         # multi-stage; cargo-leptos build; ubuntu:26.04 runtime; non-root
docker-compose.yml
build-multiarch.sh
.github/workflows/         # ci (clippy, test, deny, audit, fuzz-smoke); release (buildx, GHCR, SBOM, cosign)
```

**Structure Decision**: The canonical `cargo-leptos` workspace shape — a shared `app` crate
(components + server functions + domain types, compiled for both `ssr` and `hydrate`), a thin
`frontend` hydrate entry, and a `server` binary that owns all non-UI concerns (db, auth,
security middleware, integrations, monitoring, SSE, telemetry, public API). Keeping security
middleware, integrations, and the scheduler in `server/` (not `app/`) ensures none of that
code can be accidentally pulled into the WASM bundle, protecting both the bundle-size budget
(SC-004) and the secret-handling boundary. `migrations/` and `fuzz/`/`e2e/` sit at root per
their tool conventions.

## Complexity Tracking

> No Constitution Check violations at plan time — table intentionally empty. Security defaults
> are met, not relaxed; any future relaxation lands here with justification.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
| — | — | — |
