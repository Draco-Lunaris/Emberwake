# Feature Specification: Emberwake

**Feature Branch**: `001-greenfield`

**Created**: 2026-06-19

**Status**: Draft

**Input**: User description: "A whole new, standalone, upgraded self-hosted startpage written
end-to-end in Rust on an Ubuntu 26.04 Docker container. Goal is higher security, higher
performance, and modernization. Backwards compatibility with the legacy project's API,
database, config, or files is explicitly NOT required."

## Overview

A self-hosted startpage / application dashboard: a single-container web app that presents an
operator's services and bookmarks as a fast, searchable homescreen with built-in editors,
multi-user accounts, optional live service-status and weather widgets, optional Docker/
Kubernetes auto-discovery, and modern theming. It is a clean-sheet reimagining — inspired by
Flame/Dragons-Flame but bound by none of its formats. Everything is authored in Rust
(Leptos full-stack), security is the primary constraint, and performance budgets are tested.

Stories are prioritized as independently shippable slices. The MVP is US1–US3 (a fast,
editable, multi-user dashboard). P2/P3 stories layer on extended auth, theming, live widgets,
discovery, and data portability without breaking earlier slices.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A fast, searchable dashboard (Priority: P1) 🎯 MVP

A self-hoster opens the dashboard and sees their pinned services and bookmark groups rendered
as real HTML on first paint (SSR), then the interactive parts (search, editors) hydrate. They
type into a search box and results filter instantly with fuzzy matching; a prefix routes the
query to a configured web search provider.

**Why this priority**: A startpage that is not instant and searchable has no reason to exist;
this slice proves the core rendering pipeline (SSR + island hydration) and the performance
thesis of the rewrite.

**Independent Test**: Seed services/bookmarks/categories; load the page; confirm server-
rendered HTML contains the pinned items before any WASM runs, confirm fuzzy search filters
client-side, and confirm a prefixed query resolves to the provider URL.

**Acceptance Scenarios**:

1. **Given** seeded public content, **When** the page is requested, **Then** the initial HTML
   response already contains the pinned items (no blank-then-populate flash).
2. **Given** the page has hydrated, **When** the user types a partial/misspelled name,
   **Then** fuzzy-matched items rank to the top without a network round-trip.
3. **Given** a configured provider prefixed query, **When** submitted, **Then** the browser
   navigates to that provider's templated URL.

---

### User Story 2 - Manage content with built-in editors (Priority: P1)

A signed-in operator creates, edits, reorders (drag-and-drop), pins, and deletes services,
bookmarks, and categories through in-app editors. Changes apply optimistically and persist.

**Why this priority**: "No file editing necessary" is the product's core promise; this is the
write half of the dashboard and exercises the typed server-function boundary end-to-end.

**Independent Test**: Through the UI/server functions, create a category, add a service and a
bookmark, reorder via drag, pin, then delete; confirm optimistic UI, persistence across
reload, and that validation rejects bad input.

**Acceptance Scenarios**:

1. **Given** a signed-in operator, **When** they submit a new service, **Then** it appears
   immediately (optimistic) and is persisted with a generated id and ordering.
2. **Given** existing items, **When** dragged to reorder or toggled pinned, **Then** the new
   order/state persists and survives reload.
3. **Given** invalid input (empty name, malformed URL), **When** submitted, **Then** the
   server function rejects it with a typed validation error surfaced inline.

---

### User Story 3 - Accounts, sessions, and roles (Priority: P1)

The first run creates an admin account via a setup flow. Admins manage users; each user signs
in with a password (Argon2id), receives a server-side revocable session in a secure cookie,
and sees content per their role and the public/private visibility of items. Sign-out and
"revoke all sessions" work.

**Why this priority**: This is a multi-user, security-first product; identity and session
management are foundational, not optional. It replaces the legacy single shared password.

**Independent Test**: Complete first-run admin setup; create a second user; verify login
issues a secure session, a wrong password is throttled and rejected, private items are hidden
from anonymous/unauthorized users, sign-out invalidates the session server-side, and
"revoke all" kills other sessions.

**Acceptance Scenarios**:

1. **Given** a fresh install, **When** the operator completes first-run setup, **Then** an
   admin account exists and the setup route is closed thereafter.
2. **Given** valid credentials, **When** a user logs in, **Then** a rotating opaque session is
   stored server-side and set as an HttpOnly/Secure/SameSite cookie; repeated wrong passwords
   are rate-limited.
