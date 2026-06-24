-- Add 'restricted' visibility level to CHECK constraints on category, service, and bookmark tables.
-- SQLite cannot ALTER CHECK constraints in-place, so we use the table rebuild pattern.

-- category
CREATE TABLE IF NOT EXISTS category_new (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    icon        TEXT NULL,
    order_index INTEGER NOT NULL DEFAULT 0,
    visibility  TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'private', 'restricted')),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

INSERT INTO category_new (id, name, icon, order_index, visibility, created_at, updated_at)
SELECT id, name, icon, order_index, visibility, created_at, updated_at FROM category;

DROP TABLE category;
ALTER TABLE category_new RENAME TO category;

-- service
CREATE TABLE IF NOT EXISTS service_new (
    id                  TEXT PRIMARY KEY,
    category_id         TEXT NULL REFERENCES category(id) ON DELETE SET NULL,
    name                TEXT NOT NULL,
    url                 TEXT NOT NULL,
    icon                TEXT NULL,
    description         TEXT NULL,
    is_pinned           INTEGER NOT NULL DEFAULT 0,
    order_index         INTEGER NOT NULL DEFAULT 0,
    visibility          TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'private', 'restricted')),
    monitor_enabled     INTEGER NOT NULL DEFAULT 0,
    monitor_kind        TEXT NULL CHECK (monitor_kind IS NULL OR monitor_kind IN ('http', 'tcp')),
    monitor_target      TEXT NULL,
    monitor_interval_s  INTEGER NULL,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

INSERT INTO service_new (id, category_id, name, url, icon, description, is_pinned, order_index, visibility,
                         monitor_enabled, monitor_kind, monitor_target, monitor_interval_s, created_at, updated_at)
SELECT id, category_id, name, url, icon, description, is_pinned, order_index, visibility,
       monitor_enabled, monitor_kind, monitor_target, monitor_interval_s, created_at, updated_at
FROM service;

DROP TABLE service;
ALTER TABLE service_new RENAME TO service;

-- bookmark
CREATE TABLE IF NOT EXISTS bookmark_new (
    id          TEXT PRIMARY KEY,
    category_id TEXT NULL REFERENCES category(id) ON DELETE SET NULL,
    name        TEXT NOT NULL,
    url         TEXT NOT NULL,
    icon        TEXT NULL,
    order_index INTEGER NOT NULL DEFAULT 0,
    visibility  TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'private', 'restricted')),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

INSERT INTO bookmark_new (id, category_id, name, url, icon, order_index, visibility, created_at, updated_at)
SELECT id, category_id, name, url, icon, order_index, visibility, created_at, updated_at FROM bookmark;

DROP TABLE bookmark;
ALTER TABLE bookmark_new RENAME TO bookmark;
