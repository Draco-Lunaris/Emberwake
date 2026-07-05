# Changelog

All notable changes to Emberwake are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed — Gap Analysis P0/P1 (2026-07-05)

- **Dashboard settings wiring (P0)**: `list_dashboard_query` now reads the six persisted
  `dashboard.*` setting keys via `read_dashboard_settings` instead of returning
  `DashboardSettings::default()`. Section enable/disable and column counts now apply to the
  rendered dashboard (closes BV-004/BV-005/BV-014/BV-015 silent no-op).
- **Uncategorized bookmarks (P0)**: bookmarks with NULL `category_id` now render under a
  synthetic 'Uncategorized' category group on the dashboard (closes BV-003/BV-007).
- **Icon upload path (P0)**: `upload_icon` writes to `{icons_dir}/{id}.{ext}` where
  `icons_dir` is derived from the `db_path` parent and wired via an `IconsDir` Axum
  Extension. Fixes icon upload in containers with read-only rootfs and a data volume at
  `/var/lib/emberwake`.
- **`update_bookmark` category enforcement (P0)**: `update_bookmark` now rejects
  `Uuid::nil()` `category_id`, matching `create_bookmark` (FR-021: REQUIRED).
- **Constant-time CSRF compare (P1)**: `validate_csrf` uses `subtle::ConstantTimeEq`
  instead of `==` to avoid timing side channels.
- **Strict Origin validation (P1)**: `validate_origin` parses the Origin/Referer URL and
  compares the `host:port` authority byte-for-byte against the request's Host header. No
  substring matching (rejects `evil-localhost:5005.evil.com`); empty Host is rejected
  fail-closed. Regression tests added.
- **HMAC-signed import token (P1)**: import preview tokens are now HMAC-SHA256-signed with
  `server_key` (`<base64(json)>.<base64(hmac)>`), preventing clients from crafting arbitrary
  `ParsedData` to bypass the parser's size/depth limits.
- **Configurable WebAuthn RP (P1)**: WebAuthn RP ID/origin are configurable via the
  `[webauthn]` config section (`rp_id`, `rp_origin`). Falls back to `localhost:{port}` for
  local dev.
- **OIDC approval server functions (P1)**: `list_pending_identities` and
  `approve_external_identity` complete the admin-approve provisioning workflow (FR-013).
- **`create_application` is_pinned (P1)**: `create_application_query` honors
  `input.is_pinned` instead of hardcoding `1`.
- **Scoped reorder (P1)**: `reorder_*_query` functions now scope updates by `category_id`
  to prevent cross-category `order_index` collisions.
- **DiscoveryCache poison-safe (P1)**: `DiscoveryCache` RwLock access is poison-safe —
  `.unwrap()` replaced with graceful handling that logs and returns empty/skips on
  `PoisonError`.

### Changed — DOX (2026-07-05)

- `SPEC_STATE.md` updated to reflect Phase 13 reality (T085–T092 complete, T093 pending).
- Root `AGENTS.md` adds Phase 13 status section.
- `crates/AGENTS.md` reconciles the auth/CSRF code-location contract with the actual
  `app/src/server/auth_helper.rs` placement and documents the new wiring.
- `docker-compose.yml` comment updated (non-root, read-only rootfs, data volume).
- `build-multiarch.sh` image owner is configurable via `EMBERWAKE_IMAGE_OWNER`.
- `e2e/Cargo.toml` license corrected to Apache-2.0.

### Added — Audit reports (2026-07-05)

- `docs/audits/2026-07-05-codebase-deep-dive.md` — full codebase deep-dive.
- `docs/audits/2026-07-05-gap-analysis.md` — spec gap analysis with prioritized
  recommendations.

### Added — Phase 12 (Polish)

- Multi-stage Dockerfile with cargo-chef dependency caching, digest-pinned ubuntu:26.04
  runtime, non-root user (UID 10001), read-only rootfs, HEALTHCHECK → /readyz (T076).
- `build-multiarch.sh` for amd64+arm64 multi-arch builds pushing to GHCR (T077).
- CI workflow finalized: clippy `-D warnings` on amd64+arm64 matrix, `cargo leptos test`,
  `cargo deny check`, `cargo audit`, fuzz smoke for import parsers (T078).
- Release workflow finalized: supply-chain gate blocking publish on advisories, buildx
  multi-arch → GHCR, cosign keyless signing, CycloneDX SBOM attached to GitHub Release (T079).
- Performance validation: benchmark script (`benches/seed_benchmark.sh`), WASM bundle
  verified at 89 KB gzip (budget: 350 KB), `PERFORMANCE.md` (T080).
- Security verification: `SECURITY_VERIFICATION.md` documenting SC-005/006, OpenSSL ban,
  parameterized SQL, import limits, read-only integrations, Argon2id parameters (T081).
- E2E test suite with fantoccini: 7 scenarios (setup, login, CRUD, search, edit, delete,
  logout) in `e2e/` (T082).
- Documentation: updated README, `DEPLOYMENT.md` with Docker/Compose/proxy/ACME/secrets/
  SBOM verification guide, updated CHANGELOG (T083).

### Added — Phases 1–11 (US1–US9)

- **US1 — Dashboard**: SSR dashboard with pinned services/bookmarks, client-side fuzzy search
  (no network call), drag-and-drop reorder, optimistic UI, Leptos 0.8 SSR + hydrate.
- **US2 — CRUD**: Full create/update/delete/reorder/pin for categories, services, and
  bookmarks via typed server functions with auth + CSRF enforcement.
- **US3 — Auth**: Multi-user Argon2id auth, server-side revocable sessions (HttpOnly/Secure/
  SameSite=strict), CSRF tokens, per-account/per-IP login throttling, first-run setup
  self-closing, audit logging.
- **US4 — Extended Auth**: Optional OIDC SSO (auth code + PKCE, admin-approve provisioning),
  WebAuthn passkeys (phishing-resistant), scoped API tokens (HMAC-SHA256 hashed, expiring,
  revocable) for `/api/v1/*` REST surface.
- **US5 — Settings & Themes**: Design-token theme builder, custom CSS with CSP nonce,
  SSR theme injection (no flash of default), built-in Light + Dark themes, secret settings
  encrypted at rest.
- **US6 — Status Monitoring**: HTTP/TCP health checks, live status tiles via SSE,
  uptime summary, configurable monitoring per service, retention pruning.
- **US7 — Weather**: WeatherAPI.com integration, scheduled refresh, SSE weather events,
  config-gated (inert when unset), cached readings.
- **US8 — Docker/K8s Discovery**: Read-only Docker container discovery via bollard
  (list/inspect/events), read-only Kubernetes Ingress discovery via kube-rs (list/watch),
  `emberwake.*` label/annotation parsing, SSE discovery events.
- **US9 — Import/Export**: JSON/HTML bookmarks/OPML import with size (10MB) and depth (100)
  limits, bounded parsers on `spawn_blocking`, fuzz targets, transactional all-or-nothing
  import with duplicate handling (skip/overwrite/rename), full data export (excludes
  secrets/hashes).
- Spec-driven design package (GitHub Spec Kit): constitution, specification, implementation
  plan, technology research, data model, threat model, server-function and public-API
  contracts, quickstart, and a phased task backlog under `specs/001-greenfield/`.
- Repository governance and scaffolding: Apache-2.0 license, NOTICE, security policy,
  contributing guide, code of conduct, issue/PR templates, CI and release workflows,
  `cargo-deny` and `dependabot` configuration.
- 107 tests across foundational, server-fn, and integration test suites.

[Unreleased]: https://github.com/Draco-Lunaris/Emberwake/commits/main
