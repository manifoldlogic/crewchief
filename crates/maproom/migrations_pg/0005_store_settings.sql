-- F48: per-database settings. Holds the sticky don't-store-content minimization
-- marker (key 'minimize_content') so scan/incremental/import all honor it and a
-- later normal-mode scan cannot silently re-add content. Mirrors the SQLite
-- store_settings table (migration v13).
CREATE TABLE store_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