3. **Given** an active session, **When** the user signs out or an admin revokes it, **Then**
   the session is invalidated server-side immediately (token is not merely client-discarded).

---

### User Story 4 - Extended auth: SSO, passkeys, and API tokens (Priority: P2)

An operator optionally configures OIDC SSO (e.g. an identity provider) so users sign in via
the IdP; optionally registers a WebAuthn passkey for passwordless login; and issues scoped,
revocable API tokens for automation against a small public REST surface.

**Why this priority**: Modern identity is a headline upgrade over the legacy single password
and fits real homelab/SSO deployments, but the product is fully usable without it.

**Independent Test**: With a stub OIDC provider, complete an auth-code+PKCE login that maps to
a local user; register and authenticate a passkey (virtual authenticator); mint a scoped API
token, use it against the token-protected REST endpoint, then revoke it and confirm 401.

**Acceptance Scenarios**:

1. **Given** OIDC configured, **When** a user logs in via the IdP, **Then** they are mapped to
   a local account (provisioned per policy) and receive a normal session.
2. **Given** a registered passkey, **When** the user authenticates with it, **Then** login
   succeeds without a password.
3. **Given** a scoped API token, **When** used beyond its scope or after revocation, **Then**
   the request is rejected.

---

### User Story 5 - Modern theming and settings (Priority: P2)

An operator picks from built-in themes, edits design tokens in a theme builder, supplies
custom CSS, toggles widgets, and configures search providers and integration switches. Light/
dark follows system preference unless overridden.

**Why this priority**: Customization is a primary differentiator and the day-to-day surface,
but depends only on the settings store, not on widgets or discovery.

**Independent Test**: Save a theme (design tokens + custom CSS) and a custom search provider;
reload and confirm the theme renders and the provider routes; toggle a widget and confirm it
shows/hides.

**Acceptance Scenarios**:

1. **Given** a saved theme, **When** the page loads, **Then** the design tokens and custom CSS
   are applied server-side (no flash of default theme).
2. **Given** `prefers-color-scheme`, **When** no explicit theme is set, **Then** light/dark is
   chosen automatically.
3. **Given** a custom search provider, **When** its prefix is used, **Then** queries route to
   its URL template.

---

### User Story 6 - Live service status monitoring (Priority: P3)

An operator marks services for health monitoring (HTTP/TCP ping). The server checks them on a
schedule and the dashboard shows live up/down/latency indicators on each tile, pushed to the
browser without polling.

**Why this priority**: A genuinely new capability beyond the legacy app, turning the startpage
into a light status board. Independent of everything except the service catalog.

**Independent Test**: Configure a monitored service against a stub endpoint; confirm the
scheduler records status transitions and a connected client receives a live update when the
stub flips up→down.

**Acceptance Scenarios**:

1. **Given** a monitored service, **When** the scheduled check runs, **Then** its status and
   latency are recorded.
2. **Given** an open dashboard, **When** a monitored service changes state, **Then** the tile
   updates live via the server push (SSE/WebSocket) without a manual refresh.
3. **Given** monitoring disabled for a service, **When** checks run, **Then** it is skipped and
   no outbound request is made for it.

---

### User Story 7 - Weather widget (Priority: P3)

An operator supplies a weather API key and location; the homescreen shows current conditions,
refreshed on a schedule server-side and pushed live, never calling the upstream from the
browser.

**Why this priority**: A loved, non-essential widget reusing the same background-task +
server-push machinery as status monitoring.

**Independent Test**: With a stubbed weather upstream, confirm the scheduled fetch caches a
reading, the widget renders from cache, and a live update is pushed on refresh.

**Acceptance Scenarios**:

1. **Given** valid weather config, **When** the interval elapses, **Then** a new reading is
   fetched and cached.
2. **Given** a cached reading, **When** the widget renders, **Then** it uses cache with no
   synchronous upstream call.
3. **Given** missing config, **When** the app runs, **Then** the widget is inert and nothing
   else is affected.

---

### User Story 8 - Docker & Kubernetes auto-discovery (Priority: P3)

When enabled, the server discovers services from Docker container labels (and live Docker
events) and/or Kubernetes Ingress annotations, surfacing them as tiles. All access is
read-only.

**Why this priority**: A power-user convenience, opt-in and isolated. Modernized to react to
Docker events live rather than only on a poll.

