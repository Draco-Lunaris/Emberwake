# Emberwake Spec Gap Analysis Report

**Date**: 2026-07-05 · **Scope**: Full spec/codebase cross-reference · **Mode**: Research only (no edits)

---

## Executive Summary

The SPEC_STATE.md tracker is **substantially stale and inaccurate**. Phases 1–12 are genuinely complete and well-tested (107 tests), matching the tracker. However, **Phase 13 (T085–T093) is marked "Not started" while in reality 7 of its 9 tasks are largely implemented** — the Application entity (migration, domain types, repo methods, server functions, editor UI, dashboard tile), per-section dashboard settings (Setting keys, server functions, Settings UI), three-section dashboard component, CSS, and the bookmark-requires-category validation are all present in the code. The real critical gap is narrower and more severe than SPEC_STATE suggests: the dashboard component reads `DashboardSettings::default()` instead of persisted settings, so BV-004/BV-005/BV-014/BV-015 **silently no-op** (section enable/disable and column counts are saved but never applied); uncategorized bookmarks remain invisible (BV-003/BV-007 still genuinely unfixed despite the create-time category requirement); and SC-001/002/003/008 are "measurable" but never actually measured. None of these are reflected accurately in SPEC_STATE. I recommend updating SPEC_STATE immediately and treating the dashboard-settings-wiring gap as the single highest-priority fix.

---

## Per-User-Story Gap Table

| Story | Spec Status | SPEC_STATE Accuracy | Code Reality | Evidence | Severity of Drift |
|---|---|---|---|---|---|
| **US1 Dashboard** | Partial | Partly accurate | Three-section layout + per-section settings UI exist, but `list_dashboard_query` returns `DashboardSettings::default()` — settings never flow into the dashboard. BV-001 (three sections) rendered; BV-003 (uncategorized bookmarks) still broken; BV-004/BV-005 (section toggle/columns) saved but not applied. | `crates/app/src/server/content_queries.rs:203`; `crates/app/src/lib.rs:157`; `crates/app/src/components/dashboard/mod.rs:24-73` | **Critical** |
| **US2 CRUD** | Complete | Accurate | All CRUD + reorder + pin for category/service/bookmark/application with auth+CSRF. Icon upload persists to `data/icons` (hardcoded path, not `DATA_DIR`-relative — minor). | `crates/app/src/server/content_write.rs:1-652`; `crates/app/src/server/content_write.rs:633-636` | Low |
| **US3 Auth** | Complete | Accurate | Argon2id, server-side sessions, CSRF (token + origin), setup race-safety via `setup_complete` UNIQUE, login throttling, audit. 13 auth tests. | `crates/app/src/server/auth_queries.rs:80,130`; `crates/server/tests/server_fn/auth.rs`; `csrf.rs` | None |
| **US4 Extended auth** | Complete | Accurate (deferred items honestly listed) | OIDC (auth-code+PKCE, admin-approve), WebAuthn passkeys, scoped API tokens all implemented. Full stub-IdP roundtrip and virtual-authenticator WebAuthn genuinely deferred (test files document why). | `crates/app/src/server/extended_auth.rs:57-543`; `crates/server/tests/integration/oidc.rs:374-384`; `webauthn.rs:443-456` | None |
| **US5 Theming** | Complete | Accurate | Theme tokens + custom CSS + SSR injection (no flash), built-in Light/Dark, secret encryption at rest, `prefers-color-scheme` fallback. Dashboard settings UI (T090/T091) exists but does not feed back to dashboard (cross-listed under US1). | `crates/app/src/lib.rs:31-77`; `crates/app/src/server/settings.rs:206-308` | Medium (cross-ref) |
| **US6 Monitoring** | Complete | Accurate | HTTP/TCP checks, SSE push, StatusHistory + uptime summary, retention pruning (max-rows 1000 / max-age 30d), disabled services make no calls. 7 monitor + 2 SSE tests. | `crates/server/src/monitor/`; `crates/app/src/server/monitor_queries.rs:142,185` | None |
| **US7 Weather** | Complete | Accurate | WeatherAPI client (rustls), scheduled refresh, single-row cache, SSE weather events, config-gated (inert when unset). | `crates/server/src/integrations/weather.rs:166` | None |
| **US8 Discovery** | Complete | Accurate | Docker (bollard: list/inspect/events) + K8s (kube: list/watch), label parser, SSE discovery events, disabled = no calls. 12 discovery tests. | `crates/server/src/integrations/docker.rs:123`; `kubernetes.rs:115` | None |
| **US9 Import/Export** | Complete | Accurate | JSON/HTML/OPML parsers (bounded 10MB/depth 100, `spawn_blocking`), export excludes secrets, transactional import with duplicate strategies, 3 fuzz targets, 11 export_import tests. | `crates/app/src/server/importer/`; `fuzz/fuzz_targets/` | None |
| **Phase 13 Redesign** | **Stale — marked "Not started" but ~78% implemented** | **Inaccurate** | T085 ✅, T086 ✅, T087 ✅ (component), T088 ✅, T089 ✅ (create-time), T090 ✅ (Settings UI), T091 ✅ (Settings UI), T092 ✅ (CSS), T093 ❌ (no BV test). **Critical missing piece**: T087 is incompletely wired — `list_dashboard_query` hardcodes `DashboardSettings::default()` instead of reading persisted settings. | See Phase 13 table below | **High** (tracker accuracy) |

