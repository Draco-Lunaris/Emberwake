---
description: "Task list for the greenfield full-Rust Emberwake"
---

# Tasks: Emberwake

**Input**: Design documents from `/specs/001-greenfield/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, security.md,
contracts/server-functions.md, contracts/public-api.yaml

**Tests**: Included and a hard gate — Constitution Principles II (Security) and IV (Test-
Backed) are NON-NEGOTIABLE.

**Organization**: Grouped by user story; each slice is independently implementable, testable,
deployable. MVP = US1–US3.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different files, no dependency)
- **[Story]**: US1–US9; setup/foundational/polish unlabeled
- Paths follow the `cargo-leptos` workspace in plan.md (`crates/app`, `crates/server`,
  `crates/frontend`, `migrations/`)

---

## Phase 1: Setup

- [ ] T001 Create the `cargo-leptos` workspace: `crates/app` (ssr+hydrate), `crates/frontend`
      (hydrate bin), `crates/server` (ssr bin); `rust-toolchain.toml` (1.95, edition 2024);
      commit `Cargo.lock`.
- [ ] T002 [P] Add `Leptos.toml`/cargo-leptos config (output dir, hashed bundle, site addr
      `:5005`) and confirm `cargo leptos watch` serves a hello route.
- [ ] T003 [P] Wire base deps: leptos/leptos_axum/leptos_router, axum/tokio/tower, sqlx
      (sqlite), serde, uuid(v7), tracing, thiserror — pinned.
- [ ] T004 [P] Add tooling configs: `deny.toml` (cargo-deny), clippy `-D warnings`, rustfmt,
      `fuzz/` scaffold, `e2e/` scaffold.
- [ ] T005 [P] Scaffold `.docker/Dockerfile`, `docker-compose.yml`, `build-multiarch.sh`, and
      `.github/workflows/{ci,release}.yml` placeholders (filled in Polish).

---

## Phase 2: Foundational (Blocking Prerequisites)

**⚠️ CRITICAL**: No user-story work begins until this phase is complete.

- [ ] T006 Author initial SQL migrations in `migrations/` for all entities in `data-model.md`
      (users, sessions, external_identity, passkey_credential, api_token, category, service,
      bookmark, setting, theme, status_reading, status_history, weather_reading, audit_event).
      Include a `UNIQUE` constraint on the `setup_complete` singleton setting key.
- [ ] T007 [P] DB layer in `crates/server/src/db/`: pool init (WAL, `foreign_keys=ON`), run
      migrations on startup (idempotent), repository trait + SQLite impl skeleton.
- [ ] T008 [P] Shared domain types + validation in `crates/app/src/domain/` (DTOs, `Visibility`,
      `Role`, input/patch types with `validator`/`garde`).
- [ ] T009 [P] Typed `AppError` + `ServerFnError` mapping in `crates/app/src/error.rs`.
- [ ] T010 [P] Config loader in `crates/server/src/config.rs`: figment TOML+env, `*_FILE`
      secret resolution (file wins; unreadable fails loud), validated at startup.
- [ ] T011 Security middleware in `crates/server/src/security/`: strict CSP with per-response
      nonce via Leptos `nonce` feature (`provide_nonce()` in the Axum handler, `use_nonce` on
      inline script/style tags — confirmed supported in Leptos 0.8.x), HSTS, nosniff, frame-deny,
      referrer policy; applied to all responses.
- [ ] T012 [P] Rate-limiting layer (`tower_governor`) with per-route policies (login/token/
      import) in `security/rate_limit.rs`.
- [ ] T013 [P] Telemetry in `crates/server/src/telemetry.rs`: JSON `tracing` subscriber (env
      level), OTLP export, Prometheus `/metrics`, `/healthz` + `/readyz`.
- [ ] T014 [P] Append-only audit writer in `crates/server/src/audit.rs`.
- [ ] T014b [P] Scheduled WAL checkpoint + optional automated SQLite `.backup` to the data
      volume, with configurable retention limits (max backup count and max total size) to
      preserve disk space. Defaults: checkpoint every 15 min, daily backup, retain 7 backups
      or 500 MB total. Configurable via `db.backup.*` settings.
- [ ] T015 Assemble the Axum router + Leptos SSR handler + app state (db, config, audit,
      telemetry, security layers) in `crates/server/src/main.rs`; bind `:5005`.
- [ ] T016 [P] Base Leptos shell: `crates/app/src/lib.rs` (router, document head, theme slot)
      and `crates/frontend/src/lib.rs` (hydrate entry).
- [ ] T017 Foundational tests: `#[sqlx::test]` migration/up + repository round-trip; a
      security-headers test asserting CSP/HSTS on a sample response.

