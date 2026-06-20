<!--
SYNC IMPACT REPORT
==================
Version change: (template/unratified) → 1.0.0
Bump rationale: Initial ratification for a GREENFIELD, standalone, full-Rust reimagining of
  the self-hosted startpage idea popularized by Flame and its fork Dragons-Flame. No
  backwards-compatibility constraint with those projects (HTTP API, SQLite schema, and config
  formats are all free to be
  redesigned). MAJOR baseline because principles are established from scratch for a new
  codebase whose explicit goals are higher security, higher performance, and modernization.
Principles defined: I Full-Rust, End-to-End Type Safety; II Security by Design (NON-NEGOTIABLE);
  III Performance Is a Feature; IV Test-Backed & Verifiable; V Reproducible, Observable,
  Single Artifact.
Templates requiring update: plan-template Constitution Check gate populated by Principles I–V.
Deferred / TODO: none.
-->

# Emberwake Constitution

This is a clean-sheet creation. It is *inspired by* the Dragons-Flame / Flame startpage but
is bound by **none** of its contracts — not the HTTP API, not the SQLite schema, not the
config or file formats. The mandate is a more secure, faster, modern standalone application.

## Core Principles

### I. Full-Rust, End-to-End Type Safety

One language, one type system, from the database row to the rendered DOM node.

- The application is built on Leptos (SSR + hydration + server functions) over Axum/Tokio.
  The client/server boundary MUST be expressed as typed Leptos server functions, not
  hand-maintained JSON endpoints, so a contract change is a compile error rather than a
  runtime surprise.
- The shipped artifact MUST contain no JavaScript-runtime and no Python dependency. Any
  required JS interop is generated via wasm-bindgen from Rust; there is no Node in the image.
- Domain types are defined once and shared across server and client. External input crosses
  the boundary only after parsing into validated typed structures (`validator`/`garde`);
  stringly-typed data is not allowed to propagate past the edge.
- `unsafe` is forbidden in application code. The workspace MUST compile clean under
  `cargo clippy --all-targets --all-features -- -D warnings`.

### II. Security by Design (NON-NEGOTIABLE)

The application is operator infrastructure exposed on a network. Security is the primary
design constraint, threat-modeled up front (`security.md`), not bolted on.

- **Identity**: multi-user accounts; passwords hashed with Argon2id; optional OIDC SSO and
  optional WebAuthn passkeys. There is no single shared password and no plaintext secret at
  rest, in logs, or in any response.
- **Sessions**: server-side, opaque, revocable session tokens (rotating) in HttpOnly,
  Secure, SameSite cookies. State-changing requests MUST be CSRF-protected (origin check +
  token). Auth fails closed.
- **Authorization**: every mutation and every private resource is checked against the
  caller's role/ownership at the server-function boundary. Read of public items is the only
  unauthenticated path.
- **Transport & headers**: rustls only (system OpenSSL MUST NOT be linked). A strict
  Content-Security-Policy, HSTS, `X-Content-Type-Options`, frame-deny, and a minimal
  referrer policy are sent on every response. Optional built-in HTTPS uses rustls + ACME.
- **Abuse resistance**: per-route rate limiting and login throttling. Untrusted parsers
  (HTML/JSON/OPML import) enforce size limits and MUST be fuzzed (`cargo-fuzz`).
- **Least privilege**: Docker and Kubernetes integrations are strictly read-only; the
  container runs as a non-root user with a read-only root filesystem where feasible.
- **Auditability**: security-relevant events (login, logout, auth failure, permission
  denial, content mutation, token issue/revoke) are written to an append-only audit log.
- **Supply chain**: `cargo-deny` (advisories, licenses, bans) and `cargo-audit` are CI
  gates; `Cargo.lock` is committed; an SBOM is generated and release images are signed.

### III. Performance Is a Feature

The reason to leave the Node runtime is speed and footprint; the design defends that.

