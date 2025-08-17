-- Web UI Sessions Table
-- Manages user sessions and authentication tokens for the web UI

CREATE TABLE IF NOT EXISTS web_sessions (
  id BIGSERIAL PRIMARY KEY,
  session_id UUID UNIQUE NOT NULL DEFAULT gen_random_uuid(),
  user_id TEXT, -- Optional user identifier for multi-user support
  auth_token TEXT UNIQUE NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_accessed TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  ip_address INET,
  user_agent TEXT,
  is_active BOOLEAN NOT NULL DEFAULT true,
  
  -- Session data as JSONB for flexibility
  session_data JSONB DEFAULT '{}',
  
  -- Constraints
  CONSTRAINT valid_expiry CHECK (expires_at > created_at)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_web_sessions_token ON web_sessions(auth_token);
CREATE INDEX IF NOT EXISTS idx_web_sessions_expires ON web_sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_web_sessions_user_active ON web_sessions(user_id, is_active) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_web_sessions_last_accessed ON web_sessions(last_accessed DESC);

-- Partial index for active sessions only
CREATE INDEX IF NOT EXISTS idx_web_sessions_active ON web_sessions(session_id, expires_at) 
WHERE is_active = true;

-- Automatic cleanup function for expired sessions
CREATE OR REPLACE FUNCTION cleanup_expired_sessions()
RETURNS void AS $$
BEGIN
  UPDATE web_sessions 
  SET is_active = false 
  WHERE expires_at < NOW() AND is_active = true;
  
  -- Delete sessions older than 30 days
  DELETE FROM web_sessions 
  WHERE created_at < NOW() - INTERVAL '30 days';
END;
$$ LANGUAGE plpgsql;

-- Comments for documentation
COMMENT ON TABLE web_sessions IS 'User sessions and authentication tokens for the CrewChief web UI';
COMMENT ON COLUMN web_sessions.session_id IS 'Unique session identifier';
COMMENT ON COLUMN web_sessions.user_id IS 'Optional user identifier for multi-user installations';
COMMENT ON COLUMN web_sessions.auth_token IS 'Authentication token for API access';
COMMENT ON COLUMN web_sessions.session_data IS 'Flexible session storage for user preferences and state';
COMMENT ON COLUMN web_sessions.expires_at IS 'Session expiration timestamp';
COMMENT ON COLUMN web_sessions.last_accessed IS 'Last time this session was used';