---

## Success Criteria Status Table

| SC | Spec Requirement | SPEC_STATE / PERF.md Claim | Actual Reality | Evidence | Status |
|---|---|---|---|---|---|
| **SC-001** | SSR TTFB <50ms, interactive <1s at 200 svc / 500 bm | "Measurable" via `benches/seed_benchmark.sh` | Script exists but **never run**; no recorded numbers. Budget compliance unverified. | `PERFORMANCE.md:101-103`; `benches/seed_benchmark.sh` | **Unverified** (Medium) |
| **SC-002** | CRUD server-fn p95 <25ms | "Measurable" via script | Same — never run. | `PERFORMANCE.md:102` | **Unverified** (Medium) |
| **SC-003** | Idle RSS ≤48MB, cold start <1.5s | "Measurable" via script (`COLD_START=1`) | Same — never run. | `PERFORMANCE.md:104-105` | **Unverified** (Medium) |
| **SC-004** | WASM bundle <350KB gzip | ✅ Verified 89 KB gzip | Verified — evidence present and credible. | `PERFORMANCE.md:63-68` | ✅ **Verified** |
| **SC-005** | 100% mutating server fns enforce auth+CSRF | ✅ Verified by tests | Accurate — `require_auth_csrf`/`require_admin_csrf` on all 14 content mutations + settings + extended_auth + import_export. CSRF test file covers token + origin. | `crates/app/src/server/auth_helper.rs:85,118`; `SECURITY_VERIFICATION.md:14-29`; `crates/server/tests/server_fn/csrf.rs` | ✅ **Verified** |
| **SC-006** | cargo-deny + cargo-audit zero advisories; signed image + SBOM | CI-validated | CI workflows wire cargo-deny + cargo-audit + cosign + cyclonedx. Not run in this environment but gates are correctly encoded. | `.github/workflows/ci.yml:66-73`; `release.yml:36-87` | ✅ **Wired (CI-validated)** |
| **SC-007** | Fuzz parsers survive, reject malformed, no partial writes | ✅ Verified by tests + fuzz targets | 3 fuzz targets exist; 11 export_import tests cover reject-oversized/malformed/no-partial-write. CI runs fuzz smoke (`-max_total_time=60`). | `fuzz/fuzz_targets/{import_html,import_json,import_opml}.rs`; `.github/workflows/ci.yml:75-95`; `crates/server/tests/integration/export_import.rs` | ✅ **Verified** |
| **SC-008** | Image runs identically on amd64 + arm64 from same tag, verified in CI each release | CI matrix amd64+arm64 | CI matrix is amd64+arm64 for lint/test/fuzz; release builds both via buildx. **No runtime parity test** (only build+clippy+test parity). | `.github/workflows/ci.yml:38-42`; `release.yml:54-65` | **Partial** — build parity ✅, runtime parity not asserted (Low-Medium) |

