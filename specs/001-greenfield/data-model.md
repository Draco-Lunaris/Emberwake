# Phase 1 Data Model: Emberwake

**Branch**: `001-greenfield` | **Date**: 2026-06-19

A clean schema with no legacy-compatibility obligation. SQLite (WAL) via SQLx; UUIDv7 primary
keys (time-ordered, index-friendly); UTC timestamps; soft-typed JSON only where genuinely
schemaless (settings, theme tokens). Migrations are forward SQL files under `migrations/`.

## Identity & access

### User

| Field           | Type            | Notes                                              |
|-----------------|-----------------|----------------------------------------------------|
| id              | TEXT PK (uuidv7)|                                                    |
| username        | TEXT UNIQUE     | required; case-insensitive unique                  |
| email           | TEXT NULL UNIQUE| optional                                           |
| password_hash   | TEXT NULL       | Argon2id PHC string; NULL if SSO/passkey-only      |
| role            | TEXT            | `admin` \| `user` (extensible)                     |
| is_active       | INTEGER         | bool; disabled users cannot authenticate           |
| created_at      | TEXT            | RFC3339 UTC                                         |
| updated_at      | TEXT            |                                                    |
| last_login_at   | TEXT NULL       |                                                    |

### Session

Server-side, opaque, rotating, revocable. Backed by the `tower-sessions` SQLx store.

| Field        | Type     | Notes                                              |
|--------------|----------|----------------------------------------------------|
| id           | TEXT PK  | opaque high-entropy token id (not a JWT)           |
| user_id      | TEXT FK  | → User.id; ON DELETE CASCADE                       |
| created_at   | TEXT     |                                                    |
| expires_at   | TEXT     | absolute expiry; rotated on privilege change       |
| last_used_at | TEXT     | idle-timeout tracking                              |
| user_agent   | TEXT NULL| client metadata for the session list UI           |
| ip           | TEXT NULL|                                                    |

Revocation = row delete. "Revoke all" = delete all rows for a user except the current.

### ExternalIdentity (OIDC)

| Field      | Type    | Notes                                               |
|------------|---------|-----------------------------------------------------|
| id         | TEXT PK |                                                     |
| user_id    | TEXT FK | → User.id; ON DELETE CASCADE                        |
| provider   | TEXT    | issuer identifier                                   |
| subject    | TEXT    | OIDC `sub`; UNIQUE(provider, subject)               |
| created_at | TEXT    |                                                     |

### PasskeyCredential (WebAuthn)

| Field         | Type     | Notes                                            |
|---------------|----------|--------------------------------------------------|
| id            | TEXT PK  |                                                  |
| user_id       | TEXT FK  | → User.id; ON DELETE CASCADE                     |
| credential_id | BLOB     | UNIQUE                                            |
| public_key    | BLOB     | COSE key                                          |
| sign_count    | INTEGER  | clone-detection counter                          |
| created_at    | TEXT     |                                                  |

### ApiToken

Scoped automation credential for the public REST surface. Only a hash is stored.

| Field      | Type     | Notes                                               |
|------------|----------|-----------------------------------------------------|
| id         | TEXT PK  |                                                     |
| user_id    | TEXT FK  | → User.id; owner; ON DELETE CASCADE                 |
| name       | TEXT     | operator label                                      |
| token_hash | TEXT     | Argon2id/HMAC of the secret; secret shown once      |
| scopes     | TEXT     | JSON array of scope strings (least privilege)       |
| expires_at | TEXT NULL|                                                     |
| revoked_at | TEXT NULL| set on revoke; checked on every use                 |
| created_at | TEXT     |                                                     |
| last_used_at| TEXT NULL|                                                    |

## Content

### Category

| Field       | Type     | Notes                                               |
|-------------|----------|-----------------------------------------------------|
| id          | TEXT PK  |                                                     |
| name        | TEXT     | required                                            |
| icon        | TEXT NULL| icon ref                                            |
| order_index | INTEGER  | manual ordering                                     |
| visibility  | TEXT     | `public` \| `private`                               |
| created_at  | TEXT     |                                                     |
| updated_at  | TEXT     |                                                     |

Relationship: **Category 1—* Service** and **Category 1—* Bookmark** (both FKs nullable for
uncategorized items; ON DELETE: items set null or cascade per a chosen, tested policy).

### Service

A launchable and optionally monitored tile.

| Field          | Type      | Notes                                            |
|----------------|-----------|--------------------------------------------------|
| id             | TEXT PK   |                                                  |
| category_id    | TEXT FK NULL| → Category.id                                  |
| name           | TEXT      | required                                          |
| url            | TEXT      | required; validated                              |
| icon           | TEXT NULL | icon ref or uploaded asset                       |
| description    | TEXT NULL |                                                  |
| is_pinned      | INTEGER   | bool; drives homescreen inclusion                |
| order_index    | INTEGER   |                                                  |
| visibility     | TEXT      | `public` \| `private`                            |
| monitor_enabled| INTEGER   | bool                                             |
| monitor_kind   | TEXT NULL | `http` \| `tcp`                                  |
| monitor_target | TEXT NULL | URL/host:port to probe (defaults to `url`)       |
| monitor_interval_s | INTEGER NULL | check cadence                              |
| created_at     | TEXT      |                                                  |
| updated_at     | TEXT      |                                                  |

