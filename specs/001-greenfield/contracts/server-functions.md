# Contract: Server-Function Boundary

**Branch**: `001-greenfield` | **Date**: 2026-06-19

In a Leptos app the primary client/server contract is the set of typed `#[server]` functions,
not a hand-written JSON API. They are the gate referenced by Constitution Principle I: changing
a signature is a compile error on both sides. Signatures below are the design intent (Rust-ish
pseudocode); exact types live in `crates/app/src/domain`. Every mutating function enforces
auth + CSRF + authorization server-side (Principle II); reads exclude private rows for
unauthorized callers in SQL (`data-model.md`).

## Conventions

- Return type is `Result<T, ServerFnError<AppError>>`; `AppError` carries typed validation and
  authz failures surfaced inline in the UI.
- `Visibility`, `Role`, ids (`Uuid`), and DTOs are shared domain types.
- "auth: X" = minimum caller requirement. "public" = unauthenticated allowed (public rows
  only). Mutations are implicitly CSRF-protected.

## Content — services, bookmarks, categories

```rust
// Reads (public: returns only public rows unless an authorized session is present)
list_dashboard() -> DashboardView            // pinned services + pinned categories+bookmarks
list_categories() -> Vec<CategoryWithItems>
list_services(filter: ServiceFilter) -> Vec<Service>
list_bookmarks(category: Option<Uuid>) -> Vec<Bookmark>

// Mutations (auth: user; admin may act on any, users on owned/permitted)
create_category(input: CategoryInput) -> Category
update_category(id: Uuid, patch: CategoryPatch) -> Category
delete_category(id: Uuid) -> ()                       // item reparent/cascade per tested policy
reorder_categories(order: Vec<Uuid>) -> ()

create_service(input: ServiceInput) -> Service
update_service(id: Uuid, patch: ServicePatch) -> Service
delete_service(id: Uuid) -> ()
reorder_services(category: Option<Uuid>, order: Vec<Uuid>) -> ()
set_service_pinned(id: Uuid, pinned: bool) -> Service

create_bookmark(input: BookmarkInput) -> Bookmark
update_bookmark(id: Uuid, patch: BookmarkPatch) -> Bookmark
delete_bookmark(id: Uuid) -> ()
reorder_bookmarks(category: Uuid, order: Vec<Uuid>) -> ()

upload_icon(file: MultipartFile) -> IconRef           // auth: user; size/type validated
```

## Auth & accounts

```rust
// First-run setup (open only until an admin exists; race-safe)
setup_status() -> SetupState                          // public
complete_setup(input: AdminSetupInput) -> ()          // public, single-shot

// Password auth
login(input: LoginInput) -> SessionSummary            // public; rate-limited; audits
logout() -> ()                                        // auth: session
current_user() -> Option<UserSummary>                 // public (null if anon)

// Session management
list_sessions() -> Vec<SessionSummary>                // auth: user (own)
revoke_session(id: String) -> ()                      // auth: user (own) / admin (any)
revoke_all_other_sessions() -> ()                     // auth: user

// Admin user management
list_users() -> Vec<UserSummary>                      // auth: admin
create_user(input: NewUserInput) -> UserSummary       // auth: admin
update_user(id: Uuid, patch: UserPatch) -> UserSummary// auth: admin
deactivate_user(id: Uuid) -> ()                       // auth: admin
```

## Extended auth (optional features)

```rust
// OIDC (browser flow completes at the REST callback in public-api.yaml; these manage config/state)
oidc_begin() -> RedirectUrl                           // public; auth-code + PKCE
list_external_identities() -> Vec<ExternalIdentity>   // auth: user (own)
unlink_external_identity(id: Uuid) -> ()              // auth: user (own)

// WebAuthn passkeys
passkey_register_begin() -> CredentialCreationOptions // auth: user
passkey_register_finish(resp: RegisterResponse) -> () // auth: user
passkey_login_begin(username: String) -> RequestOptions// public
passkey_login_finish(resp: AuthResponse) -> SessionSummary // public

// Scoped API tokens
list_api_tokens() -> Vec<ApiTokenSummary>             // auth: user (own)
create_api_token(input: ApiTokenInput) -> ApiTokenSecret // auth: user; secret shown once
revoke_api_token(id: Uuid) -> ()                      // auth: user (own) / admin
```

## Settings, theming, integrations

```rust
get_settings() -> SettingsView                        // auth: admin (secrets redacted for others)
update_settings(patch: SettingsPatch) -> SettingsView // auth: admin; audits
list_themes() -> Vec<ThemeSummary>                    // public
get_active_theme() -> Theme                           // public (applied during SSR)
save_theme(input: ThemeInput) -> Theme                // auth: admin
set_active_theme(id: Uuid) -> ()                      // auth: admin

// Discovery (read-only; no-op when disabled)
discover_docker() -> Vec<DiscoveredService>           // auth: admin
discover_kubernetes() -> Vec<DiscoveredService>       // auth: admin
```

## Widgets data (cache reads; live deltas via SSE — see public-api.yaml)

```rust
get_weather() -> Option<WeatherReading>               // public; serves cache only
get_service_statuses() -> Vec<StatusReading>          // public for public services
```

## Import / export

```rust
export_data(scope: ExportScope) -> ExportDocument     // auth: admin
import_preview(file: MultipartFile, kind: ImportKind) -> ImportPreview  // auth: admin; bounded+fuzzed
import_apply(token: PreviewToken, options: ImportOptions) -> ImportResult // auth: admin; audits
```

> Notes: `ImportKind` ∈ { Json, HtmlBookmarks, Opml }. `import_preview` parses under
> `spawn_blocking` with size/derivation limits and returns a preview without writing; only
> `import_apply` mutates, transactionally, with duplicate handling per `ImportOptions`.