**Checkpoint**: Server boots, migrates, serves an SSR shell with headers/telemetry/health.

---

## Phase 3: User Story 1 — Fast, searchable dashboard (P1) 🎯 MVP

**Goal**: SSR dashboard of pinned services/bookmark-groups + fuzzy search + provider routing.

**Independent Test**: Seed public content; assert SSR HTML contains pinned items pre-hydration;
fuzzy search filters client-side; prefixed query routes to provider URL.

### Tests for US1 ⚠️ (write first)

- [x] T018 [P] [US1] Server-fn test: `list_dashboard` returns only public pinned items for an
      anonymous caller, in `crates/server/tests/server_fn/dashboard.rs`.
- [x] T019 [P] [US1] SSR test: rendered HTML for `/` contains seeded pinned items before WASM,
      in `tests/integration/ssr_dashboard.rs`.
- [x] T020 [P] [US1] Component test (wasm-bindgen-test): fuzzy matcher ranks a misspelled query.

### Implementation for US1

- [x] T021 [P] [US1] Read repository methods (dashboard/list, visibility-filtered in SQL) in
      `crates/server/src/db/`.
- [x] T022 [US1] `list_dashboard` / `list_categories` / `list_services` / `list_bookmarks`
      server functions in `crates/app/src/server/content_read.rs`.
- [x] T023 [P] [US1] Dashboard + tile + category components in
      `crates/app/src/components/dashboard/` (SSR-rendered, minimal hydration).
- [x] T024 [P] [US1] Search island (fuzzy via `nucleo`/`fuzzy-matcher` in WASM) + provider
      prefix routing in `components/search/`.
- [x] T025 [US1] Settings-backed search-provider config read for prefix routing.

**Checkpoint**: Instant server-rendered, searchable dashboard — demoable MVP shell.

---

## Phase 4: User Story 2 — Built-in editors / content CRUD (P1)

**Goal**: Full CRUD + drag-reorder + pin via server functions with optimistic UI.

**Independent Test**: Create→edit→reorder→pin→delete across all three entities; optimistic UI;
persistence across reload; validation rejects bad input.

### Tests for US2 ⚠️

- [x] T026 [P] [US2] Server-fn tests for service/bookmark/category create/update/delete/reorder
      incl. validation failures, in `tests/server_fn/content_write.rs`.
- [x] T027 [P] [US2] Integration test: CRUD lifecycle persists across a simulated restart.

### Implementation for US2

- [x] T028 [P] [US2] Write repository methods (create/update/delete/reorder/pin; delete policy
      for categories tested) in `crates/server/src/db/`.
- [x] T029 [US2] Mutating server functions in `crates/app/src/server/content_write.rs`
      (auth+CSRF+authz enforced; audited).
- [x] T030 [P] [US2] Editor components (forms + drag-and-drop reorder, optimistic updates) in
      `components/editors/`.
- [x] T031 [US2] Icon upload server function + validated multipart handler writing to the data
      volume.

**Checkpoint**: Dashboard is fully editable from the UI; US1 + US2 both standalone.

---

## Phase 5: User Story 3 — Accounts, sessions, roles (P1)

**Goal**: First-run admin setup; multi-user Argon2id login; server-side revocable sessions;
role/visibility enforcement; sign-out + revoke-all.

**Independent Test**: setup→admin; create user; login issues secure session; wrong password
throttled; private items hidden from unauthorized; sign-out/revoke invalidate server-side.

### Tests for US3 ⚠️

- [x] T032 [P] [US3] Server-fn tests: setup single-shot + race safety; login success/throttle;
      logout/revoke invalidate server-side, in `tests/server_fn/auth.rs`.
