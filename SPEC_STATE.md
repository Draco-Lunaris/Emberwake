# Project State

Last updated: 2026-06-24

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
| T085-T093 | Phase 13 Three-Section Dashboard Redesign | ❌ Not started | Spec updated |
| Restricted visibility | Feature addition | ✅ Complete | Added public/private/restricted |
| Title as dashboard link | Feature addition | ✅ Complete | Navbar title wraps Leptos A link |
| Search providers wired | T025 fix | ✅ Complete | Server fn + HomePage wiring |
| Session token rotation | T036 fix | ✅ Complete | 30-min rotation in lookup_session |
| CSRF origin check | T037 fix | ✅ Complete | validate_origin() in auth_helper |
| OTLP telemetry | T013 fix | ✅ Complete | Real OTLP exporter with guard |
| Security headers test | T017 fix | ✅ Complete | Uses shared apply_security_headers() |
| Passkey real WebAuthn | T049 fix | ✅ Complete | navigator.credentials.create() |
| HTML parser fix | T074 fix | ✅ Complete | Recursive DOM walk for categories |
| StatusTile wired | Dashboard fix | ✅ Complete | Replaced ServiceTile with StatusTile |

## Browser-Verified Status

| Feature | Test Status | Browser Status | Notes |
|---------|------------|----------------|-------|
| Login | ✅ 13 tests | ✅ BV-012 verified | Works |
| Logout | ✅ Tests pass | ⚠️ BV-013 not verified | Not tested in browser |
| Dashboard render | ✅ 2 tests | ✅ BV-001 partial | Three sections not yet implemented |
| Bookmark clickability | ❌ No test | ✅ BV-002 verified | target=_blank works |
| Bookmark without category | ❌ No test | ❌ BV-003 FAILS | Bookmarks invisible without category |
| Add bookmark | ✅ 11 tests | ✅ BV-006 verified | Appears in editor list |
| New bookmark on dashboard | ❌ No test | ❌ BV-007 FAILS | Not visible without category |
| Edit bookmark | ✅ Tests pass | ✅ BV-008 verified | Form pre-fills correctly |
| Update bookmark | ✅ Tests pass | ⚠️ BV-009 not verified | Not tested in browser |
| Icon/description fields | ❌ No test | ⚠️ BV-010 partial | Fields exist but untested |
| Layout toggle | ❌ No test | ⚠️ Not verified | Grid/List toggle exists |
| Three-section layout | ❌ No test | ❌ Not implemented | Services/Applications/Bookmarks |
| Per-section columns | ❌ No test | ❌ Not implemented | Single global setting |
| Section enable/disable | ❌ No test | ❌ Not implemented | Not available |
| Application entity | ❌ No test | ❌ Not implemented | New entity needed |

## Open Bugs

| # | Description | Severity | Task | Status |
|---|-------------|----------|------|--------|
| 1 | Bookmarks without category are invisible on dashboard | High | BV-003 | ❌ Unfixed |
| 2 | New bookmarks don't appear on dashboard (no category assigned) | High | BV-007 | ❌ Unfixed |
| 3 | Three-section layout not implemented (Services/Applications/Bookmarks) | High | BV-001 | ❌ Not started |
| 4 | Per-section column settings not implemented | Medium | BV-005 | ❌ Not started |
| 5 | Section enable/disable not implemented | Medium | BV-004 | ❌ Not started |
| 6 | Application entity not implemented | High | BV-011 | ❌ Not started |
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
| Three-section dashboard (Services/Applications/Bookmarks) | Spec updated, implementation not started | — |
| Application entity + migration | Spec updated, implementation not started | — |
| Per-section column settings | Spec updated, implementation not started | — |
| Section enable/disable | Spec updated, implementation not started | — |
| Bookmarks require category (form enforcement) | Spec updated, implementation not started | — |
