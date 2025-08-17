-- Web UI Search History Table
-- Stores search queries, filters, and results for the Maproom search interface

CREATE TABLE IF NOT EXISTS web_search_history (
  id BIGSERIAL PRIMARY KEY,
  session_id UUID REFERENCES web_sessions(session_id) ON DELETE CASCADE,
  user_id TEXT, -- Optional for multi-user support
  
  -- Search details
  query TEXT NOT NULL,
  search_type TEXT NOT NULL DEFAULT 'semantic', -- 'semantic', 'fulltext', 'symbol', 'path'
  
  -- Filters applied (stored as JSONB for flexibility)
  filters JSONB DEFAULT '{}', -- { worktree, fileTypes, dateRange, language, etc }
  
  -- Results metadata
  result_count INTEGER NOT NULL DEFAULT 0,
  execution_time_ms INTEGER, -- Query execution time in milliseconds
  relevance_threshold REAL, -- Minimum relevance score used
  
  -- Top results summary (for quick access)
  top_results JSONB DEFAULT '[]', -- Array of top 10 results with basic info
  
  -- Performance metrics
  performance_metrics JSONB DEFAULT '{}', -- { indexUsed, scanType, cacheHit, etc }
  
  -- Timestamps
  searched_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  
  -- User interaction
  clicked_results INTEGER[] DEFAULT '{}', -- Array of result IDs that were clicked
  saved BOOLEAN DEFAULT false, -- Whether user saved this search
  
  -- Connection to maproom data
  repo_id BIGINT REFERENCES maproom.repos(id) ON DELETE SET NULL,
  worktree_id BIGINT REFERENCES maproom.worktrees(id) ON DELETE SET NULL,
  
  -- Constraints
  CONSTRAINT valid_search_type CHECK (search_type IN ('semantic', 'fulltext', 'symbol', 'path', 'hybrid'))
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_web_search_session ON web_search_history(session_id, searched_at DESC);
CREATE INDEX IF NOT EXISTS idx_web_search_user ON web_search_history(user_id, searched_at DESC) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_web_search_query_trgm ON web_search_history USING gin(query gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_web_search_repo_worktree ON web_search_history(repo_id, worktree_id, searched_at DESC);
CREATE INDEX IF NOT EXISTS idx_web_search_saved ON web_search_history(saved, searched_at DESC) WHERE saved = true;
CREATE INDEX IF NOT EXISTS idx_web_search_recent ON web_search_history(searched_at DESC);

-- Index on filters for common filter queries
CREATE INDEX IF NOT EXISTS idx_web_search_filters ON web_search_history USING gin(filters);

-- Function to get popular searches
CREATE OR REPLACE FUNCTION get_popular_searches(
  time_period INTERVAL DEFAULT INTERVAL '7 days',
  limit_count INTEGER DEFAULT 10
)
RETURNS TABLE(
  query TEXT,
  search_count BIGINT,
  avg_execution_time NUMERIC,
  avg_result_count NUMERIC
) AS $$
BEGIN
  RETURN QUERY
  SELECT 
    wsh.query,
    COUNT(*) as search_count,
    ROUND(AVG(wsh.execution_time_ms::NUMERIC), 2) as avg_execution_time,
    ROUND(AVG(wsh.result_count::NUMERIC), 2) as avg_result_count
  FROM web_search_history wsh
  WHERE wsh.searched_at > NOW() - time_period
  GROUP BY wsh.query
  HAVING COUNT(*) > 1
  ORDER BY search_count DESC, avg_result_count DESC
  LIMIT limit_count;
END;
$$ LANGUAGE plpgsql;

-- Function to clean old search history
CREATE OR REPLACE FUNCTION cleanup_old_search_history()
RETURNS void AS $$
BEGIN
  -- Keep saved searches longer (6 months)
  DELETE FROM web_search_history 
  WHERE saved = false AND searched_at < NOW() - INTERVAL '30 days';
  
  DELETE FROM web_search_history 
  WHERE saved = true AND searched_at < NOW() - INTERVAL '6 months';
END;
$$ LANGUAGE plpgsql;

-- Comments for documentation
COMMENT ON TABLE web_search_history IS 'Search history and analytics for the CrewChief web UI Maproom interface';
COMMENT ON COLUMN web_search_history.query IS 'The search query string';
COMMENT ON COLUMN web_search_history.search_type IS 'Type of search performed (semantic, fulltext, symbol, path, hybrid)';
COMMENT ON COLUMN web_search_history.filters IS 'Applied filters as JSON (worktree, fileTypes, dateRange, etc.)';
COMMENT ON COLUMN web_search_history.top_results IS 'Summary of top search results for quick access';
COMMENT ON COLUMN web_search_history.performance_metrics IS 'Query performance data for optimization';
COMMENT ON COLUMN web_search_history.clicked_results IS 'Array of result IDs that were clicked by the user';
COMMENT ON COLUMN web_search_history.saved IS 'Whether the user explicitly saved this search';