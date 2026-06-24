-- Add Application entity (launchable tiles without monitoring)
CREATE TABLE IF NOT EXISTS application (
    id TEXT PRIMARY KEY NOT NULL,
    category_id TEXT REFERENCES category(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    icon TEXT,
    description TEXT,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    order_index INTEGER NOT NULL DEFAULT 0,
    visibility TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'private', 'restricted')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_application_category_id ON application(category_id);
CREATE INDEX IF NOT EXISTS idx_application_order ON application(order_index);
CREATE INDEX IF NOT EXISTS idx_application_visibility ON application(visibility);
