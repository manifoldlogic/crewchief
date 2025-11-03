-- 0000_schema_migrations.sql
-- Migration tracking table for idempotent migrations
-- This must be the first migration to run

-- Ensure the maproom schema exists first
CREATE SCHEMA IF NOT EXISTS maproom;

-- Create the schema_migrations table to track applied migrations
CREATE TABLE IF NOT EXISTS maproom.schema_migrations (
    version INTEGER PRIMARY KEY,
    filename TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add comment explaining the purpose
COMMENT ON TABLE maproom.schema_migrations IS
  'Tracks applied migrations for idempotency. Each row represents a successfully applied migration.';

COMMENT ON COLUMN maproom.schema_migrations.version IS
  'Migration version number (e.g., 1 for 0001_init.sql)';

COMMENT ON COLUMN maproom.schema_migrations.filename IS
  'Original migration filename for reference';

COMMENT ON COLUMN maproom.schema_migrations.applied_at IS
  'Timestamp when the migration was successfully applied';