- [x] T033 [P] [US3] Authz test: private rows excluded for anon/unauthorized in read fns.
- [x] T034 [P] [US3] CSRF test: a forged cross-origin mutation is rejected.

### Implementation for US3

- [x] T035 [P] [US3] Argon2id hash/verify (configurable cost) in `crates/server/src/auth/password.rs`.
- [x] T036 [P] [US3] Session layer (`tower-sessions` + SQLx store): opaque rotating tokens,
      HttpOnly/Secure/SameSite cookie, idle+absolute expiry, in `auth/session.rs`.
- [x] T037 [P] [US3] CSRF protection (origin check + per-session token) in `auth/csrf.rs`,
      applied to all mutating server functions.
- [x] T038 [US3] First-run setup (`setup_status`/`complete_setup`, race-safe close) +
      login/logout/current_user server functions in `crates/app/src/server/auth.rs`.
- [x] T039 [US3] Session management (`list_sessions`/`revoke_session`/`revoke_all_other`) and
      admin user management server functions; audit all auth events.
- [x] T040 [US3] Enforce role + visibility in every read/write fn (server-boundary, in SQL).
- [x] T041 [P] [US3] Login/setup/account UI routes + session list in `components/auth/`.

**Checkpoint**: Multi-user, secure, revocable auth gates editing and private content — **MVP
complete**. Stop and validate (SC-001..003, SC-005).

---

## Phase 6: User Story 4 — SSO, passkeys, API tokens (P2)

**Goal**: Optional OIDC (auth code + PKCE), optional WebAuthn passkeys, scoped revocable API
tokens for the public REST surface.

**Independent Test**: stub-IdP OIDC login maps to a local user; passkey register+auth via
virtual authenticator; scoped token works in-scope, fails out-of-scope and after revoke.

### Tests for US4 ⚠️

- [ ] T042 [P] [US4] OIDC callback test against a stub IdP (code+PKCE → session), in
      `tests/integration/oidc.rs`.
- [ ] T043 [P] [US4] WebAuthn register/login test with a virtual authenticator.
- [ ] T044 [P] [US4] API-token test: in-scope success, out-of-scope 403, post-revoke 401,
      against `/api/v1/*`.

### Implementation for US4

- [ ] T045 [P] [US4] OIDC client (`openidconnect`, discovery, PKCE) + `/auth/oidc/{login,callback}`
      in `crates/server/src/auth/oidc.rs`; provisioning policy (default admin-approve).
- [ ] T046 [P] [US4] WebAuthn (`webauthn-rs`) register/login server functions in `auth/webauthn.rs`.
- [ ] T047 [P] [US4] Scoped API-token issue/verify (hashed, scopes, expiry, revoke) +
      bearer middleware in `auth/api_token.rs`.
- [ ] T048 [US4] Public REST surface `crates/server/src/public_api.rs` (`/api/v1/*` per
      public-api.yaml), scope-checked, rate-limited, audited.
- [ ] T049 [P] [US4] Account UI: link IdP, manage passkeys, manage API tokens.

**Checkpoint**: Modern auth options available; base product unaffected when disabled.

---

## Phase 7: User Story 5 — Theming & settings (P2)

**Goal**: Built-in themes, design-token builder, custom CSS, light/dark via system pref,
search providers + integration toggles — applied server-side without flash.

### Tests for US5 ⚠️

- [ ] T050 [P] [US5] Server-fn tests for settings + theme CRUD incl. secret redaction for
      non-admins, in `tests/server_fn/settings.rs`.
- [ ] T051 [P] [US5] SSR test: active theme tokens + custom CSS present in first response (no
      default-theme flash).

### Implementation for US5

- [ ] T052 [P] [US5] Typed settings registry + secret encryption-at-rest in
      `crates/server/src/db/settings.rs`.
- [ ] T053 [US5] Settings/theme server functions (`get/update_settings`, theme CRUD,
      `set_active_theme`); admin-gated; audited.
- [ ] T054 [P] [US5] Theme application during SSR (token injection, CSP-safe custom CSS) +
      `prefers-color-scheme` fallback.
- [ ] T055 [P] [US5] Settings + theme-builder UI in `components/settings/`.