**Independent Test**: With a mocked Docker source, confirm labeled containers map to services
(including multi-value label syntax) and that a container-start event surfaces a tile live;
confirm disabled = no calls.

**Acceptance Scenarios**:

1. **Given** Docker discovery enabled, **When** labeled containers exist, **Then** they appear
   as services with parsed name/url/icon.
2. **Given** the Docker event stream, **When** a labeled container starts/stops, **Then** the
   dashboard reflects it live.
3. **Given** an integration disabled, **When** discovery would run, **Then** no external call
   is made.

---

### User Story 9 - Import / Export & portability (Priority: P3)

An operator exports all data to JSON for backup/migration, imports a JSON export into another
instance, and imports browser HTML bookmark exports and OPML. Imports validate, enforce size
limits, detect duplicates, and preview before applying.

**Why this priority**: Essential for backup and onboarding from other tools, but not needed
to run day to day. The parsers handle untrusted input and are fuzzed.

**Independent Test**: Round-trip a full export → fresh-instance import for equivalence; import
a sample bookmarks HTML and an OPML file and assert categories/bookmarks with duplicate
handling; assert oversized/malformed input is rejected before any write.

**Acceptance Scenarios**:

1. **Given** populated data, **When** a full export is requested, **Then** a JSON document of
   all entity types is produced.
2. **Given** a JSON export, **When** imported into an empty instance, **Then** the data set
   matches the source (modulo regenerated ids).
3. **Given** a malformed or oversized import, **When** submitted, **Then** it is rejected with
   a clear error and no partial write occurs.

---

### Edge Cases

- First-run race: two setup requests arrive together → exactly one admin is created; the
  second is rejected. Race safety enforced via a `setup_complete` singleton setting key with a
  `UNIQUE` constraint, checked inside the admin-creation transaction.
- Session fixation/rotation: a privilege change rotates the session token; the old token is
  invalid immediately.
- Argon2id parameters too aggressive for arm64 → parameters are configurable and login stays
  under the latency budget on both arches.
- Monitored service is slow/unreachable → check times out, records "down" with the timeout
  reason, and never blocks the scheduler or the request path.
- Weather/IdP upstream error → last good state retained; failure logged, never surfaced to
  unrelated views.
- Oversized import or zip-bomb-style nested bookmarks → rejected by size/derivation limits
  before any write.
- Docker socket absent but discovery enabled → integration reports unavailable; core
  unaffected.
- CSP blocks an inline style/script → all first-party assets are nonce/hash-allowed; no
  feature relies on unsafe-inline.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST server-render the dashboard (pinned services, bookmark groups,
  search box) so first paint contains content, then hydrate only interactive islands.
- **FR-002**: System MUST provide fuzzy client-side search across services/bookmarks plus
  prefix routing to configurable web search providers.
- **FR-003**: System MUST expose typed server functions for full CRUD + reorder + pin of
  services, bookmarks, and categories, with server-side validation and optimistic UI support.
- **FR-004**: System MUST support multi-user accounts with roles (at minimum admin + user), a
  first-run admin setup flow that closes after use, and admin user management.
- **FR-005**: System MUST hash passwords with Argon2id (configurable cost) and MUST never log
  or return secrets.
- **FR-006**: System MUST use server-side, opaque, rotating, revocable sessions delivered in
  HttpOnly/Secure/SameSite cookies, with working sign-out and revoke-all.
- **FR-007**: System MUST CSRF-protect all state-changing requests and enforce
  authorization (role/ownership) at the server boundary; auth MUST fail closed.
- **FR-008**: System MUST send a strict CSP and standard security headers on every response,
  use rustls for all TLS, and MUST NOT link system OpenSSL.
- **FR-009**: System MUST rate-limit sensitive routes (login, token, import) and throttle
  repeated auth failures.
- **FR-010**: System MUST optionally support OIDC SSO (auth code + PKCE), optional WebAuthn
  passkeys, and scoped revocable API tokens for a small public REST surface.
- **FR-011**: System MUST provide a settings store and theming (built-in themes, design-token
  builder, custom CSS, light/dark via system preference), applied server-side without a flash.
- **FR-012**: System MUST optionally monitor service health on a schedule (HTTP/TCP) and push
  live status to connected clients (SSE/WebSocket).
- **FR-013**: System MUST optionally fetch weather server-side on a schedule, cache it, render
  from cache, and push updates live.