---

## Phase 13 Task Detail (T085–T093)

| Task | SPEC_STATE | Reality | Evidence |
|---|---|---|---|
| T085 Application entity | ❌ Not started | ✅ **Complete** | `migrations/000005_add_applications.sql`; `crates/app/src/domain/mod.rs:85,241,260`; `content_write_queries.rs:337,382,448,461,478`; `content_queries.rs:325`; `repository.rs:42,102,205,279`; `tests/server_fn/application.rs` (8 tests) |
| T086 Per-section column settings keys | ❌ Not started | ✅ **Complete** | `crates/app/src/server/settings.rs:212-294` (6 keys: services/applications/bookmarks × enabled/columns); `tests/server_fn/dashboard_settings.rs` (3 tests) |
| T087 Dashboard renders three sections | ❌ Not started | ⚠️ **Partial — component done, settings NOT wired** | `crates/app/src/components/dashboard/mod.rs:13-76` renders 3 sections reading `data.settings`, but `content_queries.rs:203` returns `DashboardSettings::default()` — persisted settings never reach the component |
| T088 ApplicationEditor | ❌ Not started | ✅ **Complete** | `crates/app/src/components/editors/mod.rs:468-725` (`ApplicationEditor` component + `ApplicationEditPage`) |
| T089 BookmarkEditor requires category | ❌ Not started | ✅ **Complete (server-side)** | `crates/app/src/server/content_write.rs:476-480` rejects `Uuid::nil()` category with "category_id is required". Note: schema still allows NULL `category_id` (`migrations/000001_initial.sql:97`) and `update_bookmark` does NOT enforce the same check — partial. |
| T090 Section enable/disable toggle in Settings | ❌ Not started | ✅ **Complete** | `crates/app/src/components/settings/settings_page.rs:335-422` (3 enable checkboxes + save) |
| T091 Per-section column selector in Settings | ❌ Not started | ✅ **Complete** | `crates/app/src/components/settings/settings_page.rs:370,387,404` (3 column number inputs) |
| T092 CSS for three-section + per-section grid columns | ❌ Not started | ✅ **Complete** | `style/main.css:1399-1438` (`.applications-section` grid `repeat(var(--section-columns, 4), 1fr)`, `.pinned-categories .category` `column-count: var(--section-columns, 3)`) |
| T093 Browser-verify BV-001..BV-015 | ❌ Not started | ❌ Not started | No browser-verification record exists (consistent with deferred e2e). |

---

## Spec Drift Items (Spec vs Reality Mismatches)

