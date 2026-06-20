# DOX: specs/001-greenfield/contracts

## Purpose

Typed boundary contracts that define the client/server and public API surfaces. The
server-function boundary is the primary RPC; the public REST surface is a small, scoped,
token-protected addition for automation.

## Ownership

- `server-functions.md` — typed `#[server]` function signatures (content CRUD, auth, settings,
  widgets, import/export) with auth requirements and error types
- `public-api.yaml` — OpenAPI 3.0 spec for the non-server-function HTTP surface (health, SSE,
  OIDC callback, scoped-token REST)

## Local Contracts

- Server functions are the primary client/server boundary; the public REST surface is minimal.
- Every mutating server function enforces auth + CSRF + authorization (Principle II).
- Reads exclude private rows for unauthorized callers in SQL, not merely in the UI.
- `AdminSetupInput` = `{ username, password, email? }`; `complete_setup` returns 409 if already
  done; race-safe via `setup_complete` singleton.
- API tokens are scoped, hashed, expiring, revocable; bearer auth only on `/api/v1/*`.
- SSE stream (`/events`) carries public-service status + weather; session-upgraded for private.
- `get_uptime_summary` computes uptime percentage from StatusHistory over a time window.

## Work Guidance

- Signature changes are compile errors on both sides (Principle I) — update both the contract
  doc and the Rust domain types together.
- New server functions require: auth annotation, CSRF on mutations, audit logging where
  security-relevant, and a test through the real router.
- New REST endpoints require: scope definition, rate limiting, audit logging, and an entry in
  `public-api.yaml`.
- Changes propagate to `tasks.md` (new test + implementation tasks) and `data-model.md` if
  new entities are involved.

## Verification

- Server-fn contract tests in `crates/server/tests/server_fn/` exercise the real Axum router.
- SC-005: automated test asserts every mutating server function rejects unauth/CSRF.
- API-token tests: in-scope success, out-of-scope 403, post-revoke 401.

## Child DOX Index

None — leaf directory.
