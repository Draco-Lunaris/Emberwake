# Security Design & Threat Model: Emberwake

**Branch**: `001-greenfield` | **Date**: 2026-06-19

This document operationalizes Constitution Principle II. It is a gate input to `plan.md`: any
weakening of a control here requires a Complexity Tracking justification.

## Assets

- Operator and user credentials (password hashes, passkeys, OIDC links, API token secrets).
- Session state (the keys to every authenticated action).
- Private content (services/bookmarks/categories marked private).
- Integration access (Docker socket, Kubernetes API) — high value if abused.
- Secret settings (weather API key, OIDC client secret).
- The audit log (integrity of the security record).

## Trust boundaries

1. Anonymous network client → server (public reads only).
2. Authenticated browser session → server functions (role/ownership enforced).
3. API-token automation → public REST surface (scope enforced).
4. Server → external upstreams (WeatherAPI, OIDC IdP) over rustls.
5. Server → host/cluster integrations (Docker socket, K8s API), read-only.

## Threats and controls (STRIDE-oriented)

**Spoofing**

- Credential stuffing / brute force → Argon2id (configurable cost), per-account and per-IP
  login throttling (`tower_governor`), generic failure messages, audit of `login_fail`.
- Session theft → HttpOnly/Secure/SameSite=strict cookies; server-side revocable sessions;
  rotation on privilege change; idle + absolute timeouts; HSTS so cookies never traverse
  plaintext.
- Phishing → optional WebAuthn passkeys (origin-bound, phishing-resistant).

**Tampering**

- CSRF on state-changing requests → SameSite=strict + origin/referer check + per-session
  token required by every mutating server function.
- SQL injection → SQLx parameterized, compile-time-checked queries only; no string-built SQL.
- Supply-chain tampering → `Cargo.lock` committed, `cargo-deny` (bans/advisories/licenses)
  and `cargo-audit` as CI gates, SBOM generated, release images signed (cosign).

**Repudiation**

- Append-only `AuditEvent` log for auth, session, permission, content-mutation, and token
  events; no update/delete path exposed; events carry actor, ip, user-agent, result.

**Information disclosure**

- Secrets never logged or serialized into responses; secret-bearing settings encrypted at
  rest; password/token only ever stored hashed; token secret shown once.
- Private content excluded in SQL for anonymous/unauthorized callers (not merely hidden in
  the UI).
- Strict CSP (nonce/hash, no `unsafe-inline`), `X-Content-Type-Options: nosniff`, frame-deny,
  minimal referrer policy; rustls-only transport.
- Error responses are generic to clients; details go to logs/traces, not the wire.

**Denial of service**

- Rate limits on login/token/import; request body size limits; import parsers bounded by
  size and derivation depth and fuzzed (`cargo-fuzz`) — no panic/OOM on hostile input.
- All I/O async; heavy parsing on `spawn_blocking` so one request can't stall the executor;
  monitor/weather checks have timeouts and never block the request path.

**Elevation of privilege**

- Authorization enforced at the server-function boundary by role and ownership; auth fails
  closed; first-run setup route self-closes after the admin exists (guarded against the
  two-request race).
- API tokens are least-privilege scoped, expiring, and revocable.
- Docker/K8s integrations wire only read calls (list/inspect/watch); mutation is impossible by
  omission. Docker socket mounted only when the operator opts in.

## Container & deployment hardening

- Non-root user; read-only root filesystem where feasible; writable only on the data volume.
- No toolchain, Node, or Python in the runtime image (smaller attack surface).
- `ubuntu:26.04` runtime digest-pinned; image signed; SBOM shipped.
- Optional built-in HTTPS via rustls + ACME; otherwise operators terminate TLS at a proxy and
  the app still sets HSTS/secure-cookie expectations.
- Secrets provided via env or file (`*_FILE`) and never baked into the image.

## Explicit non-goals (v1)

- Not a multi-tenant SaaS; one operator org per instance.
- No built-in WAF; assumes the operator's network controls front it if internet-exposed.
- No secret-manager integration beyond env/file in v1 (future option).

## Verification hooks (mapped to tests)

- SC-005: automated test asserts every mutating server function rejects unauthenticated and
  CSRF-forged requests.
- SC-006: CI fails on any `cargo-deny`/`cargo-audit` finding; release asserts signature + SBOM.
- SC-007: fuzz run on import parsers must complete without panic/OOM and reject the malformed
  corpus with no partial write.
