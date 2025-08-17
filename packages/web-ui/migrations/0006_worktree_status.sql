-- Worktree Status Cache Table
-- Caches git status, file system state, and other worktree metadata for performance

-- Create enum types for worktree status
DO $$ BEGIN
  CREATE TYPE worktree_state AS ENUM ('active', 'stale', 'merging', 'archived', 'error');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
  CREATE TYPE git_file_status AS ENUM (
    'unmodified', 'modified', 'added', 'deleted', 'renamed', 
    'copied', 'unmerged', 'untracked', 'ignored'
  );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

CREATE TABLE IF NOT EXISTS worktree_status (
  id BIGSERIAL PRIMARY KEY,
  
  -- Worktree identification (linked to maproom)
  worktree_id BIGINT UNIQUE NOT NULL REFERENCES maproom.worktrees(id) ON DELETE CASCADE,
  repo_id BIGINT NOT NULL REFERENCES maproom.repos(id) ON DELETE CASCADE,
  
  -- Basic worktree info
  worktree_name TEXT NOT NULL,
  worktree_path TEXT NOT NULL,
  current_branch TEXT NOT NULL,
  upstream_branch TEXT,
  
  -- Worktree state
  state worktree_state NOT NULL DEFAULT 'active',
  is_clean BOOLEAN NOT NULL DEFAULT true, -- No uncommitted changes
  is_synced BOOLEAN NOT NULL DEFAULT true, -- Synced with upstream
  
  -- Git status summary
  head_commit_sha TEXT NOT NULL,
  head_commit_message TEXT,
  head_commit_author TEXT,
  head_commit_date TIMESTAMPTZ,
  
  -- Tracking information
  commits_ahead INTEGER DEFAULT 0,
  commits_behind INTEGER DEFAULT 0,
  
  -- File change summary
  modified_files INTEGER DEFAULT 0,
  added_files INTEGER DEFAULT 0,
  deleted_files INTEGER DEFAULT 0,
  untracked_files INTEGER DEFAULT 0,
  staged_files INTEGER DEFAULT 0,
  
  -- Detailed file status (as JSONB for flexibility)
  file_changes JSONB DEFAULT '[]', -- Array of {path, status, staged}
  
  -- Directory analysis
  total_files INTEGER,
  total_size_bytes BIGINT,
  programming_languages JSONB DEFAULT '{}', -- Language distribution
  
  -- Active agents and processes
  active_agents JSONB DEFAULT '[]', -- Array of agent info
  tmux_sessions TEXT[] DEFAULT '{}', -- Active tmux sessions
  
  -- Performance and health metrics
  disk_usage_bytes BIGINT,
  last_build_status TEXT, -- 'success', 'failed', 'pending', 'unknown'
  last_build_time TIMESTAMPTZ,
  test_status TEXT, -- 'passing', 'failing', 'pending', 'unknown'
  test_coverage REAL, -- Test coverage percentage
  
  -- Indexing status
  maproom_indexed_at TIMESTAMPTZ,
  maproom_index_status TEXT DEFAULT 'unknown', -- 'current', 'stale', 'indexing', 'error'
  chunk_count INTEGER, -- Number of indexed chunks
  
  -- Cache metadata
  last_scan_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  scan_duration_ms INTEGER, -- Time taken for the last scan
  cache_version INTEGER DEFAULT 1, -- For cache invalidation
  
  -- Error tracking
  last_error TEXT,
  error_count INTEGER DEFAULT 0,
  
  -- User interaction
  last_accessed_at TIMESTAMPTZ, -- When user last viewed this worktree
  pinned BOOLEAN DEFAULT false, -- User pinned this worktree
  tags TEXT[] DEFAULT '{}', -- User-defined tags
  notes TEXT, -- User notes about this worktree
  
  -- Timestamps
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  
  -- Constraints
  CONSTRAINT valid_commits_count CHECK (commits_ahead >= 0 AND commits_behind >= 0),
  CONSTRAINT valid_file_counts CHECK (
    modified_files >= 0 AND added_files >= 0 AND 
    deleted_files >= 0 AND untracked_files >= 0 AND staged_files >= 0
  ),
  CONSTRAINT valid_test_coverage CHECK (test_coverage IS NULL OR (test_coverage >= 0 AND test_coverage <= 100))
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_worktree_status_repo ON worktree_status(repo_id, state);
CREATE INDEX IF NOT EXISTS idx_worktree_status_state ON worktree_status(state, last_scan_at DESC);
CREATE INDEX IF NOT EXISTS idx_worktree_status_branch ON worktree_status(current_branch);
CREATE INDEX IF NOT EXISTS idx_worktree_status_clean ON worktree_status(is_clean, is_synced);
CREATE INDEX IF NOT EXISTS idx_worktree_status_active_agents ON worktree_status USING gin(active_agents) WHERE jsonb_array_length(active_agents) > 0;
CREATE INDEX IF NOT EXISTS idx_worktree_status_last_accessed ON worktree_status(last_accessed_at DESC) WHERE last_accessed_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_worktree_status_pinned ON worktree_status(pinned, updated_at DESC) WHERE pinned = true;
CREATE INDEX IF NOT EXISTS idx_worktree_status_stale ON worktree_status(last_scan_at) WHERE state = 'stale';

-- GIN indexes for JSONB columns
CREATE INDEX IF NOT EXISTS idx_worktree_status_file_changes ON worktree_status USING gin(file_changes);
CREATE INDEX IF NOT EXISTS idx_worktree_status_languages ON worktree_status USING gin(programming_languages);
CREATE INDEX IF NOT EXISTS idx_worktree_status_tags ON worktree_status USING gin(tags);

-- Function to update worktree status
CREATE OR REPLACE FUNCTION update_worktree_status(
  p_worktree_id BIGINT,
  p_git_status JSONB,
  p_file_summary JSONB DEFAULT NULL,
  p_agent_info JSONB DEFAULT NULL
)
RETURNS void AS $$
DECLARE
  scan_start TIMESTAMPTZ := NOW();
  scan_duration INTEGER;
BEGIN
  scan_duration := EXTRACT(EPOCH FROM (NOW() - scan_start)) * 1000;
  
  INSERT INTO worktree_status (
    worktree_id,
    repo_id,
    worktree_name,
    worktree_path,
    current_branch,
    upstream_branch,
    is_clean,
    is_synced,
    head_commit_sha,
    head_commit_message,
    head_commit_author,
    head_commit_date,
    commits_ahead,
    commits_behind,
    modified_files,
    added_files,
    deleted_files,
    untracked_files,
    staged_files,
    file_changes,
    active_agents,
    scan_duration_ms,
    updated_at
  )
  SELECT 
    p_worktree_id,
    w.repo_id,
    w.name,
    w.abs_path,
    p_git_status->>'branch',
    p_git_status->>'upstream',
    (p_git_status->>'clean')::boolean,
    (p_git_status->>'synced')::boolean,
    p_git_status->>'head_sha',
    p_git_status->>'head_message',
    p_git_status->>'head_author',
    (p_git_status->>'head_date')::timestamptz,
    COALESCE((p_git_status->>'ahead')::integer, 0),
    COALESCE((p_git_status->>'behind')::integer, 0),
    COALESCE((p_git_status->>'modified')::integer, 0),
    COALESCE((p_git_status->>'added')::integer, 0),
    COALESCE((p_git_status->>'deleted')::integer, 0),
    COALESCE((p_git_status->>'untracked')::integer, 0),
    COALESCE((p_git_status->>'staged')::integer, 0),
    COALESCE(p_git_status->'files', '[]'::jsonb),
    COALESCE(p_agent_info, '[]'::jsonb),
    scan_duration,
    NOW()
  FROM maproom.worktrees w
  WHERE w.id = p_worktree_id
  
  ON CONFLICT (worktree_id)
  DO UPDATE SET
    current_branch = EXCLUDED.current_branch,
    upstream_branch = EXCLUDED.upstream_branch,
    is_clean = EXCLUDED.is_clean,
    is_synced = EXCLUDED.is_synced,
    head_commit_sha = EXCLUDED.head_commit_sha,
    head_commit_message = EXCLUDED.head_commit_message,
    head_commit_author = EXCLUDED.head_commit_author,
    head_commit_date = EXCLUDED.head_commit_date,
    commits_ahead = EXCLUDED.commits_ahead,
    commits_behind = EXCLUDED.commits_behind,
    modified_files = EXCLUDED.modified_files,
    added_files = EXCLUDED.added_files,
    deleted_files = EXCLUDED.deleted_files,
    untracked_files = EXCLUDED.untracked_files,
    staged_files = EXCLUDED.staged_files,
    file_changes = EXCLUDED.file_changes,
    active_agents = EXCLUDED.active_agents,
    last_scan_at = NOW(),
    scan_duration_ms = EXCLUDED.scan_duration_ms,
    updated_at = NOW(),
    cache_version = worktree_status.cache_version + 1;
END;
$$ LANGUAGE plpgsql;

-- Function to mark worktree as accessed
CREATE OR REPLACE FUNCTION mark_worktree_accessed(p_worktree_id BIGINT)
RETURNS void AS $$
BEGIN
  UPDATE worktree_status 
  SET last_accessed_at = NOW()
  WHERE worktree_id = p_worktree_id;
END;
$$ LANGUAGE plpgsql;

-- Function to get worktree dashboard summary
CREATE OR REPLACE FUNCTION get_worktree_summary(
  p_repo_id BIGINT DEFAULT NULL,
  include_archived BOOLEAN DEFAULT false
)
RETURNS TABLE(
  worktree_id BIGINT,
  worktree_name TEXT,
  current_branch TEXT,
  state worktree_state,
  is_clean BOOLEAN,
  active_agent_count INTEGER,
  commits_ahead INTEGER,
  commits_behind INTEGER,
  total_changes INTEGER,
  last_scan_at TIMESTAMPTZ,
  last_accessed_at TIMESTAMPTZ
) AS $$
BEGIN
  RETURN QUERY
  SELECT 
    ws.worktree_id,
    ws.worktree_name,
    ws.current_branch,
    ws.state,
    ws.is_clean,
    jsonb_array_length(ws.active_agents) as active_agent_count,
    ws.commits_ahead,
    ws.commits_behind,
    (ws.modified_files + ws.added_files + ws.deleted_files + ws.untracked_files) as total_changes,
    ws.last_scan_at,
    ws.last_accessed_at
  FROM worktree_status ws
  WHERE (p_repo_id IS NULL OR ws.repo_id = p_repo_id)
    AND (include_archived = true OR ws.state != 'archived')
  ORDER BY 
    ws.pinned DESC,
    ws.last_accessed_at DESC NULLS LAST,
    ws.updated_at DESC;
END;
$$ LANGUAGE plpgsql;

-- Function to detect stale worktrees
CREATE OR REPLACE FUNCTION mark_stale_worktrees()
RETURNS void AS $$
BEGIN
  UPDATE worktree_status 
  SET state = 'stale'
  WHERE last_scan_at < NOW() - INTERVAL '1 hour'
    AND state = 'active';
END;
$$ LANGUAGE plpgsql;

-- Function to cleanup old cache entries
CREATE OR REPLACE FUNCTION cleanup_worktree_status_cache()
RETURNS void AS $$
BEGIN
  -- Remove cache entries for deleted worktrees
  DELETE FROM worktree_status ws
  WHERE NOT EXISTS (
    SELECT 1 FROM maproom.worktrees w 
    WHERE w.id = ws.worktree_id
  );
  
  -- Archive old inactive worktrees
  UPDATE worktree_status 
  SET state = 'archived'
  WHERE state = 'stale'
    AND last_accessed_at < NOW() - INTERVAL '30 days'
    AND pinned = false;
END;
$$ LANGUAGE plpgsql;

-- Trigger to auto-update timestamps
CREATE OR REPLACE FUNCTION update_worktree_status_timestamps()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  
  -- Determine state based on various factors
  IF NEW.error_count > 5 THEN
    NEW.state = 'error';
  ELSIF NEW.last_scan_at < NOW() - INTERVAL '1 hour' THEN
    NEW.state = 'stale';
  ELSIF jsonb_array_length(NEW.active_agents) > 0 THEN
    NEW.state = 'active';
  END IF;
  
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_worktree_status_timestamps
  BEFORE UPDATE ON worktree_status
  FOR EACH ROW
  EXECUTE FUNCTION update_worktree_status_timestamps();

-- Comments for documentation
COMMENT ON TABLE worktree_status IS 'Cached git status and metadata for worktrees to improve UI performance';
COMMENT ON COLUMN worktree_status.worktree_id IS 'Reference to the maproom worktree';
COMMENT ON COLUMN worktree_status.state IS 'Current state of the worktree (active, stale, merging, archived, error)';
COMMENT ON COLUMN worktree_status.is_clean IS 'Whether the worktree has uncommitted changes';
COMMENT ON COLUMN worktree_status.is_synced IS 'Whether the worktree is synced with upstream';
COMMENT ON COLUMN worktree_status.file_changes IS 'Detailed list of file changes with status';
COMMENT ON COLUMN worktree_status.active_agents IS 'JSON array of currently active agents in this worktree';
COMMENT ON COLUMN worktree_status.programming_languages IS 'Distribution of programming languages in the worktree';
COMMENT ON COLUMN worktree_status.cache_version IS 'Version number for cache invalidation strategies';
COMMENT ON COLUMN worktree_status.last_scan_at IS 'When this status was last updated from the filesystem';