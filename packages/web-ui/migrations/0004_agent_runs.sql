-- Agent Runs Table
-- Tracks agent execution history, performance metrics, and outcomes

-- Create enum types for agent runs
DO $$ BEGIN
  CREATE TYPE agent_status AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled', 'timeout');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
  CREATE TYPE agent_type AS ENUM ('claude', 'gemini', 'mock', 'custom');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

CREATE TABLE IF NOT EXISTS agent_runs (
  id BIGSERIAL PRIMARY KEY,
  
  -- Agent identification
  agent_id TEXT NOT NULL, -- Agent instance identifier
  agent_type agent_type NOT NULL,
  
  -- Run context
  run_id UUID UNIQUE NOT NULL DEFAULT gen_random_uuid(),
  parent_run_id UUID REFERENCES agent_runs(run_id) ON DELETE CASCADE, -- For nested/child runs
  
  -- Repository context (linked to maproom tables)
  repo_id BIGINT REFERENCES maproom.repos(id) ON DELETE CASCADE,
  worktree_id BIGINT REFERENCES maproom.worktrees(id) ON DELETE CASCADE,
  commit_sha TEXT, -- Commit when run started
  
  -- Task details
  task_description TEXT NOT NULL,
  task_type TEXT, -- e.g., 'code_review', 'bug_fix', 'feature', 'test'
  instructions JSONB, -- Structured task instructions
  context_files TEXT[], -- Array of file paths that provide context
  
  -- Execution details
  status agent_status NOT NULL DEFAULT 'pending',
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  duration_ms BIGINT, -- Computed duration in milliseconds
  
  -- Tmux integration
  tmux_session TEXT,
  tmux_window INTEGER,
  tmux_pane INTEGER,
  
  -- Results and artifacts
  exit_code INTEGER,
  error_message TEXT,
  artifacts JSONB DEFAULT '{}', -- Files created, modified, deleted, commits made
  
  -- Evaluation and quality metrics
  evaluation_score REAL, -- 0.0 to 1.0 quality score
  tests_passed BOOLEAN,
  review_required BOOLEAN DEFAULT true,
  auto_merge_eligible BOOLEAN DEFAULT false,
  
  -- Performance metrics
  cpu_usage_avg REAL, -- Average CPU usage percentage
  memory_usage_peak BIGINT, -- Peak memory usage in bytes
  disk_io_bytes BIGINT, -- Total disk I/O in bytes
  network_requests INTEGER, -- Number of network requests made
  
  -- Output and logging
  stdout_log_path TEXT, -- Path to stdout log file
  stderr_log_path TEXT, -- Path to stderr log file
  log_summary TEXT, -- Brief summary of key log events
  
  -- Metadata
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  
  -- Competition and comparison
  competition_id UUID, -- Groups runs that are competing on the same task
  competition_rank INTEGER, -- Rank within competition (1 = best)
  
  -- User interaction
  user_feedback JSONB, -- User ratings and feedback
  bookmarked BOOLEAN DEFAULT false,
  tags TEXT[] DEFAULT '{}', -- User-defined tags
  
  -- Constraints
  CONSTRAINT valid_duration CHECK (
    (completed_at IS NULL AND duration_ms IS NULL) OR 
    (completed_at IS NOT NULL AND started_at IS NOT NULL AND duration_ms IS NOT NULL)
  ),
  CONSTRAINT valid_evaluation_score CHECK (evaluation_score IS NULL OR (evaluation_score >= 0.0 AND evaluation_score <= 1.0)),
  CONSTRAINT valid_competition_rank CHECK (competition_rank IS NULL OR competition_rank > 0)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_agent_runs_agent_id ON agent_runs(agent_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_runs_status ON agent_runs(status, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_runs_worktree ON agent_runs(worktree_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_runs_repo ON agent_runs(repo_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_runs_competition ON agent_runs(competition_id, competition_rank) WHERE competition_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_agent_runs_parent ON agent_runs(parent_run_id) WHERE parent_run_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_agent_runs_task_type ON agent_runs(task_type, started_at DESC) WHERE task_type IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_agent_runs_evaluation ON agent_runs(evaluation_score DESC) WHERE evaluation_score IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_agent_runs_bookmarked ON agent_runs(bookmarked, started_at DESC) WHERE bookmarked = true;

-- GIN indexes for JSONB and array columns
CREATE INDEX IF NOT EXISTS idx_agent_runs_artifacts ON agent_runs USING gin(artifacts);
CREATE INDEX IF NOT EXISTS idx_agent_runs_instructions ON agent_runs USING gin(instructions);
CREATE INDEX IF NOT EXISTS idx_agent_runs_tags ON agent_runs USING gin(tags);
CREATE INDEX IF NOT EXISTS idx_agent_runs_user_feedback ON agent_runs USING gin(user_feedback);

-- Full-text search on task description and log summary
CREATE INDEX IF NOT EXISTS idx_agent_runs_task_search ON agent_runs USING gin(to_tsvector('english', task_description));
CREATE INDEX IF NOT EXISTS idx_agent_runs_log_search ON agent_runs USING gin(to_tsvector('english', log_summary)) WHERE log_summary IS NOT NULL;

-- Function to update duration when run completes
CREATE OR REPLACE FUNCTION update_run_duration()
RETURNS TRIGGER AS $$
BEGIN
  IF NEW.completed_at IS NOT NULL AND NEW.started_at IS NOT NULL THEN
    NEW.duration_ms = EXTRACT(EPOCH FROM (NEW.completed_at - NEW.started_at)) * 1000;
  END IF;
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_agent_runs_duration
  BEFORE UPDATE ON agent_runs
  FOR EACH ROW
  EXECUTE FUNCTION update_run_duration();

-- Function to get run statistics
CREATE OR REPLACE FUNCTION get_run_statistics(
  days_back INTEGER DEFAULT 7,
  agent_type_filter agent_type DEFAULT NULL,
  worktree_filter BIGINT DEFAULT NULL
)
RETURNS TABLE(
  total_runs BIGINT,
  successful_runs BIGINT,
  failed_runs BIGINT,
  avg_duration_seconds NUMERIC,
  avg_evaluation_score NUMERIC,
  top_performing_agent TEXT
) AS $$
BEGIN
  RETURN QUERY
  SELECT 
    COUNT(*) as total_runs,
    COUNT(*) FILTER (WHERE status = 'completed') as successful_runs,
    COUNT(*) FILTER (WHERE status = 'failed') as failed_runs,
    ROUND(AVG(duration_ms::NUMERIC) / 1000, 2) as avg_duration_seconds,
    ROUND(AVG(evaluation_score::NUMERIC), 3) as avg_evaluation_score,
    (
      SELECT agent_id 
      FROM agent_runs ar2 
      WHERE ar2.started_at > NOW() - INTERVAL '1 day' * days_back
        AND (agent_type_filter IS NULL OR ar2.agent_type = agent_type_filter)
        AND (worktree_filter IS NULL OR ar2.worktree_id = worktree_filter)
        AND ar2.evaluation_score IS NOT NULL
      GROUP BY agent_id 
      ORDER BY AVG(evaluation_score) DESC 
      LIMIT 1
    ) as top_performing_agent
  FROM agent_runs ar
  WHERE ar.started_at > NOW() - INTERVAL '1 day' * days_back
    AND (agent_type_filter IS NULL OR ar.agent_type = agent_type_filter)
    AND (worktree_filter IS NULL OR ar.worktree_id = worktree_filter);
END;
$$ LANGUAGE plpgsql;

-- Function to get recent runs with context
CREATE OR REPLACE FUNCTION get_recent_runs(
  limit_count INTEGER DEFAULT 50,
  status_filter agent_status DEFAULT NULL
)
RETURNS TABLE(
  run_id UUID,
  agent_id TEXT,
  agent_type agent_type,
  task_description TEXT,
  status agent_status,
  started_at TIMESTAMPTZ,
  duration_seconds NUMERIC,
  evaluation_score REAL,
  repo_name TEXT,
  worktree_name TEXT
) AS $$
BEGIN
  RETURN QUERY
  SELECT 
    ar.run_id,
    ar.agent_id,
    ar.agent_type,
    ar.task_description,
    ar.status,
    ar.started_at,
    ROUND((ar.duration_ms::NUMERIC) / 1000, 2) as duration_seconds,
    ar.evaluation_score,
    r.name as repo_name,
    w.name as worktree_name
  FROM agent_runs ar
  LEFT JOIN maproom.repos r ON ar.repo_id = r.id
  LEFT JOIN maproom.worktrees w ON ar.worktree_id = w.id
  WHERE (status_filter IS NULL OR ar.status = status_filter)
  ORDER BY ar.started_at DESC
  LIMIT limit_count;
END;
$$ LANGUAGE plpgsql;

-- Function to cleanup old runs
CREATE OR REPLACE FUNCTION cleanup_old_agent_runs()
RETURNS void AS $$
BEGIN
  -- Archive successful runs older than 3 months (keep metadata, remove logs)
  UPDATE agent_runs 
  SET 
    stdout_log_path = NULL,
    stderr_log_path = NULL,
    log_summary = CASE 
      WHEN log_summary IS NOT NULL 
      THEN LEFT(log_summary, 200) || '... [archived]'
      ELSE NULL 
    END
  WHERE started_at < NOW() - INTERVAL '3 months' 
    AND status = 'completed'
    AND stdout_log_path IS NOT NULL;
    
  -- Delete failed runs older than 1 month
  DELETE FROM agent_runs 
  WHERE started_at < NOW() - INTERVAL '1 month' 
    AND status IN ('failed', 'cancelled', 'timeout')
    AND bookmarked = false;
END;
$$ LANGUAGE plpgsql;

-- Comments for documentation
COMMENT ON TABLE agent_runs IS 'Agent execution history, performance tracking, and run artifacts';
COMMENT ON COLUMN agent_runs.agent_id IS 'Unique identifier for the agent instance';
COMMENT ON COLUMN agent_runs.run_id IS 'Unique identifier for this specific run';
COMMENT ON COLUMN agent_runs.parent_run_id IS 'Reference to parent run for nested/child runs';
COMMENT ON COLUMN agent_runs.task_description IS 'Human-readable description of the task';
COMMENT ON COLUMN agent_runs.instructions IS 'Structured task instructions and parameters';
COMMENT ON COLUMN agent_runs.context_files IS 'Array of file paths providing context for the task';
COMMENT ON COLUMN agent_runs.artifacts IS 'JSON object describing files created/modified and commits made';
COMMENT ON COLUMN agent_runs.evaluation_score IS 'Quality score from 0.0 to 1.0';
COMMENT ON COLUMN agent_runs.competition_id IS 'Groups runs competing on the same task';
COMMENT ON COLUMN agent_runs.competition_rank IS 'Rank within competition (1 = best performance)';