- **FR-014**: System MUST optionally discover services from Docker labels (incl. live events)
  and Kubernetes Ingress annotations, read-only, and make no mutating calls to either.
- **FR-015**: System MUST provide JSON full/selective export and JSON/HTML/OPML import with
  validation, size limits, duplicate detection, and preview; import parsers MUST be fuzzed.
- **FR-016**: System MUST write an append-only audit log of security-relevant events.
- **FR-017**: System MUST expose `/healthz`, `/readyz`, and a Prometheus `/metrics` endpoint,
  emit structured logs at an env-configurable level, and support OpenTelemetry trace export.
- **FR-018**: System MUST be delivered as a single signed, multi-arch (`amd64`/`arm64`) image
  with an `ubuntu:26.04` runtime, an SBOM, a `HEALTHCHECK`, running as non-root, persisting
  data under a single volume.
- **FR-019**: System SHOULD support optional built-in HTTPS via rustls + ACME for operators
  not behind a reverse proxy.
- **FR-020**: System MUST perform scheduled WAL checkpoints and SHOULD perform automated
  SQLite `.backup` to the data volume with configurable retention limits (max count, max total
  size) to preserve disk space.

### Key Entities *(include if feature involves data)*

- **User**: an account — id, username, optional email, optional Argon2id password hash, role,
  active flag, timestamps. May authenticate by password, OIDC identity, or passkey.
- **Session**: a server-side login — opaque id, user, issue/expiry/last-used timestamps,
  client metadata; revocable individually or en masse.
- **External Identity**: a link from a User to an OIDC provider subject.
- **Passkey Credential**: a WebAuthn credential for a User (credential id, public key,
  signature counter).
- **API Token**: a scoped, revocable automation credential (hash, scopes, expiry, owner).
- **Category**: a named group (services and/or bookmarks) — id, name, icon, order, visibility.
- **Service**: a launchable/monitored application tile — id, name, URL, icon, description,
  pinned, order, visibility, optional monitoring config; belongs to a Category optionally.
- **Bookmark**: a link in a Category — id, name, URL, icon, order, visibility.
- **Setting**: typed key/value configuration including search providers, integration toggles,
  weather config (secret-bearing), and active theme.
- **Theme**: design tokens + custom CSS + metadata.
- **Status Reading**: latest monitored-service health (state, latency, checked-at, reason).
- **Status History**: bounded log of service health transitions for uptime tracking (state,
  latency, checked-at, reason; retention-limited by max-rows and max-age).
- **Weather Reading**: latest cached weather observation.
- **Audit Event**: append-only security/action record (ts, actor, action, target, ip, result).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: First-paint (server-rendered HTML containing pinned content) Time-To-First-Byte
  under 50 ms and fully interactive (hydrated) under 1 s on commodity hardware for a catalog
  of 200 services / 500 bookmarks.
- **SC-002**: Authenticated CRUD server-function p95 under 25 ms (excluding network) at that
  catalog size.
- **SC-003**: Idle resident memory at or below 48 MB; cold start to `readyz` under 1.5 s.
- **SC-004**: WASM hydration bundle (compressed) under 350 KB for the dashboard route.
- **SC-005**: 100% of mutating server functions enforce authentication + CSRF; verified by an
  automated test that asserts each rejects unauthenticated/forged requests.
- **SC-006**: `cargo-deny` and `cargo-audit` report zero unresolved advisories or disallowed
  licenses at release; the release image is signed and ships an SBOM.
- **SC-007**: Import parsers survive a fuzzing run (no panics/OOM) and reject all malformed
  corpus inputs without partial writes.
- **SC-008**: The published image runs identically on `amd64` and `arm64` from the same tag,
  verified in CI each release.

## Assumptions

- This is greenfield: no data, API, config, or file compatibility with the legacy project is
  required or attempted. Migration from the old app, if ever wanted, is a one-way import
  feature, not a parity guarantee, and is out of scope here.
- The frontend is Leptos (SSR + hydrate); a JavaScript frontend is not used.
- SQLite (WAL) is the datastore; zero external service dependencies is intended. A Postgres
  backend behind the repository trait is a possible future option, out of scope for v1.
- Operators run behind their own TLS-terminating reverse proxy by default; built-in ACME HTTPS
  is an optional convenience.
- Docker/Kubernetes integrations are opt-in and read-only; the Docker socket is mounted only
  when the operator enables Docker discovery.
- `ubuntu:26.04` is available for both target architectures at build time.
