# Project State

Last updated: 2026-07-05

## Tasks

| Task | Spec Section | Status | Notes |
|------|-------------|--------|-------|
| T001-T005 | Phase 1 Setup | ✅ Complete | Verified 2026-06-19 |
| T006-T017 | Phase 2 Foundational | ✅ Complete | Verified 2026-06-19 |
| T018-T025 | Phase 3 US1 Dashboard | ✅ Complete | Verified 2026-06-20 |
| T026-T031 | Phase 4 US2 CRUD | ✅ Complete | Verified 2026-06-20 |
| T032-T041 | Phase 5 US3 Auth | ✅ Complete | Verified 2026-06-20 |
| T042-T049 | Phase 6 US4 Extended Auth | ✅ Complete | Verified 2026-06-20 |
| T050-T055 | Phase 7 US5 Theming | ✅ Complete | Verified 2026-06-20 |
| T056-T060 | Phase 8 US6 Monitoring | ✅ Complete | Verified 2026-06-20 |
| T061-T063 | Phase 9 US7 Weather | ✅ Complete | Verified 2026-06-20 |
| T064-T068 | Phase 10 US8 Discovery | ✅ Complete | Verified 2026-06-20 |
| T069-T075 | Phase 11 US9 Import/Export | ✅ Complete | Verified 2026-06-20 |
| T076-T084 | Phase 12 Polish | ✅ Complete | Verified 2026-06-23 |
| T085 | Phase 13 Application entity | ✅ Complete | migration 000005 + domain + CRUD + repository + editor + 8 tests |
| T086 | Phase 13 Per-section column setting keys | ✅ Complete | 6 setting keys + 3 dashboard_settings tests |
| T087 | Phase 13 Dashboard renders three sections | ✅ Complete | Dashboard component + settings wired into list_dashboard_query |
| T088 | Phase 13 ApplicationEditor | ✅ Complete | editors/mod.rs ApplicationEditor + ApplicationEditPage |
| T089 | Phase 13 BookmarkEditor requires category | ✅ Complete | create_bookmark + update_bookmark reject nil category |
| T090 | Phase 13 Section enable/disable toggle | ✅ Complete | settings_page.rs DashboardSettingsSection |
| T091 | Phase 13 Per-section column selector | ✅ Complete | settings_page.rs column inputs |
| T092 | Phase 13 CSS for three-section + per-section grid | ✅ Complete | style/main.css .applications-section + pinned-categories |
| T093 | Phase 13 Browser-verify BV-001..BV-015 | ❌ Not started | Requires running server + browser |
| Restricted visibility | Feature addition | ✅ Complete | Added public/private/restricted |
| Title as dashboard link | Feature addition | ✅ Complete | Navbar title wraps Leptos A link |
| Search providers wired | T025 fix | ✅ Complete | Server fn + HomePage wiring |
| Session token rotation | T036 fix | ✅ Complete | 30-min rotation in lookup_session |
| CSRF origin check | T037 fix | ✅ Complete | validate_origin() strict host:port equality |
| OTLP telemetry | T013 fix | ✅ Complete | Real OTLP exporter with guard |
| Security headers test | T017 fix | ✅ Complete | Uses shared apply_security_headers() |
| Passkey real WebAuthn | T049 fix | ✅ Complete | navigator.credentials.create() |
| HTML parser fix | T074 fix | ✅ Complete | Recursive DOM walk for categories |
| StatusTile wired | Dashboard fix | ✅ Complete | Replaced ServiceTile with StatusTile |
| Dashboard settings wiring | Gap fix 2026-07-05 | ✅ Complete | list_dashboard_query reads persisted settings |
| Uncategorized bookmarks | Gap fix 2026-07-05 | ✅ Complete | NULL category_id bookmarks render under synthetic Uncategorized group |
| Icon upload path | Gap fix 2026-07-05 | ✅ Complete | IconsDir Extension derived from db_path parent |
| update_bookmark category | Gap fix 2026-07-05 | ✅ Complete | Rejects nil category_id on update |
| Constant-time CSRF compare | Security fix 2026-07-05 | ✅ Complete | subtle::ConstantTimeEq |
| Strict Origin validation | Security fix 2026-07-05 | ✅ Complete | Strict host:port equality; no substring; no empty-host bypass |
| HMAC-signed import token | Security fix 2026-07-05 | ✅ Complete | preview token signed with server_key |
| Configurable WebAuthn RP | Security fix 2026-07-05 | ✅ Complete | webauthn.rp_id / webauthn.rp_origin config; localhost fallback |
| OIDC approval server fn | Feature gap 2026-07-05 | ✅ Complete | list_pending_identities + approve_external_identity |
| Application is_pinned honored | Bug fix 2026-07-05 | ✅ Complete | create_application_query uses input.is_pinned |
| Reorder scoped to category | Bug fix 2026-07-05 | ✅ Complete | services/applications/bookmarks reorder WHERE category_id = ? |
| DiscoveryCache poison-safe | Robustness fix 2026-07-05 | ✅ Complete | RwLock unwrap replaced with graceful poison handling |