### Bookmark

| Field       | Type     | Notes                                               |
|-------------|----------|-----------------------------------------------------|
| id          | TEXT PK  |                                                     |
| category_id | TEXT FK NULL| → Category.id                                     |
| name        | TEXT     | required                                            |
| url         | TEXT     | required; validated                                 |
| icon        | TEXT NULL|                                                     |
| order_index | INTEGER  |                                                     |
| visibility  | TEXT     | `public` \| `private`                               |
| created_at  | TEXT     |                                                     |
| updated_at  | TEXT     |                                                     |

## Configuration & presentation

### Setting

Typed key/value store; values are JSON, read through a typed registry (never parsed ad hoc).

| Field      | Type     | Notes                                               |
|------------|----------|-----------------------------------------------------|
| key        | TEXT PK  | e.g. `search.providers`, `search.default`,`integrations.docker`, `integrations.k8s`, `weather.*`,`theme.active`, `auth.oidc`, `auth.passkeys` |
| value      | TEXT     | JSON; secret-bearing keys flagged in the registry   |
| updated_at | TEXT     |                                                     |

Secret-bearing settings (weather key, OIDC client secret) are encrypted at rest with a
key derived from a server secret, and never returned to non-admins or logged.

The `setup_complete` key is a singleton used as a race-safe gate for first-run admin setup:
its absence means setup is open; its presence (checked inside the admin-creation transaction)
means setup is closed. A `UNIQUE` constraint on this key ensures exactly one admin is created
even under concurrent setup requests.

### Theme

| Field      | Type     | Notes                                               |
|------------|----------|-----------------------------------------------------|
| id         | TEXT PK  |                                                     |
| name       | TEXT     |                                                     |
| tokens     | TEXT     | JSON design tokens (colors, spacing, radius, etc.)  |
| custom_css | TEXT NULL| operator CSS, served with CSP-safe handling         |
| is_builtin | INTEGER  | bool; builtins are read-only                        |
| created_by | TEXT FK NULL| → User.id                                         |
| created_at | TEXT     |                                                     |

## Runtime/cache & audit

### StatusReading

Latest health per monitored service (one current row per service).

| Field        | Type     | Notes                                              |
|--------------|----------|----------------------------------------------------|
| service_id   | TEXT FK  | → Service.id; ON DELETE CASCADE                    |
| state        | TEXT     | `up` \| `down` \| `degraded`                       |
| latency_ms   | INTEGER NULL|                                                 |
| reason       | TEXT NULL| e.g. timeout, status code                          |
| checked_at   | TEXT     |                                                    |

### StatusHistory

Bounded history of service health transitions for uptime tracking. Retention limited by
configurable max-rows-per-service (default 1000) and max-age-days (default 30). Pruned on
write and on a scheduled cleanup.

| Field        | Type     | Notes                                              |
|--------------|----------|----------------------------------------------------|
| id           | TEXT PK  | uuidv7 (time-ordered)                              |
| service_id   | TEXT FK  | → Service.id; ON DELETE CASCADE                    |
| state        | TEXT     | `up` \| `down` \| `degraded`                       |
| latency_ms   | INTEGER NULL|                                                 |
| reason       | TEXT NULL| e.g. timeout, status code                          |
| checked_at   | TEXT     |                                                    |

A `get_uptime_summary(service_id, window)` read function computes uptime percentage from
this table over a configurable time window.

### WeatherReading

Single-row latest cache: temp, condition, is-day, cloud, upstream timestamp, fetched_at.

### AuditEvent (append-only)

| Field       | Type     | Notes                                               |
|-------------|----------|-----------------------------------------------------|
| id          | TEXT PK  | uuidv7 (time-ordered)                               |
| ts          | TEXT     | RFC3339 UTC                                          |
| actor_id    | TEXT NULL| → User.id (null for anonymous/system)               |
| action      | TEXT     | `login`,`login_fail`,`logout`,`session_revoke`,`user_create`,`content_mutate`,`token_issue`,`token_revoke`,`perm_denied`,… |
| target      | TEXT NULL| affected resource id/type                            |
| ip          | TEXT NULL|                                                     |
| user_agent  | TEXT NULL|                                                     |
| result      | TEXT     | `success` \| `failure`                              |

Append-only by convention and enforced by the access layer (no update/delete paths exposed).

## Relationships Summary

```text
User 1─* Session
User 1─* ExternalIdentity
User 1─* PasskeyCredential
User 1─* ApiToken
User 1─* AuditEvent (actor)

Category 1─* Service        (Service.category_id, nullable)
Category 1─* Bookmark       (Bookmark.category_id, nullable)
Service  1─1 StatusReading  (current)
Service  1─* StatusHistory  (bounded uptime log)

Setting / Theme / WeatherReading : standalone
```

## Migration & integrity notes

- Forward-only SQL migrations under `migrations/`, applied on startup (idempotent).
- Foreign keys are enforced (`PRAGMA foreign_keys = ON`); WAL mode set at pool init.
- Visibility (`public`/`private`) is enforced in queries at the server-function boundary, not
  only in the UI: anonymous and unauthorized reads MUST exclude private rows in SQL.
- No legacy import is implied by this schema; a one-way importer from the old app (US9-style)
  would map into these tables but is not a parity contract.