**Checkpoint**: Theming/settings round-trip; SSR-applied.

---

## Phase 8: User Story 6 — Live service status monitoring (P3)

**Goal**: Scheduled HTTP/TCP health checks; live tile updates pushed via SSE.

### Tests for US6 ⚠️

- [ ] T056 [P] [US6] Scheduler test: check records state/latency; transition emits an event,
      history row written, in `tests/integration/monitor.rs`.
- [ ] T057 [P] [US6] SSE test: a connected client receives a status event on up→down flip.
- [ ] T056b [P] [US6] Uptime summary test: `get_uptime_summary` computes correct percentage
      from StatusHistory over a given window; retention pruning removes old rows.

### Implementation for US6

- [ ] T058 [P] [US6] Health-check engine (HTTP/TCP, timeouts, never blocks request path) in
      `crates/server/src/monitor/`.
- [ ] T059 [P] [US6] SSE hub + `/events` stream (public vs. session-upgraded) in
      `crates/server/src/sse/`.
- [ ] T060 [US6] Scheduler task wiring + `get_service_statuses` read fn + live tile component.
      Writes StatusHistory on each check; prunes by max-rows (default 1000) and max-age-days
      (default 30). Includes `get_uptime_summary` read fn.

**Checkpoint**: Live status board; disabled services make no outbound calls.

---

## Phase 9: User Story 7 — Weather widget (P3)

**Goal**: Scheduled server-side weather fetch → cache → SSR render → live SSE push.

### Tests for US7 ⚠️

- [ ] T061 [P] [US7] Stubbed-upstream test: scheduled fetch caches a reading; widget reads
      cache; SSE push on refresh, in `tests/integration/weather.rs`.

### Implementation for US7

- [ ] T062 [P] [US7] WeatherAPI client (`reqwest`+rustls) + cache write in
      `crates/server/src/integrations/weather.rs`.
- [ ] T063 [US7] Refresh task (config-gated, inert when unset) + `get_weather` read fn +
      weather widget component reusing the SSE hub.

**Checkpoint**: Weather populates + pushes; inert and harmless when unconfigured.

---

## Phase 10: User Story 8 — Docker & Kubernetes discovery (P3)

**Goal**: Opt-in read-only discovery from Docker labels (+ live events) and K8s Ingress
annotations.

### Tests for US8 ⚠️

- [ ] T064 [P] [US8] Parser unit test: labels/annotations → services incl. multi-value syntax.
- [ ] T065 [P] [US8] Mocked-Docker test: labeled containers discovered; container-start event
      surfaces live; disabled = no calls, in `tests/integration/discovery.rs`.

### Implementation for US8

- [ ] T066 [P] [US8] Docker integration (`bollard`: list/inspect + events, read-only) in
      `integrations/docker.rs`.
- [ ] T067 [P] [US8] Kubernetes integration (`kube`: ingress list/watch, read-only) in
      `integrations/kubernetes.rs`.
- [ ] T068 [US8] `discover_docker`/`discover_kubernetes` fns gated by settings; live deltas via
      the SSE hub; assert no mutating API calls (Principle II).

**Checkpoint**: Discovery works when enabled, silent and call-free when disabled.

---

## Phase 11: User Story 9 — Import / Export & portability (P3)

**Goal**: JSON full/selective export; JSON/HTML/OPML import with validation, size limits,
duplicate detection, preview; parsers fuzzed.

### Tests for US9 ⚠️

- [ ] T069 [P] [US9] Export→import round-trip equivalence, in `tests/integration/export_import.rs`.
- [ ] T070 [P] [US9] HTML + OPML import → categories/bookmarks + duplicate handling.
- [ ] T071 [P] [US9] Reject oversized/malformed input before any write (no partial import).
- [ ] T072 [P] [US9] `cargo-fuzz` targets for the html/json/opml parsers in `fuzz/`.

### Implementation for US9

- [ ] T073 [P] [US9] JSON exporter (full+selective) in `crates/server/src/importer/export.rs`.
- [ ] T074 [P] [US9] Bounded parsers: JSON, HTML (`scraper`), OPML — size/derivation limits,
      run on `spawn_blocking` — in `importer/{json,html,opml}.rs`.