| # | Spec Reference | Expected State | Actual State | Evidence | Severity |
|---|---|---|---|---|---|
| D-1 | `spec.md` FR-001, BV-004, BV-005, BV-014, BV-015; `tasks.md` T087/T090/T091 | Per-section enable/disable + column counts applied to dashboard | Settings UI saves values; `get_dashboard_settings`/`update_dashboard_settings` server fns work; **but `list_dashboard_query` hardcodes `DashboardSettings::default()`** so the Dashboard component always renders with `services_enabled=true, applications_enabled=true, bookmarks_enabled=true, *_columns=4`. Operator changes silently no-op. | `crates/app/src/server/content_queries.rs:199-204` | **Critical** |
| D-2 | `spec.md` FR-021, Key Entities Application; `data-model.md` Application table | Application entity with `category_id` defaulting to 'Uncategorized' | Migration + entity + CRUD exist, but there is **no 'Uncategorized' default category seeded**, and `category_id` is nullable with no default. Spec says "defaults to 'Uncategorized'"; code defaults to NULL. | `migrations/000005_add_applications.sql:4` (nullable, no default); no seed of 'Uncategorized' category found | Medium |
| D-3 | `spec.md` Key Entities Bookmark: `category_id` REQUIRED; `data-model.md` Bookmark: "REQUIRED" | Bookmark.category_id NOT NULL | Schema column is `TEXT NULL` (`migrations/000001_initial.sql:97`, reaffirmed in `000004:50`). Create-time validation enforces non-nil, but **update_bookmark does not enforce it** and pre-existing NULL rows remain possible. | `migrations/000001_initial.sql:97`; `content_write.rs:476-480` (create only); `content_write_queries.rs:579` (update allows None) | Medium |
| D-4 | `spec.md` BV-003, BV-007 | Bookmarks without category appear in default 'Uncategorized' section | `list_dashboard_query` iterates categories and fetches bookmarks per `WHERE category_id = ?` — **uncategorized bookmarks (NULL category_id) are never fetched**, so they are invisible on the dashboard. Bug #1/#2 in SPEC_STATE still genuinely unfixed. | `crates/app/src/server/content_queries.rs:146-175` | **High** |
| D-5 | `spec.md` FR-019 (SHOULD): optional built-in HTTPS via rustls + ACME | ACME HTTPS support | **Not implemented.** `main.rs` has no acme/tls code; `research.md` Decision 5 lists it as optional but `quickstart.md` documents it as a feature. No `rustls-acme` dep in workspace. | grep `acme|ACME` in `crates/server/src/main.rs` → 0 matches | Low (SHOULD, not MUST) |
| D-6 | `spec.md` FR-020 | WAL checkpoint + automated `.backup` with retention | Both present (`crates/server/src/db/backup.rs`). ✅ No drift — listed for completeness. | `backup.rs:13,31` | None |
| D-7 | `spec.md` US1 Acceptance 3, FR-002 | Prefixed query routes to provider URL | Implemented (`components/search/`). ✅ No drift. | `crates/app/src/components/search/` | None |
| D-8 | `data-model.md` Setting keys list | Keys include `dashboard.services.enabled` etc. | All 6 keys present in code. ✅ No drift. | `settings.rs:212-294` | None |
| D-9 | `tasks.md` T084 | Run quickstart walkthrough end-to-end | `QUICKSTART_WALKTHROUGH.md` exists but is a **static document**, not evidence of execution. SPEC_STATE deferred-lists it honestly. | `QUICKSTART_WALKTHROUGH.md` | Low |
| D-10 | `contracts/public-api.yaml` `/api/v1/services` POST | Creates service via scoped token | Implemented in `public_api.rs:73-160`. ✅ No drift. | `crates/server/src/public_api.rs:73` | None |
| D-11 | `crates/AGENTS.md` Local Contract | "Security-critical code (auth, CSRF, sessions) lives in `server/`, never in `app/`" | **Mild drift**: `app/src/server/auth_helper.rs` contains `require_session`/`require_session_csrf`/`require_admin_csrf` and `validate_origin` — security-critical CSRF/session enforcement lives in `app`, not `server`. This is acknowledged in the same AGENTS.md (`content_write.rs` holds auth+CSRF). The contract statement is aspirational vs actual. | `crates/app/src/server/auth_helper.rs:13,85,118` | Low (doc-internal) |

---

## Open Bugs Re-Verification (from SPEC_STATE.md)

