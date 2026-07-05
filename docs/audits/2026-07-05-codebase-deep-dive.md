# Emberwake Codebase Deep Dive Report

**Repository**: `/home/openchamber/workspaces/Emberwake`  
**Date**: Sun Jul 05 2026  
**State per SPEC_STATE.md**: Phases 1–12 complete (T001–T084); Phase 13 (Three-Section Dashboard Redesign) not started.  
**Toolchain**: Rust 1.95, edition 2024, target `wasm32-unknown-unknown` configured via `.cargo/config.toml` (`--cfg getrandom_backend="wasm_js"`).  
**DOX chain read**: root `AGENTS.md`, `crates/AGENTS.md` (236 lines, fully read), `.github/AGENTS.md`.

---

## 1. Architecture Overview

### 1.1 Crate Graph

Three-crate cargo workspace (`Cargo.toml:1-2`) + standalone `fuzz/` (excluded) + standalone `e2e/` (not a workspace member, per `crates/AGENTS.md:228`).

```
                    ┌──────────────┐
                    │   frontend   │ cdylib (WASM hydrate entry)
                    │  (cdylib)    │ depends: app[hydrate]
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │     app      │ cdylib + rlib
                    │ domain, UI,  │ features: ssr | hydrate
                    │ server fns   │ (no sqlx in WASM)
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │   server     │ lib + bin "emberwake"
                    │ Axum, db,    │ features: ssr → app/ssr
                    │ auth, integ  │
                    └──────────────┘

  fuzz/  → depends on app[ssr]   (cargo-fuzz, nightly, excluded from workspace)
  e2e/   → fantoccini (standalone, kept out to avoid native-tls in deny.toml)
```

Cross-crate dependency direction: `frontend → app`, `server → app`. `app` is the shared core (domain types, server functions, UI). No `app → server` edge — this is enforced by feature gates (`sqlx` is optional behind `ssr` in app, per `crates/app/Cargo.toml:19`).

### 1.2 Shared-SQL Pattern (Circular-Dependency Avoidance)

A notable architectural pattern: SQL read/write functions live in `app/src/server/*_queries.rs` (ssr-only), and `server/src/db/repository.rs` delegates to them (`crates/server/src/db/repository.rs:186-342`). This avoids a `server → app → server` cycle by keeping the SQL in the lower crate. Per `crates/AGENTS.md:56-59`.

### 1.3 Request Lifecycle (SSR path)

1. `main.rs:171` binds `TcpListener`, `axum::serve` runs the router built at `main.rs:134-169`.
2. Router merges: `telemetry::health_routes` (`/healthz`, `/readyz`, `/metrics`), `oidc::oidc_routes` (`/auth/oidc/*`), `public_api::public_api_routes` (`/api/v1/*`), `sse::handler::sse_routes` (`/events`), then `leptos_routes(&state, routes, shell)` for the SSR app, and `file_and_error_handler` fallback.
3. Security header layers applied in order: HSTS → nosniff → frame-deny → referrer-policy (`main.rs:144-149`).
4. Axum Extensions layered on: `SqlitePool`, `ServerKey`, `WebAuthnRpInfo`, `ChallengeStore`, `DiscoveryCache`, `Argon2Params` (`main.rs:150-161`). These are extracted by server functions via `leptos_axum::extract::<Extension<T>>()`.
5. Rate limiting applied conditionally on `config.security.rate_limit_enabled` (`main.rs:123-167`). Public API gets `token_governor`, OIDC gets `login_governor`, default router gets `default_governor`.
6. `shell()` (`main.rs:17-37`) calls `leptos::nonce::provide_nonce()` (per-response CSP nonce), renders `<!DOCTYPE html>` with `<HashedStylesheet>`, `<AutoReload>`, `<HydrationScripts>` in `<head>`, and `<App />` in `<body>`.
7. `App` (`crates/app/src/lib.rs:28-96`) calls `provide_meta_context()`, fetches active theme via `Resource::new`, emits `<Meta http_equiv="Content-Security-Policy" ...>` with per-response nonce, emits `<Style>` with theme CSS custom properties (no-flash), then `<Router>` with 11 routes.
8. Server functions are dispatched via `leptos_axum`; they extract `Extension<SqlitePool>` and call into `app/src/server/*_queries.rs`.

### 1.4 Background Tasks

Spawned in `main.rs` before serving:
- `db::backup::spawn_checkpoint_task` (WAL truncate every 900s default) — `db/backup.rs:13`
- `db::backup::spawn_backup_task` (config-gated, default disabled) — `db/backup.rs:31`
- `monitor::scheduler::spawn_scheduler` (30s tick) — `monitor/scheduler.rs:20`
- `integrations::weather::spawn_scheduler` (60s tick, inert when unconfigured) — `integrations/weather.rs:166`
- `integrations::docker::spawn_scheduler` (60s tick, inert when disabled) — `integrations/docker.rs:123`
- `integrations::kubernetes::spawn_scheduler` (60s tick, inert when disabled) — `integrations/kubernetes.rs:115`

### 1.5 Data Flow

```
Browser ──HTTP──▶ Axum router ──▶ Leptos SSR (shell+App)
                                  │
                                  ▼
                            Server #[server] fn
                                  │ extracts Extension<SqlitePool>
                                  ▼
                            app/server/*_queries.rs (SQL)
                                  │
                                  ▼
                              SQLite (WAL)
                                  │
                                  ▼
                            SseHub (broadcast)
                                  │
                                  ▼
                            /events SSE stream
                                  │
                                  ▼
                        WASM hydrate (EventSource)
```

---

## 2. Per-Subsystem Deep Dive

### 2.1 Leptos SSR+Hydrate Architecture

**What it does**: Server-side renders the full HTML document with theme CSS injected in `<head>` (no flash), then ships a WASM bundle that hydrates the same DOM and wires client-only interactivity (search, SSE listeners, WebAuthn).

