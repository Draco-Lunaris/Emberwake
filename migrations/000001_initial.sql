-- Emberwake initial schema migration
-- All 14 entities from data-model.md
-- UUIDv7 TEXT primary keys, UTC RFC3339 timestamps, WAL mode, foreign_keys=ON

-- Users
CREATE TABLE IF NOT EXISTS users (
    id            TEXT PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE COLLATE NOCASE,
    email         TEXT NULL UNIQUE,
    password_hash TEXT NULL,
    role          TEXT NOT NULL DEFAULT 'user',
    is_active     INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    last_login_at TEXT NULL
);

-- Sessions (server-side, opaque, revocable)
CREATE TABLE IF NOT EXISTS sessions (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at   TEXT NOT NULL,
    expires_at   TEXT NOT NULL,
    last_used_at TEXT NOT NULL,
    user_agent   TEXT NULL,
    ip           TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);

-- ExternalIdentity (OIDC)
CREATE TABLE IF NOT EXISTS external_identity (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider   TEXT NOT NULL,
    subject    TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(provider, subject)
);

-- PasskeyCredential (WebAuthn)
CREATE TABLE IF NOT EXISTS passkey_credential (
    id            TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id BLOB NOT NULL UNIQUE,
    public_key    BLOB NOT NULL,
    sign_count    INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL
);

-- ApiToken (scoped automation credential; only hash stored)
CREATE TABLE IF NOT EXISTS api_token (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    token_hash   TEXT NOT NULL,
    scopes       TEXT NOT NULL DEFAULT '[]',
    expires_at   TEXT NULL,
    revoked_at   TEXT NULL,
    created_at   TEXT NOT NULL,
    last_used_at TEXT NULL
);

-- Category
CREATE TABLE IF NOT EXISTS category (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    icon        TEXT NULL,
    order_index INTEGER NOT NULL DEFAULT 0,
    visibility  TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'private')),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- Service (launchable + optionally monitored tile)
CREATE TABLE IF NOT EXISTS service (
    id                  TEXT PRIMARY KEY,
    category_id         TEXT NULL REFERENCES category(id) ON DELETE SET NULL,
    name                TEXT NOT NULL,
    url                 TEXT NOT NULL,
    icon                TEXT NULL,
    description         TEXT NULL,
    is_pinned           INTEGER NOT NULL DEFAULT 0,
    order_index         INTEGER NOT NULL DEFAULT 0,
    visibility          TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'private')),
    monitor_enabled     INTEGER NOT NULL DEFAULT 0,
    monitor_kind        TEXT NULL CHECK (monitor_kind IS NULL OR monitor_kind IN ('http', 'tcp')),
    monitor_target      TEXT NULL,
    monitor_interval_s  INTEGER NULL,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

-- Bookmark
CREATE TABLE IF NOT EXISTS bookmark (
    id          TEXT PRIMARY KEY,
    category_id TEXT NULL REFERENCES category(id) ON DELETE SET NULL,
    name        TEXT NOT NULL,
    url         TEXT NOT NULL,
    icon        TEXT NULL,
    order_index INTEGER NOT NULL DEFAULT 0,
    visibility  TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'private')),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- Setting (typed key/value store; values are JSON)
-- The setup_complete key is a singleton: its PK uniqueness ensures race-safe first-run admin setup.
CREATE TABLE IF NOT EXISTS setting (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Theme
CREATE TABLE IF NOT EXISTS theme (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    tokens     TEXT NOT NULL,
    custom_css TEXT NULL,
    is_builtin INTEGER NOT NULL DEFAULT 0,
    created_by TEXT NULL REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL
);

-- StatusReading (latest health per monitored service; one current row per service)
CREATE TABLE IF NOT EXISTS status_reading (
    service_id  TEXT PRIMARY KEY REFERENCES service(id) ON DELETE CASCADE,
    state       TEXT NOT NULL CHECK (state IN ('up', 'down', 'degraded')),
    latency_ms  INTEGER NULL,
    reason      TEXT NULL,
    checked_at  TEXT NOT NULL
);

-- StatusHistory (bounded uptime log; retention-limited)
CREATE TABLE IF NOT EXISTS status_history (
    id          TEXT PRIMARY KEY,
    service_id  TEXT NOT NULL REFERENCES service(id) ON DELETE CASCADE,
    state       TEXT NOT NULL CHECK (state IN ('up', 'down', 'degraded')),
    latency_ms  INTEGER NULL,
    reason      TEXT NULL,
    checked_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_status_history_service_checked ON status_history(service_id, checked_at);

-- WeatherReading (single-row latest cache)
CREATE TABLE IF NOT EXISTS weather_reading (
    id          INTEGER PRIMARY KEY DEFAULT 1,
    temp        REAL NULL,
    condition   TEXT NULL,
    is_day      INTEGER NULL,
    cloud       INTEGER NULL,
    upstream_ts TEXT NULL,
    fetched_at  TEXT NOT NULL,
    CHECK (id = 1)
);

-- AuditEvent (append-only; no update/delete paths exposed)
CREATE TABLE IF NOT EXISTS audit_event (
    id         TEXT PRIMARY KEY,
    ts         TEXT NOT NULL,
    actor_id   TEXT NULL REFERENCES users(id) ON DELETE SET NULL,
    action     TEXT NOT NULL,
    target     TEXT NULL,
    ip         TEXT NULL,
    user_agent TEXT NULL,
    result     TEXT NOT NULL CHECK (result IN ('success', 'failure'))
);

CREATE INDEX IF NOT EXISTS idx_audit_event_ts ON audit_event(ts);
CREATE INDEX IF NOT EXISTS idx_audit_event_actor ON audit_event(actor_id);