| # | Bug | SPEC_STATE Status | Re-Verification | Evidence | Verdict |
|---|---|---|---|---|---|
| 1 | Bookmarks without category invisible on dashboard | ❌ Unfixed | **Still unfixed.** `list_dashboard_query` fetches bookmarks only via per-category loop; NULL `category_id` rows are never selected. | `content_queries.rs:146-175` | ✅ Real bug |
| 2 | New bookmarks don't appear on dashboard (no category) | ❌ Unfixed | **Partially mitigated, still real.** `create_bookmark` now rejects `Uuid::nil()` category (`content_write.rs:476`), so NEW uncategorized bookmarks can no longer be created via the server fn. However, (a) `update_bookmark` can still clear category, (b) NULL category_id rows from imports/older data remain invisible, (c) the UI `<option value="">` still offers "Select category" with empty value which maps to `Uuid::nil()` and would be rejected server-side (poor UX — silent failure). | `content_write.rs:476-480`; `editors/mod.rs:875-876` | ⚠️ **Partially real** — creation path fixed, dashboard rendering still broken |
| 3 | Three-section layout not implemented | ❌ Not started | **STALE — implemented.** `Dashboard` component renders Services/Applications/Bookmarks sections. | `components/dashboard/mod.rs:13-76` | ❌ **Stale bug** — should be closed (with caveat: settings not wired, see D-1) |
| 4 | Per-section column settings not implemented | ❌ Not started | **STALE — implemented (storage + UI), but NOT APPLIED to dashboard.** Settings UI + server fns exist; `list_dashboard_query` ignores them. | `settings.rs:206-308`; `content_queries.rs:203` | ⚠️ **Half-stale** — storage done, application broken (D-1) |
| 5 | Section enable/disable not implemented | ❌ Not started | **STALE — same as #4.** Toggle UI + server fns exist; dashboard ignores. | `settings_page.rs:365-402`; `content_queries.rs:203` | ⚠️ **Half-stale** |
| 6 | Application entity not implemented | ❌ Not started | **STALE — fully implemented.** Migration + entity + CRUD + repository + server fns + editor + dashboard tile + 8 tests. | `migrations/000005`; `domain/mod.rs:85`; `tests/server_fn/application.rs` | ❌ **Stale bug** — should be closed |
| 7 | Docker build may fail with web-sys version issues | ⚠️ Fixed in Cargo.lock | Cannot re-verify without a Docker build, but `Cargo.lock` is committed and `web-sys` is pinned. Plausibly resolved. | `Cargo.lock` present | ✅ Plausibly fixed |

---

## Deferred Items Re-Verification (from SPEC_STATE.md)

| Item | Deferred Reason | Re-Verification | Verdict |
|---|---|---|---|
| Full Leptos SSR pipeline tests (T019, T051) | Requires cargo-leptos build pipeline | **Still deferred.** `ssr_dashboard.rs` explicitly uses a hand-rolled handler (not the Leptos router) and documents why (lines 5-12). `ssr_theme.rs` similar. | ✅ Honestly deferred |
| Full stub IdP OIDC roundtrip (T042) | Requires mock HTTP server | **Still deferred.** `oidc.rs:374-384` documents the deferral and what is tested instead (PKCE gen, state store, admin-approve, HTTP 503/400 routes). | ✅ Honestly deferred |
| Full virtual authenticator WebAuthn (T043) | Requires browser WebAuthn API | **Still deferred.** `webauthn.rs:443-456` documents deferral; tests cover builder, ChallengeStore, query layer, state serialization. | ✅ Honestly deferred |
| Full Docker API mocking (T065) | Requires mock Docker daemon | **Partially deferred.** `discovery.rs` has 12 tests but they test the label parser + cache + SSE, not a live bollard connection (the bollard `connect` returns None when socket absent). No mock daemon. | ✅ Honestly deferred |
| Performance benchmarks SC-001/002/003 (T080) | Requires running server + seeded catalog | **Still deferred.** `benches/seed_benchmark.sh` exists but `PERFORMANCE.md` marks SC-001/002/003 as "Measurable" not "Verified". | ✅ Honestly deferred |
| E2E tests (T082) | Require WebDriver server | **Still deferred.** `e2e/src/lib.rs` has 7 tests all `#[ignore = "requires WebDriver server"]`. | ✅ Honestly deferred |
| Quickstart walkthrough execution (T084) | Requires running server + browser | **Still deferred.** `QUICKSTART_WALKTHROUGH.md` is a static doc. | ✅ Honestly deferred |
| CI/CD pipeline validation (T078/T079) | Requires GitHub Actions run | **Still deferred** (cannot run Actions locally). Workflows are well-formed. | ✅ Honestly deferred |
| Three-section dashboard (Services/Applications/Bookmarks) | "Spec updated, implementation not started" | **STALE — implemented** (component + entity + editor). Should be removed from deferred list. | ❌ **No longer deferred** |
| Application entity + migration | "Spec updated, implementation not started" | **STALE — implemented** (`migrations/000005_add_applications.sql`). Should be removed. | ❌ **No longer deferred** |
| Per-section column settings | "Spec updated, implementation not started" | **STALE (storage) / PARTIALLY REAL (application).** Storage + UI implemented; dashboard application broken (D-1). Should be revised, not removed. | ⚠️ **Half-stale** |
| Section enable/disable | "Spec updated, implementation not started" | **Same as per-section columns** — storage/UI done, dashboard application broken. | ⚠️ **Half-stale** |
| Bookmarks require category (form enforcement) | "Spec updated, implementation not started" | **STALE (server) / PARTIAL (UI).** Server `create_bookmark` enforces non-nil category; UI still offers an empty "Select category" option that maps to nil (would be rejected). `update_bookmark` does not enforce. | ⚠️ **Mostly done** |

