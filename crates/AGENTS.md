# DOX: crates

## Purpose

Cargo workspace for the Emberwake application: a Leptos 0.8.x SSR+hydrate full-stack
Rust web app on Axum. Three crates form the canonical cargo-leptos workspace shape.

## Ownership

- `app/` — Leptos UI components + server functions + shared domain types. Compiled
  for both `ssr` and `hydrate` features. Crate-type: `cdylib` + `rlib`.
- `frontend/` — Thin WASM hydrate entry point. Crate-type: `cdylib`. Depends on
  `app` with `hydrate` feature. Exports `#[wasm_bindgen] pub fn hydrate()` which the
  Leptos hydration script calls as `mod.hydrate()` after WASM initialization.
- `server/` — Axum binary (`emberwake`) + library (`server`). Depends on `app`
  with `ssr` feature. Owns all non-UI concerns (db, auth, security middleware,
  integrations, telemetry). Library target enables integration tests.

## Local Contracts

- Workspace deps are pinned in root `Cargo.toml` `[workspace.dependencies]`.
- Feature flags: `ssr` (server-side rendering) and `hydrate` (WASM client hydration).
- `leptos` 0.8.x with `nonce` feature enabled (CSP nonce support).
- `leptos_meta` 0.8.x for `<Meta>` tags (CSP header via per-response nonce).
- `uuid` v7 with `js` feature for WASM randomness.
- `getrandom` 0.3 with `wasm_js` feature + `.cargo/config.toml` cfg flag for WASM.
- `sqlx` 0.9 with `sqlite`, `runtime-tokio`, `migrate` features.
- `figment` 0.10 for TOML+env config with `*_FILE` secret resolution.
- `tower_governor` 0.5 + `governor` 0.8 for per-route rate limiting.
- `tower-http` 0.6 for security header layers (HSTS, nosniff, frame-deny, referrer).
- `garde` 0.20 for input validation on domain types.
- `argon2` 0.5 for password hashing (default: m=32 MiB, t=3, p=1).
- `prometheus` 0.13 for metrics endpoint.
- cargo-leptos config lives in root `Cargo.toml` `[[workspace.metadata.leptos]]`.
- Binary name: `emberwake` (set via `[[bin]]` in server and `bin-exe-name` in metadata).
- No `unsafe` in application code (Constitution Principle I).
- Security-critical code (auth, CSRF, sessions) lives in `server/`, never in `app/`.
- `app` crate: `sqlx` is optional (behind `ssr` feature) to keep it out of WASM.
- `server` crate: library + binary targets. `lib.rs` re-exports modules for tests.

## Work Guidance