**Key files**:
- `crates/frontend/src/lib.rs:8` — `#[wasm_bindgen] pub fn hydrate()` calls `hydrate_body(App)`. The thin WASM entry; the hydration script calls `mod.hydrate()`.
- `crates/app/src/lib.rs:28-96` — `App` component: `provide_meta_context()`, theme `Resource`, CSP `<Meta>` with nonce, `<Style>` for theme CSS, `<Router>` with 11 routes.
- `crates/app/src/lib.rs:99-139` — `format_theme_css()` maps `DesignTokens` to `:root { --var: value; }` CSS custom properties.
- `crates/app/src/lib.rs:231-236` — `#[cfg(feature = "hydrate")] pub fn hydrate()` (duplicate of frontend's; both exist for different build paths).
- `crates/server/src/main.rs:17-37` — `shell()` for SSR: `provide_nonce()`, `<HashedStylesheet>`, `<AutoReload>`, `<HydrationScripts>`.
- `Cargo.toml:56-67` — `[[workspace.metadata.leptos]]`: `name=emberwake`, `hash-files=true`, `style-file=style/main.css`, `lib-features=["hydrate"]`, `bin-features=["ssr"]`.

**Public API surface**:
- `app::App` (component)
- `app::hydrate()` (hydrate feature only)

**Internal contracts**:
- CSP nonce is per-response, generated by `leptos::nonce::provide_nonce()` in `shell()`, consumed via `leptos::nonce::use_nonce()` in `App` (`lib.rs:40`). CSP allows `script-src 'self' 'nonce-{nonce}' 'wasm-unsafe-eval'`. Note: `unsafe` appears only as a CSP keyword string (`lib.rs:44`), not as Rust `unsafe` — confirmed by grep (only 1 match, in a string literal).
- Theme no-flash: `<Style>` is rendered inside `<head>` during SSR via `leptos_meta`, so the browser receives CSS custom properties on first paint. Fallback to `prefers-color-scheme` media queries when no active theme (`lib.rs:69`).
- `cargo-leptos` hashes asset filenames (`hash-files=true`); `hash.txt` is copied into the Docker image (`Dockerfile:65`) so `HashedStylesheet`/`HydrationScripts` can resolve hashed names. `LEPTOS_SITE_ROOT` env var overrides `site_root` (`main.rs:68`).

**Tech debt / risks**:
- `App` contains a *literal* `prefers-color-scheme` fallback CSS string (`lib.rs:69`) that duplicates tokens also defined in `style/main.css` and in `seed_builtin_themes` (`settings_queries.rs:430-479`). Three sources of truth for default theme tokens — drift risk.
- The `hydrate()` function is defined in both `app/src/lib.rs:233` and `frontend/src/lib.rs:8`. The frontend one is the canonical WASM entry per `crates/AGENTS.md:13-14`; the app one exists for unknown reasons (possibly legacy). Mild duplication.
- `shell()` in `main.rs` is used both for SSR routes and the fallback `file_and_error_handler`, ensuring error/404 pages also load assets — good.

### 2.2 Server-Function Pattern (Auth + CSRF + Authorization)

**What it does**: `#[leptos::server]` functions are the typed RPC boundary between WASM client and server. Each mutating function enforces session + CSRF + role authorization, writes an audit event, and fails closed.

**Enforcement layers** (defined in `crates/app/src/server/auth_helper.rs`):
- `require_session(pool)` — extracts `emberwake_session` cookie, looks up session, returns `SessionInfo` or `AppError::Unauthorized` (`auth_helper.rs:13-34`).
- `validate_origin(headers)` — Layer 1 CSRF: Origin header preferred, falls back to Referer, fail-closed if both missing. Checks `host` substring match (`auth_helper.rs:40-71`). **Note**: uses `origin.contains(host)` substring matching, not strict URL-host equality — see Risks.
- `require_session_csrf(pool)` — Layer 2 CSRF: per-session CSRF token comparison (cookie `emberwake_csrf` or `x-csrf-token` header vs. session's stored `csrf_token`). Uses `subtle`-style constant-time? No — `auth_queries::validate_csrf` (`auth_queries.rs:592-597`) does a plain `provided != expected` string compare. See Risks.
- `require_admin_csrf(pool)` — adds `Role::Admin` check (`auth_helper.rs:118-124`).

**Where enforced**:
- Content writes (`content_write.rs:17-21`): every mutating function calls `require_auth_csrf()` which delegates to `require_admin_csrf` — **all content writes require admin** (not just auth). This is stricter than the docs imply.
- Settings/theme mutations (`settings.rs:37-46`): local `require_admin_csrf` (admin-gated).
- Import/export (`import_export.rs:18-26`): local `require_admin_csrf`.
- Extended auth mutations (`extended_auth.rs`): per-function — `unlink_external_identity`, `passkey_register_finish`, `delete_passkey`, `create_api_token`, `revoke_api_token` use `require_session_csrf`; `list_*` use `require_session` only.
- Discovery reads (`discovery.rs:109-116`): `require_admin` (admin-gated, session-only — no CSRF on reads).
- User management (`auth.rs:400-535`): `list_users`, `create_user`, `update_user`, `deactivate_user` — admin-gated, CSRF on mutations.
- `revoke_session` (`auth.rs:306-353`): allows user to revoke own session without admin, but checks ownership.

**Where it's TODO / partial**:
- `crates/AGENTS.md:60` says "fail-closed auth with TODO for Phase 5 session wiring" — but Phase 5 is marked complete in SPEC_STATE.md. The TODO appears stale; session wiring is fully implemented in `auth_helper.rs`.
- `oidc.rs` callback (`oidc.rs:200-207`): on approved identity, calls `create_session` and sets cookies directly via `axum::response::Response::builder()` — bypasses the Leptos `ResponseOptions` path used by `login()`. This is because OIDC callback is a plain Axum handler, not a server function. The cookies are set correctly but the code path is divergent.
- `oidc.rs:228`: `username = format!("oidc_{}", &subject[..subject.len().min(32)])` — truncates OIDC subject to 32 chars for username. Could collide if two subjects share a 32-char prefix.

**Audit logging**: every mutating server function calls `auth_queries::audit_write_query` (best-effort insert into `audit_event`). Failures only log a warning (`auth_queries.rs:629-631`).

### 2.3 Data Model (SQLite Schema)

5 migrations in `migrations/`. All PKs are TEXT (UUIDv7), timestamps are RFC3339 TEXT, foreign keys ON, WAL mode.

**Tables** (from `000001_initial.sql` + amendments):

| Table | Columns | Notes |
|---|---|---|
| `users` | id PK, username UNIQUE NOCASE, email UNIQUE NULL, password_hash NULL, role default 'user', is_active default 1, created_at, updated_at, last_login_at NULL | OIDC-only users have NULL password_hash (`oidc.rs:231-240`) |
| `sessions` | id PK, user_id FK→users CASCADE, created_at, expires_at, last_used_at, user_agent NULL, ip NULL, csrf_token (added 000002) | Opaque server-side tokens, revocable. Index on user_id. |
| `external_identity` | id PK, user_id FK→users CASCADE, provider, subject, created_at, approved (added 000003) | UNIQUE(provider, subject). Admin-approve policy. |
| `passkey_credential` | id PK, user_id FK→users CASCADE, credential_id BLOB UNIQUE, public_key BLOB, sign_count, created_at | WebAuthn credentials. |
| `api_token` | id PK, user_id FK→users CASCADE, name, token_hash, scopes JSON default '[]', expires_at NULL, revoked_at NULL, created_at, last_used_at NULL | Only HMAC-SHA256 hash stored. |
| `category` | id PK, name, icon NULL, order_index, visibility CHECK(public/private/restricted — added 000004), created_at, updated_at | |
| `service` | id PK, category_id FK→category SET NULL, name, url, icon NULL, description NULL, is_pinned, order_index, visibility, monitor_enabled, monitor_kind CHECK(NULL\|http\|tcp), monitor_target NULL, monitor_interval_s NULL, created_at, updated_at | |
| `bookmark` | id PK, category_id FK→category SET NULL, name, url, icon NULL, order_index, visibility, created_at, updated_at | |
| `application` | id PK, category_id FK→category SET NULL, name, url, icon, description, is_pinned, order_index, visibility, created_at default now, updated_at default now | Added in 000005. Has its own defaults via `strftime`. |
| `setting` | key PK, value, updated_at | Typed key/value JSON store. `setup_complete` key singleton enforces first-run race safety. |
| `theme` | id PK, name, tokens JSON, custom_css NULL, is_builtin, created_by FK→users SET NULL, created_at | |
| `status_reading` | service_id PK FK→service CASCADE, state CHECK(up/down/degraded), latency_ms NULL, reason NULL, checked_at | One current row per monitored service. |
| `status_history` | id PK, service_id FK→service CASCADE, state, latency_ms NULL, reason NULL, checked_at | Bounded retention. Index `(service_id, checked_at)`. |
| `weather_reading` | id INTEGER PK DEFAULT 1, temp REAL NULL, condition NULL, is_day NULL, cloud NULL, upstream_ts NULL, fetched_at, CHECK(id=1) | Single-row cache. |
| `audit_event` | id PK, ts, actor_id FK→users SET NULL, action, target NULL, ip NULL, user_agent NULL, result CHECK(success/failure) | Append-only. Indexes on ts and actor_id. |

**Relationships**:
- `users` 1—∞ `sessions`, `external_identity`, `passkey_credential`, `api_token`
- `category` 1—∞ `service`, `bookmark`, `application` (with SET NULL on category delete)
- `service` 1—1 `status_reading`, 1—∞ `status_history`

**Constraints / invariants**:
- `setup_complete` setting key UNIQUE → race-safe first-run admin creation (`foundational.rs:77-97` tests this).
- `external_identity` UNIQUE(provider, subject) → one OIDC account per IdP subject.
- `passkey_credential` UNIQUE(credential_id) → one credential per WebAuthn ID.
- `weather_reading` CHECK(id=1) → single-row cache enforced at DB level.
- `visibility` CHECK constraint (public/private/restricted) verified by `foundational.rs:100-116`.

### 2.4 Auth Deep Dive

#### 2.4.1 Password (Argon2)

- `auth_queries.rs:34-50` `hash_password`: Argon2id, configurable m/t/p costs. Default m=32 MiB, t=3, p=1 (`config.rs:74-82`).
- `auth_queries.rs:53-61` `verify_password`: uses `Argon2::default()` (ignores configured params on verify — fine, PHC string carries params).
- Salt: 16 random bytes via `getrandom::fill` (`auth_queries.rs:40`).
- Password validation: min 8 chars enforced in `complete_setup_query` (`auth_queries.rs:97`) and `create_user_query` (`auth_queries.rs:438`).
- `crates/AGENTS.md:32`: "No `unsafe` in application code" — `argon2` crate is unsafe-free.

#### 2.4.2 Session (Cookie + Rotation)

- `auth_queries.rs:148-175` `create_session`: 32-byte random token (hex), separate CSRF token, 24h absolute expiry.
- `auth_queries.rs:190-286` `lookup_session`: checks `is_active`, `expires_at`, then rotation logic:
  - `SESSION_ROTATION_INTERVAL_MINUTES = 30` (`auth_queries.rs:145`): if `last_used_at` is older than 30 min, generate new token, UPDATE row's `id` (rotates the PK), return `rotated_token: Some(new_token)`.
  - `SESSION_IDLE_TIMEOUT_MINUTES = 30` (`auth_queries.rs:141`): if idle > 30 min (and not just rotated), DELETE session, return None.
  - Order matters: rotation check runs BEFORE idle check so just-rotated sessions survive (`auth_queries.rs:236-269`).
- Cookies: `emberwake_session` (HttpOnly, SameSite=Lax, Max-Age=86400, Secure if `is_secure_request()`) and `emberwake_csrf` (SameSite=Lax, NOT HttpOnly — readable by JS to send as header) (`auth_queries.rs:667-713`).
- `is_secure_request()` checks `X-Forwarded-Proto: https` (`auth_queries.rs:655-665`).
- Rotation is tested: `tests/server_fn/auth.rs:372` `session_token_rotates_after_interval`.

#### 2.4.3 OIDC

- `crates/server/src/auth/oidc.rs`: full auth-code + PKCE flow via `openidconnect` 4.0 crate (rustls-tls, no default features).
- Routes: `/auth/oidc/login` (redirect to IdP), `/auth/oidc/callback` (exchange code).
- In-memory PKCE state store: `OidcStateStore` (`Arc<Mutex<HashMap>>`) keyed by CSRF state, with a process-global `LazyLock` (`oidc.rs:53`). **Not shared across processes** — single-process deployment only. See Risks.
- Provisioning: admin-approve policy. New OIDC identity → create local user with NULL password (`oidc.rs:230-240`) → create `external_identity` with `approved=0` (`oidc.rs:242-245`) → return 403 "pending approval". Admin must approve via... no server function exists for approval. `extended_auth_queries::approve_external_identity` is defined (`extended_auth_queries.rs:74-83`) but **no `#[server]` function calls it**. This is a gap — see Tech Debt.
- Scopes requested: `openid`, `email`, `profile` (`oidc.rs:103-105`).

#### 2.4.4 WebAuthn Passkeys

- Server side: `extended_auth.rs:130-250` — `passkey_register_begin`/`finish`, `passkey_login_begin`/`finish`, `list_passkeys`, `delete_passkey`.
- `build_webauthn` (`extended_auth.rs:131-136`): `WebauthnBuilder::new(&rp_id, &origin)`, rp_name "Emberwake".
- RP ID is computed in `main.rs:110-117` as `localhost:{port}` from `bind_addr` — **hardcoded to localhost**, won't work for real deployments. See Risks.
- Challenge store: in-memory `ChallengeStore` (`Arc<Mutex<HashMap>>`) via Axum Extension (`extended_auth.rs:29-52`). Single-process only.
- `webauthn-rs` feature `danger-allow-state-serialisation` enabled (`Cargo.toml:38`) — required to serialize `SecurityKeyRegistration`/`SecurityKeyAuthentication` state into the challenge store.
- Client side: `components/auth/account_page.rs:17-68` `webauthn_create()` — calls `navigator.credentials.create()`, base64url-encodes `attestationObject` and `clientDataJSON`, returns `RegisterResponse`. Uses `unwrap()` on `js_sys::Reflect::get` (`account_page.rs:34, 36`) — panics if the API shape is wrong (defensive but brittle).

#### 2.4.5 API Tokens (HMAC-SHA256)

- `extended_auth_queries.rs:276-283` `generate_token_secret`: `ew_<base64url(32 random bytes)>`.
- `extended_auth_queries.rs:286-290` `hash_token`: `HMAC-SHA256(server_key, token)` → hex.
- `extended_auth_queries.rs:308-346` `create_api_token_query`: only `token_hash` stored, secret returned once in `ApiTokenSecret`.
- `extended_auth_queries.rs:349-403` `verify_api_token`: computes hash, looks up by `token_hash = ?`, checks `revoked_at IS NULL`, checks `expires_at`, updates `last_used_at`.
- `api_token.rs:13-64` `verify_bearer`: extracts `Authorization: Bearer <token>`, calls `verify_api_token`, audits every use.
- `api_token.rs:67-79` `require_scope`: checks `verified.scopes` contains required scope.
- Valid scopes (`domain/mod.rs:488-493`): `services:read`, `services:write`, `bookmarks:read`, `export`.
- **If `server_key` is empty, API tokens are disabled** (`main.rs:95-100`) — `verify_api_token` would fail HMAC construction. The warning is logged but the extension is still wired with an empty key.

### 2.5 Settings (Encryption at Rest)

**What it does**: Secret-bearing settings (`weather`, `auth` keys) are encrypted at rest using an XOR keystream PRF derived from HMAC-SHA256, keyed by `server_key`.

**Key files**:
- `settings_queries.rs:35-61` `encrypt_secret`: 16-byte nonce ‖ ciphertext. Keystream = HMAC-SHA256(server_key, nonce ‖ counter.to_le_bytes()) per 32-byte block, XOR'd with plaintext.
- `settings_queries.rs:64-92` `decrypt_secret`: inverse.
- `settings_queries.rs:94-97` `is_secret_key`: keys starting with `weather` or `auth`.
- `settings_queries.rs:121-144` `set_setting_raw`: upsert, encrypts if secret-bearing and `server_key` non-empty.
- `settings_queries.rs:147-163` `get_setting_decrypted`: decrypts on read.
- `settings_queries.rs:169-187` `get_settings_view`: assembles typed view; redacts secrets (sets to None) when `include_secrets=false`.
- Round-trip tests: `settings_queries.rs:486-519` (3 tests: roundtrip, empty, long).

**Invariants**:
- If `server_key` is empty, secrets are stored in plaintext (no encryption). `main.rs:95-100` warns but does not refuse to start. See Risks.
- Secret-bearing keys never returned to non-admins (`settings.rs:65-85` `get_settings` checks role).
- Export excludes secrets: `export_queries.rs:151-178` skips keys starting with `weather` or `auth`.
- The encryption is a custom XOR stream cipher using HMAC-SHA256 as PRF. This is a roll-your-own crypto pattern. Per `crates/AGENTS.md:67` it's documented as intentional. The HMAC-SHA256 PRF is sound, but the construction is non-standard (not AES-GCM, not ChaCha20-Poly1305). See Risks.

### 2.6 Monitoring (US6)

**What it does**: Background scheduler runs HTTP/TCP health checks on `monitor_enabled=true` services, records state+latency, inserts bounded history, emits SSE on state change.

**Key files**:
- `server/src/monitor/mod.rs:23-78` `http_check`: GET with timeout, 2xx=up, 3xx/4xx=degraded, timeout/error=down.
- `monitor/mod.rs:82-108` `tcp_check`: `TcpStream::connect` with timeout.
- `monitor/mod.rs:124-187` `check_service`: runs check → upserts `status_reading` → inserts `status_history` → prunes history (max 1000 rows, max 30 days: `monitor_queries.rs:15-17`) → emits SSE if state changed (first check always emits).
- `monitor/scheduler.rs:20-49` `spawn_scheduler`: 30s tick, `list_monitored_services`, `tokio::spawn` per service (concurrent).
- `app/server/monitor_queries.rs`: SQL functions. `prune_status_history` (`monitor_queries.rs:142-182`) deletes by age then by row count.
- `app/server/monitor_read.rs`: `get_service_statuses` (public sees public-only, auth sees all, admin sees restricted — via `visibility_for_caller`), `get_uptime_summary` (verifies service visibility before computing).
- `app/server/monitor_queries.rs:225-262` `compute_uptime_summary`: counts `up` rows in `status_history` over a window, returns `uptime_percent`.

**Tests**: `tests/integration/monitor.rs` (437 lines, 7+ tests), `tests/integration/sse.rs` (2 tests).

**Notable**: disabled services make NO outbound calls (SQL filters `monitor_enabled=1` in `list_monitored_services`).

### 2.7 Weather Integration (US7)

**What it does**: Scheduled fetch from WeatherAPI-style endpoint, single-row DB cache, SSE push on refresh. Config-gated: inert when unconfigured.

**Key files**:
- `integrations/weather.rs:32-92` `fetch_weather`: reqwest GET with 15s timeout, parses `current.temp_c`, `condition.text`, `is_day`, `cloud`, `last_updated`.
- `weather.rs:96-161` `refresh_weather`: reads `WeatherSettings` from DB (decrypted), config-gated, fetches, upserts `weather_reading`, emits `SseEvent::Weather`. On upstream error: retains cache, logs warning, does NOT emit.
- `weather.rs:166-206` `spawn_scheduler`: 60s tick, checks if configured, runs refresh, resets interval to configured (default 600s, min 60s).
- `app/server/weather_queries.rs`: `get_weather_reading`, `upsert_weather_reading` (single-row, id=1).
- `app/server/weather_read.rs:11-28` `get_weather`: public, serves cache only.
- `components/dashboard/weather_widget.rs`: renders temp/condition/day-indicator; EventSource on `/events` for live updates. Inert (shows nothing) when no data.

**Tests**: `tests/integration/weather.rs` (245 lines) — stub HTTP server returning WeatherAPI JSON, tests caching + SSE + inert-when-disabled.

### 2.8 Discovery (US8)

**What it does**: Docker (bollard) and Kubernetes (kube) integrations list containers/ingresses, parse `emberwake.*` labels/annotations, populate a thread-safe cache, emit SSE on add/remove. Strictly read-only.

**Key files**:
- `server/src/integrations/labels.rs:23-56` `parse_labels`: pure function, parses `emberwake.name` (required), `emberwake.url` (required, comma-separated for multi-value), `emberwake.icon`/`category`/`description` (optional). Missing name/url → no services. Multi-value URL → one service per URL.
- `integrations/docker.rs:28-35` `connect`: checks socket path exists before connecting (graceful None if absent).
- `docker.rs:39-59` `list_containers`: `list_containers(all=true)`, filters labels, flattens via `parse_labels`.
- `docker.rs:63-117` `watch_events`: subscribes to Docker events; on `start` → inspect + parse + cache.add + SSE Added; on `die`/`stop` → cache.remove + SSE Removed.
- `docker.rs:123-157` `spawn_scheduler`: 60s tick, reads `IntegrationSettings.docker_enabled`, if disabled skip, else connect + list + watch.
- `integrations/kubernetes.rs`: analogous with `kube::Api<Ingress>`, `watcher` stream, `Apply`/`Delete` events.
- `app/server/discovery.rs:14-79` `DiscoveryCache`: `Arc<RwLock<Vec<DiscoveredService>>>` for docker and k8s. Cloneable. Methods: `get_*`, `set_*`, `add_*`, `remove_*` (retain by source_id).
- `discovery.rs:120-164` `discover_docker`/`discover_kubernetes`: admin-gated server functions, return empty vec when disabled (no calls), read from cache when enabled.
- SSE: `SseEvent::Discovery(SseDiscoveryEvent)` (`sse/mod.rs:20`), `SseHub::broadcast_discovery` (`sse/mod.rs:66-68`). SSE handler sends discovery events to **admin sessions only** (`sse/handler.rs:82-89`).

**Read-only guarantee**: by construction — only `list_containers`, `inspect_container`, `events` are called for Docker; only `list` and `watcher` for K8s. No mutating API calls. Per `crates/AGENTS.md:155, 163`.

**Tests**: `tests/integration/discovery.rs` (411 lines, 12 tests): label parsing, multi-value, missing fields, empty values, `has_emberwake_labels`, cache+SSE, disabled=empty.

### 2.9 Import/Export (US9)

**What it does**: Export all data to JSON (excluding secrets); import JSON/HTML(Netscape)/OPML with size+depth limits, preview-then-apply, transactional, duplicate handling.

**Key files**:
- `app/server/importer/mod.rs:16-19`: `MAX_IMPORT_SIZE = 10 MB`, `MAX_DERIVATION_DEPTH = 100`.
- `importer/json.rs:11-82` `parse_json`: checks nesting depth (counts consecutive `{`/`[`), parses `ExportDocument`, validates `version == "1.0"`.
- `importer/html.rs:15-52` `parse_html`: `scraper` crate, counts nested `<dl>` elements for depth, walks DOM in document order mapping `<h3>`→category, `<a href>`→bookmark.
- `importer/opml.rs:38-136` `parse_opml`: `quick-xml` streaming, tracks `outline` depth, `xmlUrl` → bookmark, title/text → category stack.
- `app/server/export_queries.rs:18-76` `export_data_query`: Full or Selective scope; excludes secrets (weather/auth keys).
- `app/server/import_export.rs:30-48` `export_data`: admin-gated.
- `import_export.rs:53-111` `import_preview`: admin-gated, `spawn_blocking` parse, NO writes, returns `ImportPreviewData` with base64-encoded `ParsedData` token.
- `import_export.rs:116-494` `import_apply`: admin-gated, decodes token, transactional (all-or-nothing), regenerates UUIDs, `DuplicateStrategy` (Skip/Overwrite/Rename) per entity, audits.

**Fuzz targets**: `fuzz/fuzz_targets/{import_html,import_json,import_opml}.rs` — call sync parsers with arbitrary bytes, assert no panic/OOM. Run in CI (`ci.yml:90-95`) with `cargo +nightly fuzz run -- -max_total_time=60 -runs=100000`.

**Tests**: `tests/integration/export_import.rs` (574 lines, 11 tests): round-trip, HTML/OPML parsing, duplicate skip/overwrite, reject oversized/malformed/deeply-nested, no partial writes on rejection.

**Risk**: `import_apply` decodes a base64 `ParsedData` token from the client. The token is **not authenticated** — a malicious admin could craft an arbitrary `ParsedData` with hand-crafted SQL-ish fields. The apply loop uses parameterized SQL, so injection is mitigated, but the preview/apply separation trusts the client-supplied token. See Risks.

### 2.10 Security Headers, Rate Limiting, CSP Nonce

- **Headers** (`server/src/security/headers.rs`): HSTS (`max-age=N; includeSubDomains`, default 31536000), `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`. Applied as tower layers in `main.rs:144-149`. `apply_security_headers` is the shared function used by main.rs and tests (no mock duplication — `foundational.rs:138-216`).
- **CSP**: per-response nonce via `leptos::nonce::provide_nonce()` in `shell()`, injected as `<Meta http_equiv="Content-Security-Policy">` in `App`. Allows `default-src 'self'; script-src 'self' 'nonce-{nonce}' 'wasm-unsafe-eval'; style-src 'self' 'nonce-{nonce}'; img-src 'self' data: https:; font-src 'self' https://fonts.googleapis.com https://fonts.gstatic.com; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'`. **Not verified in unit tests** — the foundational test explicitly notes CSP requires the full cargo-leptos SSR pipeline (`foundational.rs:140-143`).
- **Rate limiting** (`security/rate_limit.rs`): `tower_governor` + `governor`. Default 60/min, login 10/min, token 20/min, import 5/min. Per-IP via `PeerIpKeyExtractor`. Applied conditionally on `config.security.rate_limit_enabled` (default false — `config.rs:191-193`).

### 2.11 CI Pipeline

**`.github/workflows/ci.yml`** — 5 jobs:
1. `has-cargo` — guard, checks `Cargo.toml` exists.
2. `lint-and-test` — matrix amd64+arm64, `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, install cargo-leptos, `cargo leptos test`.
3. `supply-chain` — `cargo-deny check` (advisories/licenses/bans) + `cargo-audit`.
4. `fuzz-smoke` — matrix amd64+arm64, nightly, `cargo +nightly fuzz run` for 60s/100k runs on all 3 import targets.
5. `docs` — always-on, asserts `.specify/memory/constitution.md`, `specs/001-greenfield/{spec,plan,tasks}.md` exist.

**`RUSTFLAGS: "-D warnings"`** (`ci.yml:21`) — clippy warnings are build failures (Constitution Principle I).

**`.github/workflows/release.yml`** — triggered by `v*` tags:
1. Supply-chain gate (cargo-deny + cargo-audit) blocks publish.
2. docker buildx multi-arch (amd64+arm64) → GHCR, with provenance + SBOM.
3. cosign keyless sign via OIDC.
4. CycloneDX SBOM generated, attached to GitHub Release.

**Gates encoded**: clippy zero-warning (Principle I), cargo-deny/audit (Principle II), fuzz smoke (Principle IV), signed image + SBOM (Principle V), both-arch CI (SC-008).

---

## 3. Test Coverage Map

**Test files** (`crates/server/tests/`):

| File | Lines | Tests | Coverage |
|---|---|---|---|
| `foundational.rs` | 232 | 7 | Migrations create all tables; repository round-trip; setup_complete UNIQUE; visibility CHECK; audit_result CHECK; security headers; OTLP exporter |
| `server_fn/auth.rs` | 443 | 12 | Setup open/complete/single-shot; login success/wrong-password/nonexistent; logout; revoke session; revoke all other; session rotation |
| `server_fn/authz.rs` | 137 | 2 | Anonymous sees only public; authenticated sees all |
| `server_fn/csrf.rs` | 175 | 9 | CSRF valid/missing/invalid/empty; Origin correct/wrong/both-missing/referer-fallback/referer-wrong/origin-precedence |
| `server_fn/content_write.rs` | 466 | 8 | Category/service/bookmark create/update/delete/reorder; pin toggle; delete-reparents |
| `server_fn/application.rs` | 258 | 5 | Application CRUD, reorder, pin, dashboard, with-category |
| `server_fn/dashboard.rs` | 215 | 3 | Dashboard public-only, all-includes-private, search-provider wiring |
| `server_fn/dashboard_settings.rs` | 40 | 3 | Defaults, persist, overwrite |
| `server_fn/settings.rs` | 334 | 11 | Theme CRUD, set_active, list, settings update, admin-sees-secrets, non-admin-redacted, encrypted-at-rest, empty-name-rejected, builtin seeding idempotent |
| `server_fn/api_token.rs` | 366 | ~10 | Create/verify in-scope/out-of-scope/revoked/missing; HTTP bearer layer |
| `integration/content_crud.rs` | 146 | — | Content CRUD integration |
| `integration/monitor.rs` | 437 | 7 | Check records state/latency, history written, SSE on transition, disabled not listed, uptime summary, pruning by age/rows, visibility filter |
| `integration/sse.rs` | 129 | 2 | Client receives status event on flip; no event on same state |
| `integration/ssr_theme.rs` | 239 | — | Active theme in HTTP response, no-flash |
| `integration/ssr_dashboard.rs` | 215 | — | SSR dashboard rendering |
| `integration/oidc.rs` | 418 | — | OIDC identity created unapproved, approved, find, unlink |
| `integration/webauthn.rs` | 485 | — | Passkey store/find/update/list/delete, sign count, session via passkey |
| `integration/weather.rs` | 245 | — | Fetch caches + SSE, get_weather reads cache, missing config inert |
| `integration/discovery.rs` | 411 | 12 | Label parsing, multi-value, missing fields, cache+SSE, disabled=empty |
| `integration/export_import.rs` | 574 | 11 | Round-trip, HTML/OPML parse, duplicate skip/overwrite, reject oversized/malformed/deep, no partial writes |

**E2E** (`e2e/src/lib.rs`, 540 lines, 7 tests, all `#[ignore]` requiring WebDriver): setup, login, CRUD, search, edit, delete, logout.

**Fuzz** (`fuzz/fuzz_targets/`, 3 targets): import_html, import_json, import_opml — assert no panic/OOM on arbitrary bytes.

**WASM tests**: `crates/app/src/components/search/fuzzy.rs:104-153` — 3 `wasm_bindgen_test` tests for fuzzy matcher (run via `cargo leptos test`).

**Coverage gaps / untested areas**:
- **CSP header**: not verified in unit tests (requires full SSR pipeline). Noted in `foundational.rs:140-143`.
- **OIDC full flow**: `oidc.rs` tests are stub-IdP unit tests on `extended_auth_queries` only — the actual `oidc_login`/`oidc_callback` HTTP handlers (`oidc.rs:67-250`) are **not tested** (would require a mock IdP server). The PKCE state store, token exchange, ID token verification paths are untested.
- **WebAuthn full flow**: `webauthn.rs` tests `extended_auth_queries` storage layer only — `passkey_register_begin/finish`, `passkey_login_begin/finish` server functions are not integration-tested (would require a WebAuthn authenticator emulator). `build_webauthn` is tested.
- **Approval workflow**: `approve_external_identity` is defined but **never called by any server function or test**. Admins have no UI/API to approve OIDC identities. Dead code or unfinished feature.
- **Docker/K8s live integration**: `docker.rs`/`kubernetes.rs` `spawn_scheduler` and `list_*`/`watch_*` are not tested against real Docker/K8s — only `labels::parse_labels` is unit-tested. The graceful-absence path (`connect` returns None) is not tested.
- **Backup task**: `db/backup.rs` `spawn_backup_task`/`create_backup`/`prune_backups` — no tests. Default disabled.
- **Public API write paths**: `public_api.rs` `create_service`/`update_service`/`delete_service` — only `api_token.rs` tests the bearer auth layer; the actual handlers' SQL and response shaping are not directly tested.
- **Rate limiting**: not tested (default disabled, no test exercises the governor layers).
- **Import preview token**: `encode_token`/`decode_token` round-trip is not directly unit-tested (covered indirectly by export_import.rs integration tests).

---

## 4. Tech Debt Inventory

### 4.1 TODOs / FIXMEs / Markers

Grep for `TODO|FIXME|XXX|HACK|todo!()|unimplemented!()|panic!()|TBD|deferred` in `crates/{app,server}/src`:
- **Zero TODO/FIXME/XXX/HACK comments** in application code (app/src and server/src). Clean.
- **No `todo!()` or `unimplemented!()` macros** in application code.
- `panic!()` only in `server/src/telemetry.rs:53` (OTLP exporter creation — intentional fail-fast on startup) and `server/src/config.rs:219, 232` (unreadable `*_FILE` secret path — intentional fail-fast on misconfiguration). Both are startup-time only, acceptable.
- `crates/AGENTS.md:60` mentions "TODO for Phase 5 session wiring" but Phase 5 is complete and the code is implemented — **stale doc claim** (DOX violation: AGENTS.md should be updated).

### 4.2 `unwrap()` / `expect()` in Application Code

**`server/src`** (15 matches):
- `auth/oidc.rs:219, 256, 265` — `.unwrap()` on `Response::builder().body()` and `error_response`. These builders can only fail on invalid header values which are static strings — acceptable but `expect()` with a message would be cleaner.
- `main.rs:41, 54, 57, 75, 173, 176` — `.expect("failed to ...")` on startup config load, DB init, bind. Startup-only, fail-fast is correct.
- `security/rate_limit.rs:18, 28, 38, 48` — `.expect("valid governor config")` on static config builders. Acceptable.
- `security/headers.rs:27` — `.expect("valid HSTS header")` on static header value. Acceptable.
- `telemetry.rs:81` — `.expect("valid counter opts")` on Prometheus counter. Acceptable.

**`app/src`** (18 matches):
- `server/discovery.rs:36, 41, 46, 51, 56, 61, 68, 76` — `.unwrap()` on `RwLock` read/write. **Lock poisoning panics**. If a background task panics while holding the lock, all subsequent cache accesses panic. See Risks.
- `server/auth_queries.rs:26`, `extended_auth_queries.rs:278` — `.expect("getrandom failed")`. Acceptable (getrandom failure = system is broken).
- `server/settings_queries.rs:494-517` — `.expect("encrypt"/"decrypt")` — **test-only**, fine.
- `components/auth/account_page.rs:34, 36` — `.unwrap()` on `js_sys::Reflect::get` for `navigator.credentials.create`. **Panics if the API shape is unexpected**. Should use `?` with an error string. See Risks.
- `components/dashboard/status_tile.rs:72`, `dashboard/mod.rs:87, 90` — `.unwrap_or('E')`/`'A'` on `chars().next()` — safe (defaults provided), not a panic.

### 4.3 `unsafe`

Grep for `unsafe` in `crates/`: **1 match** — `crates/app/src/lib.rs:44`, which is the string `'wasm-unsafe-eval'` inside the CSP `script-src` directive. **No Rust `unsafe` blocks or functions.** Constitution Principle I (no `unsafe` in application code) is satisfied.

### 4.4 Deferred / Unfinished Items (from SPEC_STATE.md)

- **Phase 13 (T085-T093) — Three-Section Dashboard Redesign**: "Not started". The dashboard currently has three sections (`components/dashboard/mod.rs`) but the redesign is pending.
- **Logout browser verification (BV-013)**: not verified in browser (E2E test exists but is `#[ignore]`).

### 4.5 Code Smells

- **Duplicated cookie-setting code paths**: `login()` (`auth.rs:124-144`) uses `ResponseOptions`; `oidc_callback` (`oidc.rs:213-218`) uses `Response::builder()` directly. Divergent.
- **Duplicated audit-writing code**: `AuditWriter` struct in `server/src/audit.rs` (41-70) AND `audit_write_query` function in `app/src/server/auth_queries.rs` (601-632). Two paths to write the same `audit_event` table. The server functions all use `auth_queries::audit_write_query`; the `AuditWriter` in `AppState` (`state.rs:19`) appears unused by server functions. Possibly dead code or used elsewhere.
- **`AuditWriter` in AppState but unused by server functions**: server functions call `app::server::auth_queries::audit_write_query` directly with the pool, not via `AppState.audit`. The `Arc<AuditWriter>` in `AppState` may only be used by Axum handlers (none observed) or is vestigial.
- **`create_application_query` hardcodes `is_pinned = 1i64`** (`content_write_queries.rs:362`) — applications are always created pinned. The `ApplicationInput` has `is_pinned` field but it's ignored on insert. Bug or intentional?
- **Three sources of truth for default theme tokens**: `lib.rs:69` (fallback CSS), `style/main.css:31-54` (`:root` defaults), `settings_queries.rs:430-479` (seeded Light/Dark themes). Drift risk.
- **`reorder_*` functions ignore their `category` parameter**: `reorder_services_query` (`content_write_queries.rs:286-301`) and `reorder_applications_query` (`content_write_queries.rs:461-476`) take `_category: Option<Uuid>` but don't use it — reordering is global, not per-category. The `reorder_bookmarks_query` (`content_write_queries.rs:626-641`) takes `_category: Uuid` and also ignores it. Either a bug (reorder should be scoped) or the parameter should be removed.
- **`DashboardView.settings` is always `DashboardSettings::default()`** from `list_dashboard_query` (`content_queries.rs:199-204`) — the actual dashboard settings from the `setting` table are fetched separately via `get_dashboard_settings` server function. The `DashboardView.settings` field is dead/always-default.
- **`oidc_begin` server function** (`extended_auth.rs:57-68`) just returns a hardcoded `/auth/oidc/login` redirect URL — it doesn't do anything the client couldn't do itself. Thin wrapper.
- **`webauthn.rs` in server crate is a doc-only stub** (`server/src/auth/webauthn.rs:1-4`) — 4 lines, just a module doc comment. The actual WebAuthn logic lives in `app/src/server/extended_auth.rs`. The `crates/AGENTS.md:37` says "Security-critical code (auth, CSRF, sessions) lives in `server/`, never in `app/`" — but WebAuthn challenge generation/verification lives in `app/`. Minor contract inconsistency.

---

## 5. Build / Deploy Pipeline Summary

### 5.1 Local Build

- `cargo build` — workspace compiles.
- `cargo clippy --all-targets --all-features -- -D warnings` — zero warnings.
- `cargo fmt --all --check` — formatting.
- `cargo leptos build` — produces WASM (`frontend` crate → `target/site/pkg/*.wasm`) + server binary (`server` → `target/release/emberwake`).
- `cargo leptos test` — runs workspace tests + WASM tests.
- `cargo test -p e2e --manifest-path e2e/Cargo.toml -- --ignored` — E2E (requires WebDriver + running server).
- `cargo +nightly fuzz run <target>` — fuzz (requires nightly).

### 5.2 Docker

`.docker/Dockerfile` (86 lines): 3-stage.
1. **chef planner**: `cargo-chef prepare` from `Cargo.toml`+`Cargo.lock`.
2. **builder**: `cargo chef cook --release` (cached deps), `cargo leptos build --release` (server bin + hashed WASM bundle).
3. **runtime**: `ubuntu:26.04`, non-root user UID 10001, `ca-certificates` + `curl` only, copies `emberwake` binary + `hash.txt` + `target/site/` → `/var/lib/emberwake/site`, `VOLUME ["/var/lib/emberwake"]`, `HEALTHCHECK` → `/readyz`, `EXPOSE 5005`. Env: `EMBERWAKE_DB_PATH`, `LEPTOS_SITE_ROOT`, `RUST_LOG=info`.

### 5.3 Multi-arch

`build-multiarch.sh`: `docker buildx build --platform linux/amd64,linux/arm64 -f .docker/Dockerfile -t ghcr.io/draco-lunaris/emberwake:{version,latest} --push`. Note: hardcoded to `draco-lunaris/emberwake` — may not match `github.repository_owner`.

### 5.4 Compose

`docker-compose.yml`: single `emberwake` service, `user: "10001:10001"`, `read_only: true`, `tmpfs: [/tmp]`, `volumes: emberwake-data:/var/lib/emberwake`, port 5005. Placeholder note "finalized in Phase 12" — but Phase 12 is complete.

### 5.5 Supply Chain

`deny.toml`: bans `native-tls` (rustls-only), allows permissive licenses (Apache-2.0, MIT, BSD, ISC, Unicode-3.0, Zlib, MPL-2.0, CC0-1.0), denies strong copyleft by omission, `yanked = "deny"`, `unmaintained = "workspace"`, `wildcards = "deny"`, `multiple-versions = "warn"`. Note: `openssl`/`openssl-sys` NOT banned (webauthn-rs uses them for X.509 attestation cert parsing, not TLS — per `deny.toml:33-35`).

---

## 6. Notable Risks and Inconsistencies

### 6.1 Security Risks

1. **CSRF token comparison is not constant-time**: `auth_queries::validate_csrf` (`auth_queries.rs:592-597`) uses `provided != expected` plain string comparison — vulnerable to timing side-channels. The `subtle` crate IS in `Cargo.toml:43` and `server/Cargo.toml:49` but is not used for CSRF comparison. Should use `subtle::ConstantTimeEq`.

2. **Origin validation uses substring matching**: `auth_helper.rs:50, 60` — `origin.contains(host)`. An attacker could register `evil-localhost:5005.evil.com` and pass the `contains("localhost:5005")` check. Should parse the Origin URL and compare scheme+host+port strictly.

3. **`host.is_empty()` bypass**: `auth_helper.rs:50, 60` — if `host` header is empty, `origin_matches` is always true. A request with no Host header bypasses CSRF origin check. Axum normally requires Host, but this is a defense-in-depth gap.

4. **Custom encryption at rest (non-standard)**: `settings_queries.rs:35-92` rolls an XOR stream cipher using HMAC-SHA256 as PRF. While the HMAC-SHA256 PRF is cryptographically sound, the construction is non-standard (not authenticated encryption — no integrity tag). An attacker with DB write access could tamper with ciphertext undetectably. Industry standard would be AES-256-GCM or ChaCha20-Poly1305 (both available in RustCrypto). Documented as intentional in `crates/AGENTS.md:67` but worth re-evaluating.

5. **Import preview token is unauthenticated**: `import_export.rs:498-512` — `encode_token`/`decode_token` are plain base64 of JSON. A malicious admin client could craft an arbitrary `ParsedData` token with hand-crafted fields and submit it to `import_apply`. The apply loop uses parameterized SQL (no injection), but the client controls what gets imported, bypassing the parser's size/depth limits. Should HMAC-sign the token with `server_key`.

6. **Empty `server_key` disables encryption**: `settings_queries.rs:128` — if `server_key` is empty, secrets are stored in plaintext. `main.rs:95-100` only warns. Should refuse to start if secret-bearing settings exist with empty key, or refuse to write secrets with empty key.

7. **OIDC PKCE state store is process-local**: `oidc.rs:53` `static OIDC_STORE: LazyLock<OidcStateStore>` — in-memory, lost on restart, not shared across replicas. Multi-instance deployment would break OIDC login. Same for `ChallengeStore` (`extended_auth.rs:30`) and `DiscoveryCache` (passed as Extension, but `Arc<RwLock<Vec>>` — process-local). All assume single-process deployment.

8. **WebAuthn RP ID hardcoded to localhost**: `main.rs:110-117` — `rp_id = "localhost:{port}"`, `rp_origin = "http://localhost:{port}"`. **Will not work for real deployments** over HTTPS with a real domain. Must be configurable. Currently impossible to deploy WebAuthn in production.

9. **`account_page.rs:34, 36` `unwrap()` on WebAuthn JS calls**: panics if `navigator.credentials.create` shape is unexpected. Should propagate errors via `?`.

10. **`DiscoveryCache` `RwLock` poisoning**: `discovery.rs:36, 41, 46, 51, 56, 61, 68, 76` — all `.unwrap()` on lock acquisition. If a background task panics while holding the write lock, all subsequent discovery reads/writes panic, cascading to server function failures. Should use `parking_lot::RwLock` (no poisoning) or handle `PoisonError`.

### 6.2 Functional Gaps

11. **OIDC approval has no UI/API**: `approve_external_identity` (`extended_auth_queries.rs:74-83`) is defined but never invoked by any `#[server]` function or Axum handler. New OIDC users are stuck "pending approval" forever with no admin path to approve them. Dead code / unfinished feature.

12. **`create_application_query` ignores `is_pinned` input**: `content_write_queries.rs:362` hardcodes `1i64`. Applications are always created pinned. The `ApplicationInput.is_pinned` field is dead.

13. **`reorder_*` functions ignore category scoping**: `reorder_services_query`/`reorder_applications_query`/`reorder_bookmarks_query` all take a category parameter but don't use it (`_category`). Reordering is global. If the UI sends per-category reorder lists, services from other categories could have their `order_index` overwritten to conflicting values.

14. **`DashboardView.settings` is always default**: `content_queries.rs:203` returns `DashboardSettings::default()` — the real dashboard section enable/column settings (from `setting` table via `get_dashboard_settings`) are not wired into `list_dashboard_query`. The dashboard renders based on `data.settings` which is always defaults. The `get_dashboard_settings` server function exists but `HomePage` (`lib.rs:154-173`) doesn't call it — it only calls `list_dashboard`. **Dashboard section enable/column settings have no effect on the rendered dashboard.**

15. **Backup task untested**: `db/backup.rs` `spawn_backup_task`/`create_backup`/`prune_backups` — zero tests. Default disabled, but if enabled, file-copy-based backup has no verification of consistency (could copy during a write).

### 6.3 Inconsistencies

16. **`crates/AGENTS.md:60` stale TODO**: claims "fail-closed auth with TODO for Phase 5 session wiring" — Phase 5 is complete, session wiring is fully implemented. DOX violation (stale text should be removed per root AGENTS.md:65).

17. **`crates/AGENTS.md:37` vs actual**: claims "Security-critical code (auth, CSRF, sessions) lives in `server/`, never in `app/`" — but `auth_helper.rs` (CSRF validation), `auth_queries.rs` (session management, password hashing), `extended_auth_queries.rs` (API token hashing), and `extended_auth.rs` (WebAuthn challenge generation) all live in `app/`. The `server/` crate only has `auth/oidc.rs` (OIDC HTTP handlers) and `auth/api_token.rs` (bearer extraction). Contract drift.

18. **`docker-compose.yml` says "Placeholder — finalized in Phase 12"** but Phase 12 is complete per SPEC_STATE.md and CHANGELOG.md. Stale comment.

19. **`build-multiarch.sh` hardcodes `IMAGE="${REGISTRY}/draco-lunaris/emberwake"`** — may not match `github.repository_owner`. Release workflow uses `${{ github.repository_owner }}`. Inconsistency between manual build script and CI.

20. **`AuditWriter` in `AppState` appears unused by server functions**: server functions call `app::server::auth_queries::audit_write_query` directly. The `Arc<AuditWriter>` in `AppState` (`state.rs:19`) may be vestigial or used only by Axum handlers (none observed in `public_api.rs` or `oidc.rs` — they don't call `state.audit.write()`).

21. **`hydrate()` defined twice**: `app/src/lib.rs:233` and `frontend/src/lib.rs:8`. Both compile under different features. The frontend one is the canonical entry per DOX; the app one is redundant.

22. **`import_governor()` defined but never wired**: `security/rate_limit.rs:43-50` defines a 5/min import governor, but `main.rs` only applies `token_governor()` to public_api and `login_governor()` to oidc. Import goes through server functions (Leptos routes), which only get `default_governor` (60/min). Import rate limiting is not enforced as documented in `crates/AGENTS.md:43` (mentions "5 requests per minute per IP" for import).

---

## 7. Cross-Crate Dependency Summary

| Crate | Depends on (within workspace) | External highlights |
|---|---|---|
| `app` | (none within workspace) | leptos 0.8.19 (nonce), leptos_router, leptos_meta, sqlx (optional/ssr), argon2 (ssr), hmac/sha2/base64 (ssr), webauthn-rs (ssr), scraper/quick-xml (ssr), tokio (ssr), web-sys (wasm32 only), wasm-bindgen (hydrate), garde |
| `frontend` | `app` (hydrate feature) | leptos, wasm-bindgen |
| `server` | `app` (ssr feature) | leptos, leptos_axum, axum, tower, tower-http, tower_governor, governor, sqlx, figment, argon2, prometheus, openidconnect (rustls-tls), webauthn-rs/core, hmac/sha2/base64/subtle, reqwest (rustls), futures-util, tokio-stream (sync), bollard (pipe/http), kube (client/runtime/rustls-tls), k8s-openapi (latest), opentelemetry/otlp |
| `fuzz` | `app` (ssr feature) | libfuzzer-sys (nightly) |
| `e2e` | (none) | fantoccini 0.21, tokio, serde_json (standalone, not workspace member) |

**Workspace deps pinned in root `Cargo.toml:12-54`** — 43 entries. Notable: `leptos = 0.8.19`, `leptos_axum = 0.8.9`, `sqlx = 0.9.0`, `axum = 0.8.9`, `tokio = 1.52.3`, `uuid = 1.23.3` (v7+js), `bollard = 0.18`, `kube = 0.99`, `k8s-openapi = 0.24`, `opentelemetry = 0.27`. `Cargo.lock` has 583 dependency entries.

---

## 8. Summary

Emberwake is a mature, well-structured Leptos 0.8 SSR+hydrate full-stack Rust application with comprehensive test coverage (77 server functions, ~90+ test functions across foundational/server_fn/integration layers, 3 fuzz targets, 7 E2E scenarios). The architecture cleanly separates concerns via the three-crate cargo-leptos workspace pattern and uses a shared-SQL-in-`app` technique to avoid circular dependencies.

**Strengths**:
- Zero `unsafe` in application code (Constitution Principle I satisfied).
- Zero TODO/FIXME markers in application code.
- Strong test coverage for the happy paths and key security invariants (CSRF, visibility filtering, secret redaction, encryption-at-rest round-trip, session rotation, API token scoping).
- Read-only-by-construction discovery integrations (no mutating API calls possible).
- Bounded import parsers with size+depth limits and fuzz coverage.
- Comprehensive CI pipeline encoding constitution hard gates (clippy zero-warning, cargo-deny/audit, fuzz smoke, both-arch matrix).
- Signed release images with SBOM.

**Top risks to address**:
1. CSRF comparison not constant-time (use `subtle`).
2. Origin validation substring match + `host.is_empty()` bypass.
3. Import preview token unauthenticated (HMAC-sign it).
4. WebAuthn RP ID hardcoded to localhost (unusable in production).
5. OIDC approval workflow has no admin UI/API (dead code).
6. Dashboard section settings have no effect (`DashboardView.settings` always default).
7. `create_application_query` ignores `is_pinned` input.
8. `reorder_*` functions ignore category scoping (global reorder).
9. `DiscoveryCache` RwLock poisoning panics.
10. Stale DOX claims in `crates/AGENTS.md:60` (Phase 5 TODO) and contract drift on `crates/AGENTS.md:37` (auth location).

**Research only — no files were edited.**