---

## Newly Discovered Gaps NOT in SPEC_STATE.md

| # | Gap | Spec Reference | Evidence | Severity |
|---|---|---|---|---|
| G-1 | **Dashboard settings not wired into `list_dashboard_query`** — the single most impactful hidden bug. Settings UI and server fns exist and pass tests, but the dashboard always renders with defaults. BV-004/BV-005/BV-014/BV-015 will all fail browser verification despite the UI looking functional. | FR-001; BV-004/005/014/015; T087 | `content_queries.rs:199-204` (`settings: DashboardSettings::default()`) vs `settings.rs:206-308` (real reader) | **Critical** |
| G-2 | `update_bookmark` does not enforce category_id requirement — only `create_bookmark` does. An operator can edit a bookmark and clear its category, recreating the BV-003 invisibility condition. | FR-021; data-model "REQUIRED" | `content_write.rs:499-536` (no nil check); compare `:476-480` | **High** |
| G-3 | No 'Uncategorized' default category is seeded. Spec says Application/Service category "defaults to 'Uncategorized'" but nothing creates that category on startup or migration. | FR-021; data-model Application/Service notes | grep `Uncategorized` in `migrations/` and `main.rs` → no seed found | Medium |
| G-4 | `upload_icon` writes to a hardcoded `data/icons` relative path, ignoring `DATA_DIR`/`EMBERWAKE_DB_PATH`. In the container (read-only rootfs, volume at `/var/lib/emberwake`), this would write to `/data/icons` (relative to CWD) and fail or be lost. | FR-003; T031; container hardening | `content_write.rs:633-636` (`"data/icons"` literal, `std::fs::write`) | **High** (deployment-breaking) |
| G-5 | No runtime amd64/arm64 parity test for SC-008. CI asserts build/clippy/test parity but never runs the image on both arches and compares behavior. | SC-008 | `.github/workflows/ci.yml` (no docker run step); `release.yml` builds but doesn't test-run | Medium |
| G-6 | SSR dashboard integration test (`ssr_dashboard.rs`) does not assert the Applications section — it only checks `pinned-services` and `pinned-categories` classes. BV-001 (three sections) is not regression-tested at the HTTP layer. | BV-001; T019 | `ssr_dashboard.rs:78-110` (no `applications-section` class assertion) | Medium |
| G-7 | ACME/built-in HTTPS (FR-019, a SHOULD) is entirely absent — no `rustls-acme` dependency, no code path in `main.rs`. `quickstart.md:103-104` documents it as a feature. | FR-019; quickstart.md:103 | grep `acme` in `crates/` → 0 matches | Low (SHOULD) |
| G-8 | `SECURITY_VERIFICATION.md:38` claims SC-006 has `authz.rs` tests, but the table row `tests/server_fn/authz.rs | — | Authorization boundary tests` shows **"—" (no test count)** — the file exists (137 lines, T033 visibility tests) but is undercounted in the evidence table. | SC-005 evidence | `SECURITY_VERIFICATION.md:38` | Low (doc) |
| G-9 | `e2e/Cargo.toml:6` declares `license = "MIT"` while the project is Apache-2.0 (`constitution.md:118`, `LICENSE` file). Minor license metadata inconsistency in the standalone e2e crate. | constitution.md:118 | `e2e/Cargo.toml:6` | Low |
| G-10 | `CHANGELOG.md:60` claims "107 tests" but this is a static count from Phase 12; Phase 13 added `application.rs` (8 tests) and `dashboard_settings.rs` (3 tests) without updating the count. | — | `CHANGELOG.md:60` | Low (doc) |
| G-11 | The root `AGENTS.md` "Phase 12 Deliverables" section (lines 88-96) is a historical record that does not mention Phase 13 at all, even though Phase 13 tasks now exist in `tasks.md` and Phase 13 code has landed. The DOX chain is out of sync with the task list. | DOX closeout contract | `AGENTS.md:88-96` | Low (DOX hygiene) |
| G-12 | `crates/AGENTS.md` Local Contract states "Security-critical code (auth, CSRF, sessions) lives in `server/`, never in `app/`" but `app/src/server/auth_helper.rs` houses CSRF/session enforcement. The contract is contradicted by the actual architecture (which is reasonable — server fns must enforce at the boundary). | crates/AGENTS.md:37 | `crates/app/src/server/auth_helper.rs:85,118` | Low (DOX hygiene) |