- New shared domain types go in `app/src/domain/`.
- New UI components go in `app/src/components/`.
- New server functions go in `app/src/server/`.
- New routes go in `app/src/routes/`.
- Server-side infrastructure (db, auth, middleware) goes in `server/src/`.
- Server modules: `db/` (pool, repository, backup), `security/` (headers, rate_limit),
  `auth/` (oidc, webauthn, api_token), `public_api.rs` (REST /api/v1/*), `config.rs`,
  `telemetry.rs`, `audit.rs`, `state.rs`.
- `AppState` in `state.rs` holds `LeptosOptions`, `SqlitePool`, `Config`, `AuditWriter`.
- `FromRef<AppState>` for `LeptosOptions` enables leptos_axum router integration.
- `FromRef<AppState>` for `SqlitePool` enables server-function pool extraction.
- Server functions extract `SqlitePool` via `leptos_axum::extract::<Extension<SqlitePool>>()`;
  the pool is wired through `axum::Extension(pool.clone())` in `main.rs`.
- `app/src/server/content_queries.rs` holds shared SQL read functions (ssr-only) used by both
  server functions (`app`) and repository methods (`server`); this avoids a circular dependency.
- `app/src/server/content_write_queries.rs` holds shared SQL write functions (ssr-only) for
  create/update/delete/reorder/pin operations; same circular-dependency avoidance pattern.
- `app/src/server/content_write.rs` holds mutating `#[server]` functions (auth+CSRF+authz enforced,
  audited, fail-closed auth with TODO for Phase 5 session wiring).
- `app/src/server/extended_auth_queries.rs` holds shared SQL functions for OIDC external identities,
  WebAuthn passkeys, and scoped API tokens (ssr-only, parameterized SQL, HMAC-SHA256 token hashing).
- `app/src/server/extended_auth.rs` holds `#[server]` functions for extended auth (OIDC begin, passkey
  register/login, API token CRUD). Uses `ServerKey`, `WebAuthnRpInfo`, `ChallengeStore` Axum Extensions.
- `app/src/server/settings_queries.rs` holds shared SQL functions for settings and theme CRUD (ssr-only,
  parameterized SQL, static string literals). Secret-bearing settings (`weather`, `auth` keys) are encrypted
  at rest using an XOR keystream derived from HMAC-SHA256, keyed from the server secret. Secrets are never
  returned to non-admins or logged. Includes built-in theme seeding (Light + Dark).
- `app/src/server/settings.rs` holds `#[server]` functions for settings and themes: `get_settings` (admin:
  secrets redacted for others), `update_settings` (admin-gated, audited), `list_themes` (public),
  `get_active_theme` (public, applied during SSR), `save_theme` (admin-gated, audited), `set_active_theme`
  (admin-gated, audited). All mutations enforce auth + CSRF + admin authorization.
- `app/src/components/settings/` holds the settings page and theme builder UI components (admin-gated).
- `app/src/server/monitor_queries.rs` holds shared SQL functions for service monitoring (ssr-only, parameterized
  SQL, static string literals): `list_monitored_services`, `get_status_reading`, `upsert_status_reading`,
  `insert_status_history`, `prune_status_history`, `list_status_readings` (visibility-filtered),
  `compute_uptime_summary`. Constants: `DEFAULT_MAX_ROWS` (1000), `DEFAULT_MAX_AGE_DAYS` (30).
- `app/src/server/monitor_read.rs` holds `#[server]` functions for monitoring reads: `get_service_statuses`
  (public for public services, all for authenticated), `get_uptime_summary` (public for public services).
  Uses `visibility_for_caller` pattern same as content_read.rs.
- `app/src/components/dashboard/status_tile.rs` holds the `StatusTile` component — a live status tile with
  up/down/degraded indicator + latency. On hydrate, uses `EventSource` on `/events` to update in real-time
  without page refresh. Uses `wasm-bindgen` (hydrate feature) for JS interop.
- `server/src/sse/` holds the SSE hub (`mod.rs`) and `/events` handler (`handler.rs`).
  - `SseHub` wraps a `tokio::sync::broadcast` channel; stored in `AppState` as `Arc<SseHub>`.
  - `SseEvent` enum: `Status(SseStatusEvent)` and `Weather(serde_json::Value)` (weather for US7).
  - `/events` endpoint: `text/event-stream`; public stream carries only public-service status + weather;
    authenticated session upgrades to include private-service status. Uses `BroadcastStream` from
    `tokio-stream` (sync feature). Keep-alive every 15s.
- `server/src/integrations/weather.rs` holds the weather API client + scheduled refresh task (US7).
  - `fetch_weather`: GET via reqwest (rustls, already default TLS); parses WeatherAPI-style JSON (current.temp_c, condition.text, is_day, cloud, last_updated).
  - `refresh_weather`: reads weather settings from DB, config-gated (inert when unset/disabled), fetches upstream, caches to `weather_reading` table (single-row upsert), emits `SseEvent::Weather` on success. On upstream error: retains last good cache, logs warning, does NOT emit event.
  - `spawn_scheduler`: background task ticking every 60s; if weather is configured, runs `refresh_weather` and resets interval to configured value (default 600s, min 60s). If not configured: no outbound calls.
- `server/src/monitor/` holds the health-check engine (`mod.rs`) and scheduler (`scheduler.rs`).
  - `http_check`: GET via reqwest (rustls); 2xx=up, 3xx/4xx=degraded, timeout/error=down. Records latency_ms.
  - `tcp_check`: `tokio::net::TcpStream::connect` with timeout; up if connect succeeds, down if refused/timeout.
  - `check_service`: runs a check, upserts StatusReading, inserts StatusHistory row, prunes history,
    emits SSE event on state change (first check always emits).
  - `spawn_scheduler`: background task scanning every 30s for `monitor_enabled=true` services,
    spawning concurrent `tokio::spawn` per service. Disabled services make NO outbound calls.
- Monitor domain types in `app/src/domain/mod.rs`: `MonitorState` (Up/Down/Degraded), `StatusReading`,
  `StatusHistory`, `UptimeSummary`, `SseStatusEvent`.
- Workspace deps added: `futures-util` 0.3, `tokio-stream` 0.1 (sync feature). `wasm-bindgen` 0.2 added
  to app crate as optional dep behind `hydrate` feature.
- `AppState` now holds `sse_hub: Arc<SseHub>`. `main.rs` creates the hub, merges `sse::handler::sse_routes()`,
  and spawns `monitor::scheduler::spawn_scheduler()`.
- Test targets: `tests/integration/monitor.rs` (T056/T056b: 7 tests — check records state/latency, history
  row written, state transition emits SSE event, disabled services not listed, uptime summary computes
  percentage, retention pruning by age and by max-rows, visibility filter), `tests/integration/sse.rs`
  (T057: 2 tests — client receives status event on up→down flip, no event on same state).
- `app/src/lib.rs` injects the active theme as CSS custom properties in the document head during SSR
  (no flash of default theme). Falls back to `prefers-color-scheme` when no active theme is set. Custom CSS
  is served with CSP nonce (`nonce=true` on `<style>` tags).
- `app/src/lib.rs` calls `provide_meta_context()` and includes `<HashedStylesheet>`, `<HydrationScripts>`,
  and `<AutoReload>` in the `<head>` for WASM hydration and CSS asset loading. `LeptosOptions` is obtained
  via `expect_context`.
- `style/main.css` is the component CSS file, bundled by cargo-leptos via `style-file` in workspace metadata.
  Uses theme tokens (CSS custom properties: `--bg`, `--surface`, `--text`, `--text-muted`, `--accent`,
  `--accent-text`, `--border`, `--radius`, `--spacing`, `--font`). Dark/light mode works via `prefers-color-scheme`.
- `server/src/main.rs` `LeptosOptions::builder()` sets `site_root` from `LEPTOS_SITE_ROOT` env var (defaults
  to `target/site`) and `hash_files(true)` to match cargo-leptos `hash-files = true` config.
- `server/src/main.rs` `shell()` function (for error/404 pages) includes `<HashedStylesheet>`, `<AutoReload>`,
  and `<HydrationScripts>` for consistent asset loading on error pages.
- `.docker/Dockerfile` copies `hash.txt` to `/usr/local/bin/hash.txt` (needed by `HashedStylesheet`/
  `HydrationScripts` to resolve hashed asset names) and sets `LEPTOS_SITE_ROOT=/var/lib/emberwake/site`.
- `server/src/main.rs` seeds built-in themes (Light + Dark) on startup if none exist.
- Settings domain types: `DesignTokens`, `Theme`, `ThemeSummary`, `ThemeInput`, `SettingsView`,
  `SettingsPatch`, `IntegrationSettings`, `WeatherSettings`, `AuthSettings` in `app/src/domain/mod.rs`.
- Test targets: `tests/server_fn/settings.rs` (T050: settings + theme CRUD, secret redaction, encryption
  at rest), `tests/integration/ssr_theme.rs` (T051: SSR theme no-flash, prefers-color-scheme fallback).
- `server/src/auth/oidc.rs` implements OIDC auth code + PKCE flow with admin-approve provisioning.
- `server/src/auth/api_token.rs` implements bearer token verification + scope checking for `/api/v1/*`.
- `server/src/public_api.rs` implements REST handlers for `/api/v1/*` (services, bookmarks, export).
- Config: `server_key` (HMAC token hashing) and `oidc` section (enabled, issuer_url, client_id,
  client_secret, redirect_url) are optional with `#[serde(default)]`.
- `app/src/components/editors/` holds editor components with drag-and-drop reorder + optimistic UI.
- SQL queries use static string literals per `VisibilityFilter` branch (sqlx 0.9 `SqlSafeStr`).
- Write queries use parameterized SQL with static string literals (sqlx 0.9 `SqlSafeStr`).
- Private rows are excluded in SQL via `VisibilityFilter::PublicOnly`/`All`, not in the UI.
- `AppError` implements `Display` + `FromStr` for `ServerFnError<AppError>` custom error transport.
- `app` crate depends on `axum` and `leptos_axum` behind the `ssr` feature.
- `app` crate depends on `web-sys` for WASM browser APIs (search island navigation).
- Fuzzy search is client-side only (`components/search/fuzzy.rs`), no external crate dependency.
- Search provider config is read from the `setting` table key `search.providers` (JSON).
- Dashboard SSR uses `Resource` + `Suspense` to render pinned content on first paint.
- `server/src/integrations/docker.rs` holds the Docker discovery integration (US8).
  - Uses `bollard` crate for Docker API access (list_containers, inspect_container, events stream).
  - `connect`: checks if Docker socket exists before connecting; returns None if absent (graceful).
  - `list_containers`: lists all containers with labels → parses via shared `labels::parse_labels`.
  - `watch_events`: subscribes to Docker events stream; on container start → inspect + parse labels +
    update cache + emit SSE discovery event; on container stop/die → remove from cache + emit removal SSE.
  - `spawn_scheduler`: background task ticking every 60s; reads integration settings; if disabled = no
    connection, no calls; if enabled → list containers, populate cache, watch events for live deltas.
  - STRICTLY read-only: only list_containers, inspect_container, events — no create/delete/start/stop/exec.
- `server/src/integrations/kubernetes.rs` holds the K8s discovery integration (US8).
  - Uses `kube` + `k8s-openapi` crates for Kubernetes API access (list Ingress, watch Ingress).
  - `list_ingresses`: lists all Ingress resources with annotations → parses via shared `labels::parse_labels`.
  - `watch_ingresses`: watches Ingress changes; on apply → parse annotations + update cache + emit SSE;
    on delete → remove from cache + emit removal SSE.
  - `spawn_scheduler`: background task ticking every 60s; reads integration settings; if disabled = no
    connection, no calls; if enabled → list ingresses, populate cache, watch for live deltas.
  - STRICTLY read-only: only list and watch — no create/update/delete.
- `server/src/integrations/labels.rs` holds the shared label/annotation parser (US8).
  - `parse_labels`: pure function — parses `emberwake.*` labels/annotations into `DiscoveredService`.
  - Label keys: `emberwake.name` (required), `emberwake.url` (required, comma-separated for multi-value),
    `emberwake.icon`, `emberwake.category`, `emberwake.description` (all optional).
  - Missing name or URL = service not discovered. Multi-value URL creates one service per URL.
  - `has_emberwake_labels`: checks if any `emberwake.*` labels are present.
- `app/src/server/discovery.rs` holds the DiscoveryCache + server functions (US8).
  - `DiscoveryCache`: thread-safe cache (`Arc<RwLock<Vec<DiscoveredService>>>`) for Docker and K8s
    discovered services. Shared via Axum Extension between background tasks (populate) and server
    functions (read). Cloneable (inner is Arc).
  - `discover_docker() -> Vec<DiscoveredService>`: admin-gated server function; returns empty vec when
    `integrations.docker_enabled` is false (no calls made); reads from cache when enabled.
  - `discover_kubernetes() -> Vec<DiscoveredService>`: admin-gated server function; returns empty vec
    when `integrations.k8s_enabled` is false (no calls made); reads from cache when enabled.
- Discovery domain types in `app/src/domain/mod.rs`: `DiscoverySource` (Docker/Kubernetes),
  `DiscoveredService` (name, url, icon, category, description, source, source_id),
  `DiscoveryAction` (Added/Removed), `SseDiscoveryEvent` (service_id, action, name, url).
- `SseEvent` enum extended with `Discovery(SseDiscoveryEvent)` variant. `SseHub::broadcast_discovery`
  emits discovery events. SSE handler sends discovery events to authenticated sessions only (admin-gated
  via server function; SSE stream gates on session existence).
- `IntegrationSettings` extended with `docker_socket: Option<String>` for configurable Docker socket path.
- `get_integrations_typed` made `pub` in `settings_queries.rs` (was private) for use by discovery modules.
- Workspace deps added: `bollard` 0.18 (pipe, http features), `kube` 0.99 (client, runtime, rustls-tls
  features), `k8s-openapi` 0.24 (latest feature). All in server crate only (behind ssr via server dep).
- `main.rs` creates `DiscoveryCache`, spawns `docker::spawn_scheduler` and `kubernetes::spawn_scheduler`,
  and wires `DiscoveryCache` as Axum Extension.
- Test targets: `tests/integration/discovery.rs` (T064/T065: 12 tests — Docker label parsing, K8s
  annotation parsing, multi-value URL syntax, missing name/URL handling, empty values, no emberwake
  labels detection, container-start SSE event emission, container-stop removal SSE event, disabled
  returns empty cache, read-only by construction verification).
- Existing test `tests/integration/sse.rs` updated to handle `SseEvent::Discovery` variant in match arms.
- Existing test `tests/server_fn/settings.rs` updated for new `docker_socket` field in `IntegrationSettings`.
- Changes to workspace deps require updating root `Cargo.toml` and `Cargo.lock`.
- `app/src/server/importer/` holds the bounded import parsers (US9): `mod.rs` (dispatch + size/depth limits),
  `json.rs` (ExportDocument JSON parser with depth check), `html.rs` (Netscape bookmark format via `scraper`),
  `opml.rs` (OPML XML via `quick-xml` streaming). All parsers are sync, never panic, enforce MAX_IMPORT_SIZE
  (10 MB) and MAX_DERIVATION_DEPTH (100). Server functions wrap these in `spawn_blocking`.
- `app/src/server/export_queries.rs` holds the JSON exporter (US9): `export_data_query` reads all entity types
  and converts to export DTOs. Excludes: password hashes, session data, API token hashes, secret settings
  (weather key, OIDC client secret). ExportScope: Full or Selective(Vec<ExportEntity>).
- `app/src/server/import_export.rs` holds `#[server]` functions for import/export (US9): `export_data`
  (admin-gated), `import_preview` (admin-gated, parse under spawn_blocking, NO writes, returns ImportPreviewData
  with base64-encoded ParsedData token), `import_apply` (admin-gated, transactional all-or-nothing, regenerates
  UUIDs, duplicate handling via DuplicateStrategy: Skip/Overwrite/Rename, audited).
- Import domain types in `app/src/domain/mod.rs`: ExportScope, ExportEntity, ExportDocument, ExportCategory,
  ExportService, ExportBookmark, ExportTheme, ImportKind (Json/HtmlBookmarks/Opml), DuplicateStrategy,
  ImportOptions, ImportResult, ParsedData, ParsedCategory, ParsedBookmark, ParsedService, ParsedTheme,
  ImportPreviewData.
- Workspace deps added: `scraper` 0.22 (HTML parsing, behind ssr in app crate), `quick-xml` 0.37 (OPML XML
  parsing, behind ssr in app crate), `tokio` added to app crate behind ssr for `spawn_blocking`.
- Fuzz targets in `fuzz/fuzz_targets/{import_html,import_json,import_opml}.rs` call the sync parser functions
  with arbitrary bytes and assert no panic/OOM. `fuzz/Cargo.toml` depends on `app` with ssr feature.
- Test target: `tests/integration/export_import.rs` (T069–T071: 11 tests — export→import round-trip
  equivalence, HTML import parses categories/bookmarks, OPML import parses bookmarks, duplicate handling
  skip/overwrite, reject oversized input, reject malformed/truncated JSON, reject deeply nested HTML,
  no partial writes on rejection, no partial writes on oversized).

## Verification

- `cargo build` — workspace compiles
- `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
- `cargo fmt --all --check` — formatting clean
- `cargo leptos build` — both WASM frontend and server binary build
- `cargo test` — foundational tests (migrations, repository, constraints)
- `e2e/` is NOT a workspace member — fantoccini pulls native-tls/openssl which would
  violate `deny.toml`. E2E tests run standalone: `cargo test -p e2e --manifest-path e2e/Cargo.toml`
- `deny.toml` bans `native-tls` only (not `openssl`/`openssl-sys` — webauthn-rs uses
  OpenSSL for X.509 attestation cert parsing, not TLS; all transport uses rustls)

## Child DOX Index

None — `app/`, `frontend/`, and `server/` are individual crate packages, not
durable boundaries with their own contracts.
