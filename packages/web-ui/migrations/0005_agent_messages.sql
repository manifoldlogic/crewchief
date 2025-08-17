-- Agent Messages Table
-- Stores inter-agent communication logs and message bus events

-- Create enum types for messages
DO $$ BEGIN
  CREATE TYPE message_type AS ENUM (
    'command', 'response', 'notification', 'error', 'log', 
    'status_update', 'file_change', 'git_event', 'system_event'
  );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
  CREATE TYPE message_priority AS ENUM ('low', 'normal', 'high', 'critical');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

CREATE TABLE IF NOT EXISTS agent_messages (
  id BIGSERIAL PRIMARY KEY,
  
  -- Message identification
  message_id UUID UNIQUE NOT NULL DEFAULT gen_random_uuid(),
  correlation_id UUID, -- Groups related messages together
  reply_to_id UUID REFERENCES agent_messages(message_id) ON DELETE SET NULL,
  
  -- Agent context
  run_id UUID REFERENCES agent_runs(run_id) ON DELETE CASCADE,
  sender_agent_id TEXT, -- Agent that sent the message
  recipient_agent_id TEXT, -- Specific recipient (NULL for broadcast)
  
  -- Repository context
  repo_id BIGINT REFERENCES maproom.repos(id) ON DELETE SET NULL,
  worktree_id BIGINT REFERENCES maproom.worktrees(id) ON DELETE SET NULL,
  
  -- Message content
  message_type message_type NOT NULL,
  priority message_priority NOT NULL DEFAULT 'normal',
  subject TEXT, -- Brief message subject/title
  content TEXT NOT NULL, -- Message body
  content_format TEXT DEFAULT 'text', -- 'text', 'json', 'markdown', 'xml'
  
  -- Structured data
  metadata JSONB DEFAULT '{}', -- Additional structured message data
  attachments JSONB DEFAULT '[]', -- File attachments, links, etc.
  
  -- Message routing and delivery
  broadcast BOOLEAN DEFAULT false, -- Whether this is a broadcast message
  delivered_at TIMESTAMPTZ, -- When message was delivered/processed
  acknowledged_at TIMESTAMPTZ, -- When recipient acknowledged receipt
  
  -- Timing
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ, -- Optional message expiration
  
  -- Processing status
  processed BOOLEAN DEFAULT false, -- Whether the message was processed
  processing_result JSONB, -- Result of processing the message
  retry_count INTEGER DEFAULT 0, -- Number of delivery retries
  max_retries INTEGER DEFAULT 3, -- Maximum retry attempts
  
  -- Message bus integration
  bus_topic TEXT, -- Message bus topic/channel
  bus_partition INTEGER, -- Message bus partition
  bus_offset BIGINT, -- Message bus offset
  
  -- Filtering and search
  tags TEXT[] DEFAULT '{}', -- Message tags for categorization
  search_vector TSVECTOR, -- Full-text search vector
  
  -- Performance tracking
  size_bytes INTEGER, -- Message size in bytes
  processing_time_ms INTEGER, -- Time taken to process message
  
  -- Constraints
  CONSTRAINT valid_expiry CHECK (expires_at IS NULL OR expires_at > created_at),
  CONSTRAINT valid_retry_count CHECK (retry_count >= 0 AND retry_count <= max_retries),
  CONSTRAINT valid_content_format CHECK (content_format IN ('text', 'json', 'markdown', 'xml', 'binary'))
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_agent_messages_run ON agent_messages(run_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_messages_sender ON agent_messages(sender_agent_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_messages_recipient ON agent_messages(recipient_agent_id, created_at DESC) WHERE recipient_agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_agent_messages_type ON agent_messages(message_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_messages_priority ON agent_messages(priority, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_messages_correlation ON agent_messages(correlation_id, created_at) WHERE correlation_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_agent_messages_reply_to ON agent_messages(reply_to_id) WHERE reply_to_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_agent_messages_unprocessed ON agent_messages(processed, priority, created_at) WHERE processed = false;
CREATE INDEX IF NOT EXISTS idx_agent_messages_broadcast ON agent_messages(broadcast, created_at DESC) WHERE broadcast = true;
CREATE INDEX IF NOT EXISTS idx_agent_messages_bus_topic ON agent_messages(bus_topic, bus_offset) WHERE bus_topic IS NOT NULL;

-- GIN indexes for JSONB and array columns
CREATE INDEX IF NOT EXISTS idx_agent_messages_metadata ON agent_messages USING gin(metadata);
CREATE INDEX IF NOT EXISTS idx_agent_messages_attachments ON agent_messages USING gin(attachments);
CREATE INDEX IF NOT EXISTS idx_agent_messages_tags ON agent_messages USING gin(tags);
CREATE INDEX IF NOT EXISTS idx_agent_messages_search ON agent_messages USING gin(search_vector);

-- Trigger to update search vector
CREATE OR REPLACE FUNCTION update_message_search_vector()
RETURNS TRIGGER AS $$
BEGIN
  NEW.search_vector = to_tsvector('english', 
    COALESCE(NEW.subject, '') || ' ' || 
    COALESCE(NEW.content, '') || ' ' ||
    COALESCE(array_to_string(NEW.tags, ' '), '')
  );
  
  -- Calculate message size if not provided
  IF NEW.size_bytes IS NULL THEN
    NEW.size_bytes = octet_length(NEW.content) + 
                     octet_length(COALESCE(NEW.subject, '')) +
                     octet_length(NEW.metadata::text) +
                     octet_length(NEW.attachments::text);
  END IF;
  
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_agent_messages_search_vector
  BEFORE INSERT OR UPDATE ON agent_messages
  FOR EACH ROW
  EXECUTE FUNCTION update_message_search_vector();

-- Function to get message thread
CREATE OR REPLACE FUNCTION get_message_thread(
  p_message_id UUID,
  include_descendants BOOLEAN DEFAULT true
)
RETURNS TABLE(
  message_id UUID,
  reply_to_id UUID,
  level INTEGER,
  sender_agent_id TEXT,
  subject TEXT,
  content TEXT,
  created_at TIMESTAMPTZ,
  message_type message_type
) AS $$
BEGIN
  -- Get the root message of the thread
  WITH RECURSIVE thread_messages AS (
    -- Start with the target message
    SELECT 
      am.message_id,
      am.reply_to_id,
      0 as level,
      am.sender_agent_id,
      am.subject,
      am.content,
      am.created_at,
      am.message_type
    FROM agent_messages am
    WHERE am.message_id = p_message_id
    
    UNION ALL
    
    -- Find parent messages (going up the chain)
    SELECT 
      am.message_id,
      am.reply_to_id,
      tm.level - 1,
      am.sender_agent_id,
      am.subject,
      am.content,
      am.created_at,
      am.message_type
    FROM agent_messages am
    INNER JOIN thread_messages tm ON am.message_id = tm.reply_to_id
    
    UNION ALL
    
    -- Find child messages (going down the chain) - only if requested
    SELECT 
      am.message_id,
      am.reply_to_id,
      tm.level + 1,
      am.sender_agent_id,
      am.subject,
      am.content,
      am.created_at,
      am.message_type
    FROM agent_messages am
    INNER JOIN thread_messages tm ON am.reply_to_id = tm.message_id
    WHERE include_descendants = true
  )
  
  RETURN QUERY
  SELECT DISTINCT
    tm.message_id,
    tm.reply_to_id,
    tm.level,
    tm.sender_agent_id,
    tm.subject,
    tm.content,
    tm.created_at,
    tm.message_type
  FROM thread_messages tm
  ORDER BY tm.level, tm.created_at;
END;
$$ LANGUAGE plpgsql;

-- Function to get recent messages for an agent or run
CREATE OR REPLACE FUNCTION get_recent_messages(
  p_agent_id TEXT DEFAULT NULL,
  p_run_id UUID DEFAULT NULL,
  p_limit INTEGER DEFAULT 100,
  p_message_types message_type[] DEFAULT NULL
)
RETURNS TABLE(
  message_id UUID,
  correlation_id UUID,
  sender_agent_id TEXT,
  recipient_agent_id TEXT,
  message_type message_type,
  priority message_priority,
  subject TEXT,
  content TEXT,
  created_at TIMESTAMPTZ,
  delivered_at TIMESTAMPTZ,
  tags TEXT[]
) AS $$
BEGIN
  RETURN QUERY
  SELECT 
    am.message_id,
    am.correlation_id,
    am.sender_agent_id,
    am.recipient_agent_id,
    am.message_type,
    am.priority,
    am.subject,
    am.content,
    am.created_at,
    am.delivered_at,
    am.tags
  FROM agent_messages am
  WHERE (p_agent_id IS NULL OR am.sender_agent_id = p_agent_id OR am.recipient_agent_id = p_agent_id)
    AND (p_run_id IS NULL OR am.run_id = p_run_id)
    AND (p_message_types IS NULL OR am.message_type = ANY(p_message_types))
  ORDER BY am.created_at DESC
  LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- Function to mark message as processed
CREATE OR REPLACE FUNCTION mark_message_processed(
  p_message_id UUID,
  p_processing_result JSONB DEFAULT NULL,
  p_processing_time_ms INTEGER DEFAULT NULL
)
RETURNS void AS $$
BEGIN
  UPDATE agent_messages 
  SET 
    processed = true,
    delivered_at = COALESCE(delivered_at, NOW()),
    processing_result = p_processing_result,
    processing_time_ms = p_processing_time_ms
  WHERE message_id = p_message_id;
END;
$$ LANGUAGE plpgsql;

-- Function to clean up old messages
CREATE OR REPLACE FUNCTION cleanup_old_agent_messages()
RETURNS void AS $$
BEGIN
  -- Delete expired messages
  DELETE FROM agent_messages 
  WHERE expires_at IS NOT NULL AND expires_at < NOW();
  
  -- Delete old processed log messages (keep for 7 days)
  DELETE FROM agent_messages 
  WHERE message_type = 'log' 
    AND processed = true 
    AND created_at < NOW() - INTERVAL '7 days';
    
  -- Delete old notification messages (keep for 30 days)
  DELETE FROM agent_messages 
  WHERE message_type = 'notification' 
    AND processed = true 
    AND created_at < NOW() - INTERVAL '30 days';
    
  -- Archive old important messages (keep metadata, truncate content)
  UPDATE agent_messages 
  SET content = LEFT(content, 200) || '... [archived]'
  WHERE message_type IN ('command', 'response', 'error')
    AND processed = true 
    AND created_at < NOW() - INTERVAL '3 months'
    AND length(content) > 200;
END;
$$ LANGUAGE plpgsql;

-- Function to get message statistics
CREATE OR REPLACE FUNCTION get_message_statistics(
  days_back INTEGER DEFAULT 7,
  agent_filter TEXT DEFAULT NULL
)
RETURNS TABLE(
  total_messages BIGINT,
  by_type JSONB,
  by_priority JSONB,
  avg_processing_time_ms NUMERIC,
  unprocessed_count BIGINT
) AS $$
BEGIN
  RETURN QUERY
  SELECT 
    COUNT(*) as total_messages,
    json_object_agg(message_type, type_count) as by_type,
    json_object_agg(priority, priority_count) as by_priority,
    ROUND(AVG(processing_time_ms::NUMERIC), 2) as avg_processing_time_ms,
    COUNT(*) FILTER (WHERE processed = false) as unprocessed_count
  FROM (
    SELECT 
      am.message_type,
      COUNT(*) as type_count,
      am.priority,
      COUNT(*) as priority_count,
      am.processing_time_ms,
      am.processed
    FROM agent_messages am
    WHERE am.created_at > NOW() - INTERVAL '1 day' * days_back
      AND (agent_filter IS NULL OR am.sender_agent_id = agent_filter)
    GROUP BY ROLLUP(am.message_type, am.priority, am.processing_time_ms, am.processed)
  ) stats;
END;
$$ LANGUAGE plpgsql;

-- Comments for documentation
COMMENT ON TABLE agent_messages IS 'Inter-agent communication logs and message bus events';
COMMENT ON COLUMN agent_messages.message_id IS 'Unique identifier for this message';
COMMENT ON COLUMN agent_messages.correlation_id IS 'Groups related messages together in a conversation';
COMMENT ON COLUMN agent_messages.reply_to_id IS 'Reference to the message this is replying to';
COMMENT ON COLUMN agent_messages.content IS 'Main message content/body';
COMMENT ON COLUMN agent_messages.metadata IS 'Structured additional data for the message';
COMMENT ON COLUMN agent_messages.attachments IS 'File attachments, links, and related resources';
COMMENT ON COLUMN agent_messages.broadcast IS 'Whether this message was sent to all agents';
COMMENT ON COLUMN agent_messages.bus_topic IS 'Message bus topic/channel for routing';
COMMENT ON COLUMN agent_messages.search_vector IS 'Full-text search index for subject, content, and tags';