- Rendering uses Leptos fine-grained reactivity with SSR for first paint and islands-style
  hydration so only interactive regions ship and run WASM. No virtual-DOM diffing.
- The runtime is fully async on Tokio; blocking work (parsing large imports, file I/O on the
  hot path) MUST use `spawn_blocking`. Blocking the executor is a defect.
- SQLite runs in WAL mode with a connection pool and prepared statements; queries are
  compile-time-checked (SQLx) so N+1s and type drift are caught at build time.
- Performance budgets are explicit and tested: see Success Criteria in the spec. A change
  that regresses a budget is not "done."

### IV. Test-Backed & Verifiable (NON-NEGOTIABLE)

Every behavioral change ships with automated tests; the suite is a hard CI gate.

- Server functions and HTTP surface (health, SSE, token API, OIDC callback) MUST have tests
  exercised through the real router.
- Data access MUST be tested against a real SQLite database using isolated per-test
  databases (`#[sqlx::test]`).
- Untrusted-input parsers MUST have fuzz targets in addition to unit tests.
- Critical user journeys (login, create/edit, search) MUST have end-to-end browser tests.
- Tests MUST pass on the published target matrix (`linux/amd64`, `linux/arm64`). Bug fixes
  start with a failing regression test.

### V. Reproducible, Observable, Single Artifact

Delivery is one signed, reproducible image an operator pulls and runs.

- The deliverable is a single multi-stage Docker image whose runtime stage is
  `ubuntu:26.04` (digest-pinned), built by `cargo-leptos`, containing one server binary plus
  the hashed WASM/asset bundle — no toolchain, no Node, no Python.
- Images MUST build for `linux/amd64` and `linux/arm64` from one Dockerfile via buildx, be
  published to GHCR on every `v*` tag, carry an SBOM, and be signed (cosign/sigstore).
- Builds MUST be dependency-pinned and reproducible for a given tag; caching (cargo-chef)
  never introduces nondeterminism.
- Observability is first-class: structured `tracing` logs (JSON, env-configurable level),
  OpenTelemetry (OTLP) trace export, and a Prometheus `/metrics` endpoint. `/healthz` +
  `/readyz` back a container `HEALTHCHECK`.

## Additional Constraints

- **Toolchain**: Rust stable 1.95+, edition 2024, pinned via `rust-toolchain.toml`.
- **Framework**: Leptos 0.8.x (SSR + hydrate) on Axum; build via `cargo-leptos`. Dioxus is
  the documented single-swap alternative if cross-platform native targets become a goal.
- **Datastore**: embedded SQLite (WAL) via SQLx — zero external dependencies is a product
  value. A Postgres backend behind the same repository trait is an accepted future option.
- **Runtime base**: `ubuntu:26.04`. GNU target on that base; musl static linking only if all
  dependencies build cleanly on both arches.
- **License**: Apache-2.0. Emberwake is an independent project that shares no source code or
  assets with Flame or Dragons-Flame; it credits them as inspiration only. Contributions are
  accepted under Apache-2.0 (inbound = outbound); copied third-party code MUST be
  license-compatible and recorded in `NOTICE`.

## Governance

This constitution supersedes ad-hoc convention. Amendments require a recorded version bump
here and a matching update to the plan template's Constitution Check gate. Because there is
no legacy parity contract, the highest-friction amendments are expected to touch Principle II
(Security) — any relaxation of a security default MUST be justified in the plan's Complexity
Tracking table and recorded in `security.md`.

Versioning is semantic: MAJOR for principle removal/redefinition, MINOR for new or materially
expanded principles, PATCH for clarifications. Every plan's Constitution Check MUST cite the
principles it satisfies or the Complexity Tracking row that justifies a deviation.
Unjustified violations block merge; a weakened security default without justification is an
automatic block.

**Version**: 1.0.0 | **Ratified**: 2026-06-19 | **Last Amended**: 2026-06-19
