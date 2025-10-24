-- Context Assembly Cache
--
-- This migration creates a cache table for storing assembled context bundles
-- to improve performance by avoiding redundant graph traversals and assembly operations.
--
-- Key features:
-- - Composite key: (chunk_id, options_hash) uniquely identifies a cached bundle
-- - JSONB storage for flexible bundle structure
-- - TTL support via created_at timestamp for time-based eviction
-- - Access tracking for LRU eviction strategy
-- - Statistics fields for cache monitoring

CREATE TABLE IF NOT EXISTS maproom.context_cache (
  -- Primary cache key components
  chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  options_hash TEXT NOT NULL,

  -- Cached data
  bundle JSONB NOT NULL,

  -- Cache metadata
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  access_count INT NOT NULL DEFAULT 1,
  bundle_size_bytes INT NOT NULL,

  -- Composite primary key
  PRIMARY KEY (chunk_id, options_hash)
);

-- Index for TTL-based eviction (find oldest entries)
CREATE INDEX IF NOT EXISTS idx_context_cache_created_at
  ON maproom.context_cache(created_at);

-- Index for LRU eviction (find least recently used entries)
CREATE INDEX IF NOT EXISTS idx_context_cache_lru
  ON maproom.context_cache(last_accessed_at);

-- Index for cache statistics queries
CREATE INDEX IF NOT EXISTS idx_context_cache_access_count
  ON maproom.context_cache(access_count DESC);

-- Add comment explaining the table
COMMENT ON TABLE maproom.context_cache IS
  'Caches assembled context bundles to improve performance. ' ||
  'Key: (chunk_id, options_hash) where options_hash is SHA-256 of ExpandOptions. ' ||
  'Supports TTL and LRU eviction strategies.';

COMMENT ON COLUMN maproom.context_cache.chunk_id IS
  'ID of the primary chunk for which context was assembled';

COMMENT ON COLUMN maproom.context_cache.options_hash IS
  'SHA-256 hash of the ExpandOptions used for assembly';

COMMENT ON COLUMN maproom.context_cache.bundle IS
  'The assembled ContextBundle stored as JSONB';

COMMENT ON COLUMN maproom.context_cache.created_at IS
  'When this cache entry was created (for TTL-based eviction)';

COMMENT ON COLUMN maproom.context_cache.last_accessed_at IS
  'When this cache entry was last accessed (for LRU eviction)';

COMMENT ON COLUMN maproom.context_cache.access_count IS
  'How many times this cache entry has been accessed';

COMMENT ON COLUMN maproom.context_cache.bundle_size_bytes IS
  'Size of the serialized bundle in bytes (for memory monitoring)';

-- Cache statistics view for monitoring
CREATE OR REPLACE VIEW maproom.context_cache_stats AS
SELECT
  COUNT(*) as total_entries,
  SUM(bundle_size_bytes) as total_size_bytes,
  AVG(access_count) as avg_access_count,
  MAX(access_count) as max_access_count,
  MIN(created_at) as oldest_entry,
  MAX(created_at) as newest_entry,
  MIN(last_accessed_at) as least_recently_used,
  MAX(last_accessed_at) as most_recently_used,
  -- Count entries by age
  COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '1 hour') as entries_last_hour,
  COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '24 hours') as entries_last_day,
  COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '7 days') as entries_last_week
FROM maproom.context_cache;

COMMENT ON VIEW maproom.context_cache_stats IS
  'Aggregate statistics about the context cache for monitoring and optimization';

-- Function to clean up expired cache entries based on TTL
CREATE OR REPLACE FUNCTION maproom.evict_expired_cache_entries(ttl_seconds INT)
RETURNS TABLE(evicted_count BIGINT) AS $$
BEGIN
  WITH deleted AS (
    DELETE FROM maproom.context_cache
    WHERE created_at < NOW() - (ttl_seconds || ' seconds')::INTERVAL
    RETURNING *
  )
  SELECT COUNT(*) FROM deleted;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION maproom.evict_expired_cache_entries IS
  'Removes cache entries older than the specified TTL in seconds. ' ||
  'Returns the number of entries evicted.';

-- Function to evict least recently used entries when cache is full
CREATE OR REPLACE FUNCTION maproom.evict_lru_cache_entries(
  max_entries INT,
  evict_count INT DEFAULT 100
)
RETURNS TABLE(evicted_count BIGINT) AS $$
BEGIN
  WITH current_count AS (
    SELECT COUNT(*) as cnt FROM maproom.context_cache
  ),
  to_delete AS (
    SELECT chunk_id, options_hash
    FROM maproom.context_cache
    ORDER BY last_accessed_at ASC
    LIMIT CASE
      WHEN (SELECT cnt FROM current_count) > max_entries
      THEN LEAST(evict_count, (SELECT cnt FROM current_count) - max_entries + evict_count)
      ELSE 0
    END
  ),
  deleted AS (
    DELETE FROM maproom.context_cache
    WHERE (chunk_id, options_hash) IN (SELECT chunk_id, options_hash FROM to_delete)
    RETURNING *
  )
  SELECT COUNT(*) FROM deleted;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION maproom.evict_lru_cache_entries IS
  'Evicts the least recently used cache entries when total entries exceeds max_entries. ' ||
  'evict_count determines how many entries to remove at once (default: 100). ' ||
  'Returns the number of entries evicted.';

-- Function to invalidate cache entries for a specific chunk
-- (useful when a chunk is updated)
CREATE OR REPLACE FUNCTION maproom.invalidate_chunk_cache(target_chunk_id BIGINT)
RETURNS TABLE(invalidated_count BIGINT) AS $$
BEGIN
  WITH deleted AS (
    DELETE FROM maproom.context_cache
    WHERE chunk_id = target_chunk_id
    RETURNING *
  )
  SELECT COUNT(*) FROM deleted;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION maproom.invalidate_chunk_cache IS
  'Invalidates all cache entries for a specific chunk. ' ||
  'Used when a chunk is updated to ensure cache consistency. ' ||
  'Returns the number of entries invalidated.';

-- Function to clear the entire cache
CREATE OR REPLACE FUNCTION maproom.clear_context_cache()
RETURNS TABLE(cleared_count BIGINT) AS $$
BEGIN
  WITH deleted AS (
    DELETE FROM maproom.context_cache
    RETURNING *
  )
  SELECT COUNT(*) FROM deleted;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION maproom.clear_context_cache IS
  'Clears the entire context cache. ' ||
  'Useful for manual cache clearing or testing. ' ||
  'Returns the number of entries cleared.';