- [ ] T075 [US9] `import_preview` (no write) + transactional `import_apply` (duplicate options;
      audited) server functions.

**Checkpoint**: Backup/migrate end-to-end; parsers hardened and fuzzed.

---

## Phase 12: Polish & Cross-Cutting

- [ ] T076 Finalize multi-stage `.docker/Dockerfile`: cargo-chef builder runs
      `cargo leptos build --release`; digest-pinned `ubuntu:26.04` runtime; non-root user;
      read-only rootfs + tmpfs; copies binary + hashed bundle; `HEALTHCHECK` → `/readyz`.
- [ ] T077 [P] `build-multiarch.sh` + buildx for `amd64,arm64`; verify identical run on both.
- [ ] T078 CI workflow: clippy + `cargo leptos test` + `cargo deny` + `cargo audit` + fuzz
      smoke on the amd64/arm64 matrix.
- [ ] T079 Release workflow: buildx multi-arch → GHCR; generate SBOM (`cargo-cyclonedx`); sign
      image (cosign); block publish on any advisory/unsigned build.
- [ ] T080 [P] Performance validation against budgets: SSR TTFB < 50 ms, interactive < 1 s,
      CRUD p95 < 25 ms, bundle < 350 KB, idle RSS ≤ 48 MB, cold start < 1.5 s (SC-001..004,007).
- [ ] T081 [P] Security verification pass mapped to `security.md`: SC-005 (every mutation
      rejects unauth/CSRF), no OpenSSL linkage, secrets never logged/serialized, parameterized
      SQL only, import limits, read-only integrations, per-arch Argon2id parameters tuned.
- [ ] T082 [P] E2E suite (login, create/edit, search) green on both arches.
- [ ] T083 [P] Docs: `README`, deployment guide (proxy + built-in ACME), security notes, SBOM/
      signature verification instructions.
- [ ] T084 Run the `quickstart.md` acceptance walkthrough end to end.

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (1)** → **Foundational (2)** [BLOCKS all stories] → **Stories (3–11)** → **Polish (12)**.
- MVP critical path: **US1 → US2 → US3** (US2 builds on US1 reads; US3 secures US1/US2).
- **US4** depends on US3 (extends auth). **US5** depends only on Foundational (settings).
- **US6, US7** share the SSE hub (build it once in US6; US7 reuses). **US8** reuses the SSE
  hub for live deltas. **US9** depends only on Foundational + content repos.

### Within each story

- Tests first and failing before implementation.
- Migrations/repository → server functions → components → integration.
- Every mutation wires auth + CSRF + authz before it is considered done (Principle II).

### Parallel opportunities

- Foundational `[P]` tasks (config, security headers, rate limit, telemetry, audit, domain,
  error) parallelize after migrations + db skeleton (T006/T007) land.
- After Foundational: one track runs the P1 chain (US1→US2→US3); separate tracks can take US5,
  then US6/US7/US8/US9 once the SSE hub exists.
- All `[P]` test tasks within a story run in parallel.

---

## Implementation Strategy

### MVP first

Setup → Foundational → US1 → US2 → US3 → **STOP and validate** (fast SSR dashboard, editable,
multi-user secure auth; check SC-001/002/003/005) → demo/deploy.

### Incremental delivery

MVP → US4 (modern auth) → US5 (theming) → US6 (live status) → US7 (weather) → US8 (discovery)
→ US9 (portability) → Polish (sign + ship the multi-arch image). Each slice independently
testable and deployable.

### Parallel team strategy

After Foundational: Dev A drives US1→US2→US3; Dev B takes US5 then US4; Dev C builds the SSE
hub via US6 then US7/US8; Dev D takes US9 and the fuzzing. Polish converges all.

---

## Notes

- [P] = different files, no dependency. [Story] maps each task for traceability.
- Security tasks are not optional polish — they ship inside each story (Principle II).
- Tests are a hard gate (Principle IV); fuzz the untrusted parsers (US9).
- No legacy parity: do not spend effort matching old shapes; a one-way legacy importer, if
  ever wanted, is a future US9-style addition, not a contract.
- Commit per task or logical group; stop at any checkpoint to validate a slice.