## Browser-Verified Status

| Feature | Test Status | Browser Status | Notes |
|---------|------------|----------------|-------|
| Login | ✅ 13 tests | ✅ BV-012 verified | Works |
| Logout | ✅ Tests pass | ⚠️ BV-013 not verified | Not tested in browser |
| Dashboard render | ✅ 2 tests | ⚠️ BV-001 partial | Three sections implemented; BV not run |
| Bookmark clickability | ❌ No test | ✅ BV-002 verified | target=_blank works |
| Bookmark without category | ❌ No test | ⚠️ BV-003 fixed, unverified | Uncategorized synthetic group renders |
| Add bookmark | ✅ 11 tests | ✅ BV-006 verified | Appears in editor list |
| New bookmark on dashboard | ❌ No test | ⚠️ BV-007 fixed, unverified | Category now required at create+update |
| Edit bookmark | ✅ Tests pass | ✅ BV-008 verified | Form pre-fills correctly |
| Update bookmark | ✅ Tests pass | ⚠️ BV-009 not verified | Not tested in browser |
| Icon/description fields | ❌ No test | ⚠️ BV-010 partial | Fields exist but untested |
| Layout toggle | ❌ No test | ⚠️ Not verified | Grid/List toggle exists |
| Three-section layout | ❌ No test | ⚠️ Implemented, unverified | Services/Applications/Bookmarks |
| Per-section columns | ✅ 3 tests | ⚠️ Implemented, unverified | Wired into list_dashboard_query |
| Section enable/disable | ✅ 3 tests | ⚠️ Implemented, unverified | Wired into list_dashboard_query |
| Application entity | ✅ 8 tests | ⚠️ Implemented, unverified | Migration + CRUD + editor + dashboard tile |

## Open Bugs

| # | Description | Severity | Task | Status |
|---|-------------|----------|------|--------|
| 1 | Bookmarks without category invisible on dashboard | High | BV-003 | ✅ Fixed 2026-07-05 (synthetic Uncategorized group) |
| 2 | New bookmarks don't appear on dashboard (no category assigned) | High | BV-007 | ✅ Fixed 2026-07-05 (category required at create+update) |
| 3 | Three-section layout not implemented | High | BV-001 | ✅ Fixed (component + settings wired) |
| 4 | Per-section column settings not implemented | Medium | BV-005 | ✅ Fixed (wired into list_dashboard_query) |
| 5 | Section enable/disable not implemented | Medium | BV-004 | ✅ Fixed (wired into list_dashboard_query) |
| 6 | Application entity not implemented | High | BV-011 | ✅ Fixed (migration + CRUD + editor) |
| 7 | Docker build may fail with web-sys version issues | Medium | Deployment | ⚠️ Fixed in Cargo.lock |

## Deferred Items

| Item | Reason | Blocked by |
|------|--------|------------|
| Full Leptos SSR pipeline tests (T019, T051) | Requires cargo-leptos build pipeline in test env | — |
| Full stub IdP OIDC roundtrip (T042) | Requires mock HTTP server serving discovery + token endpoints | — |
| Full virtual authenticator WebAuthn (T043) | Requires browser WebAuthn API | — |
| Full Docker API mocking (T065) | Requires mock Docker daemon | — |
| Performance benchmarks SC-001/002/003 (T080) | Requires running server + seeded catalog | — |
| E2E tests (T082) | Require WebDriver server (geckodriver/chromedriver) | — |
| Quickstart walkthrough execution (T084) | Requires running server + browser | — |
| CI/CD pipeline validation (T078/T079) | Requires GitHub Actions run | — |
| Browser verification BV-001..BV-015 (T093) | Requires running server + browser | — |