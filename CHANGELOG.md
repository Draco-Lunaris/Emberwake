# Changelog

All notable changes to Emberwake are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
