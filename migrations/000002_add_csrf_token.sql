-- Add csrf_token column to sessions table for per-session CSRF protection
ALTER TABLE sessions ADD COLUMN csrf_token TEXT NOT NULL DEFAULT '';