---

## Recommendations (Prioritized)

### P0 — Critical (fix before any further Phase 13 work)

1. **Wire dashboard settings into `list_dashboard_query`** (G-1 / D-1). Replace `settings: DashboardSettings::default()` at `content_queries.rs:203` with a call to the existing `get_dashboard_settings` reader (or inline the 6 `get_setting_raw` calls). This single change makes BV-004/BV-005/BV-014/BV-015 functional and closes bugs #4/#5 in SPEC_STATE. Add a regression test asserting a saved `services_enabled=false` actually hides the Services section.

2. **Fix uncategorized-bookmark dashboard rendering** (Bug #1 / D-4). Add a query in `list_dashboard_query` that fetches `WHERE category_id IS NULL` bookmarks and renders them under a synthetic 'Uncategorized' group. Pair with seeding a real 'Uncategorized' category (G-3).

3. **Fix `upload_icon` path** (G-4). Read `DATA_DIR` from config/`AppState` and write to `{data_dir}/icons/{id}.{ext}` instead of the hardcoded `"data/icons"`. Without this, icon upload breaks in the container.

### P1 — High

4. **Enforce category_id on `update_bookmark`** (G-2). Add the same `Uuid::nil()` rejection that `create_bookmark` has, and update the UI to make "Select category" a non-selectable placeholder rather than a nil-valued option.

5. **Update SPEC_STATE.md to reflect reality** — Phase 13 is ~78% done, not "Not started". Reclassify T085/T086/T088/T089/T090/T091/T092 as Complete; T087 as Partial (blocked on P0 #1); T093 as Not started. Close bugs #3 and #6. Revise deferred list (remove three-section/application-entity; keep dashboard-settings-application and bookmark-category-enforcement with accurate descriptions).

6. **Run the performance benchmark** (`benches/seed_benchmark.sh`) against a live server and record actual SC-001/002/003 numbers in `PERFORMANCE.md`. Currently three of four SC-004-adjacent criteria are "Measurable" but unverified.

### P2 — Medium

7. **Add an Applications-section assertion to `ssr_dashboard.rs`** (G-6) so BV-001 has an HTTP-layer regression test.
8. **Add a runtime amd64/arm64 parity smoke** to the release workflow (G-5) — even a trivial `docker run --platform ... /readyz` check would satisfy SC-008's "runs identically" intent.
9. **Seed an 'Uncategorized' category** on startup in `main.rs` (alongside the existing builtin-theme seeding) so FR-021's default category contract holds (G-3).

### P3 — Low (DOX / doc hygiene)

10. Update root `AGENTS.md` "Phase 12 Deliverables" to reference Phase 13 status (G-11).
11. Reconcile `crates/AGENTS.md` Local Contract about where auth/CSRF code lives vs the actual `app/src/server/auth_helper.rs` placement (G-12 / D-11).
12. Update `CHANGELOG.md` test count or remove the specific number (G-10).
13. Fix `e2e/Cargo.toml` license field to Apache-2.0 (G-9).
14. Decide whether ACME HTTPS (FR-019) is in-scope; if not, note it as a documented non-goal in `quickstart.md` (G-7).

---

**End of report.** No files were modified. All evidence paths are absolute or repo-relative with line numbers as noted.
