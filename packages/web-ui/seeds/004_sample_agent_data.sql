-- Sample Agent Runs and Messages for Development
-- Creates realistic agent execution data and message logs for testing

-- First, ensure we have maproom data to reference
-- This script assumes maproom.repos and maproom.worktrees exist

-- Insert sample agent runs
INSERT INTO agent_runs (
  agent_id, agent_type, run_id, repo_id, worktree_id, commit_sha,
  task_description, task_type, instructions, context_files,
  status, started_at, completed_at, duration_ms,
  tmux_session, tmux_window, tmux_pane,
  exit_code, artifacts, evaluation_score, tests_passed, review_required,
  cpu_usage_avg, memory_usage_peak, disk_io_bytes, network_requests,
  log_summary, competition_id, competition_rank, user_feedback, tags
) VALUES
  -- Successful Claude run
  (
    'claude-001',
    'claude',
    '123e4567-e89b-12d3-a456-426614174001',
    1, -- Assumes repo ID 1 exists
    1, -- Assumes worktree ID 1 exists
    'abc123def456',
    'Implement user authentication middleware for Express routes',
    'feature',
    '{"priority": "high", "requirements": ["JWT tokens", "session management", "error handling"], "constraints": ["TypeScript", "Express.js"]}',
    ARRAY['src/auth/types.ts', 'src/middleware/index.ts', 'docs/authentication.md'],
    'completed',
    NOW() - INTERVAL '2 hours',
    NOW() - INTERVAL '1 hour 45 minutes',
    900000, -- 15 minutes
    'crewchief-session',
    1,
    0,
    0,
    '{"files_created": ["src/auth/middleware.ts", "src/auth/session.ts"], "files_modified": ["src/app.ts", "package.json"], "commits": ["feat: add authentication middleware"]}',
    0.92,
    true,
    false,
    45.2,
    512000000, -- 512MB
    1048576, -- 1MB
    12,
    'Successfully implemented JWT-based authentication with session management. All tests pass.',
    '550e8400-e29b-41d4-a716-446655441001',
    1,
    '{"rating": 5, "comment": "Excellent implementation, clean code", "would_use_again": true}',
    ARRAY['authentication', 'middleware', 'security']
  ),
  
  -- Competing Gemini run (same task)
  (
    'gemini-001',
    'gemini',
    '123e4567-e89b-12d3-a456-426614174002',
    1,
    1,
    'abc123def456',
    'Implement user authentication middleware for Express routes',
    'feature',
    '{"priority": "high", "requirements": ["JWT tokens", "session management", "error handling"], "constraints": ["TypeScript", "Express.js"]}',
    ARRAY['src/auth/types.ts', 'src/middleware/index.ts', 'docs/authentication.md'],
    'completed',
    NOW() - INTERVAL '2 hours',
    NOW() - INTERVAL '1 hour 30 minutes',
    1800000, -- 30 minutes
    'crewchief-session',
    1,
    1,
    0,
    '{"files_created": ["src/auth/auth.middleware.ts", "src/utils/jwt.ts"], "files_modified": ["src/app.ts"], "commits": ["add: JWT authentication system"]}',
    0.85,
    true,
    true,
    52.8,
    768000000, -- 768MB
    2097152, -- 2MB
    8,
    'Implemented authentication system with different approach. Requires code review.',
    '550e8400-e29b-41d4-a716-446655441001',
    2,
    '{"rating": 4, "comment": "Good but verbose implementation", "would_use_again": true}',
    ARRAY['authentication', 'jwt', 'middleware']
  ),
  
  -- Failed run
  (
    'claude-002',
    'claude',
    '123e4567-e89b-12d3-a456-426614174003',
    1,
    2, -- Different worktree
    'def456ghi789',
    'Fix database connection pool memory leak',
    'bug_fix',
    '{"priority": "critical", "issue_id": "DB-123", "symptoms": ["memory usage increasing", "connection timeouts"]}',
    ARRAY['src/db/connection.ts', 'src/db/pool.ts', 'logs/error.log'],
    'failed',
    NOW() - INTERVAL '3 hours',
    NOW() - INTERVAL '2 hours 30 minutes',
    1800000,
    'crewchief-session',
    1,
    2,
    1,
    '{"files_modified": ["src/db/connection.ts"], "commits": []}',
    NULL,
    false,
    true,
    78.3,
    1024000000, -- 1GB
    5242880, -- 5MB
    25,
    'Failed to identify root cause of memory leak. Timeout occurred during debugging.',
    NULL,
    NULL,
    '{"rating": 2, "comment": "Could not solve the issue", "would_use_again": false}',
    ARRAY['database', 'memory-leak', 'debugging']
  ),
  
  -- Currently running
  (
    'gemini-002',
    'gemini',
    '123e4567-e89b-12d3-a456-426614174004',
    1,
    1,
    'ghi789jkl012',
    'Add real-time WebSocket support for agent communication',
    'feature',
    '{"priority": "medium", "technologies": ["WebSocket", "Socket.io", "Redis"], "scope": "web-ui integration"}',
    ARRAY['src/server.ts', 'src/websocket/index.ts', 'docs/websocket-api.md'],
    'running',
    NOW() - INTERVAL '45 minutes',
    NULL,
    NULL,
    'crewchief-session',
    1,
    3,
    NULL,
    '{"files_created": ["src/websocket/server.ts"], "files_modified": ["src/server.ts"]}',
    NULL,
    NULL,
    true,
    42.1,
    256000000, -- 256MB
    524288, -- 512KB
    5,
    'In progress: Setting up WebSocket server and Redis integration.',
    NULL,
    NULL,
    NULL,
    ARRAY['websocket', 'real-time', 'communication']
  ),
  
  -- Older successful run
  (
    'claude-003',
    'claude',
    '123e4567-e89b-12d3-a456-426614174005',
    1,
    1,
    'jkl012mno345',
    'Optimize search query performance with better indexing',
    'optimization',
    '{"benchmark_target": "< 100ms", "focus_areas": ["full-text search", "vector search"], "constraints": ["no breaking changes"]}',
    ARRAY['src/db/search.ts', 'migrations/add_indexes.sql'],
    'completed',
    NOW() - INTERVAL '1 day',
    NOW() - INTERVAL '23 hours 30 minutes',
    1800000,
    'crewchief-session',
    2,
    0,
    0,
    '{"files_modified": ["src/db/search.ts"], "files_created": ["migrations/003_search_optimization.sql"], "commits": ["perf: optimize search with composite indexes"]}',
    0.88,
    true,
    false,
    35.7,
    384000000, -- 384MB
    1572864, -- 1.5MB
    3,
    'Successfully reduced average search time from 245ms to 78ms using composite indexes.',
    NULL,
    NULL,
    '{"rating": 5, "comment": "Significant performance improvement", "would_use_again": true}',
    ARRAY['performance', 'search', 'database', 'indexing']
  );

-- Insert sample agent messages
INSERT INTO agent_messages (
  message_id, correlation_id, reply_to_id, run_id, sender_agent_id, recipient_agent_id,
  message_type, priority, subject, content, content_format, metadata, attachments,
  broadcast, delivered_at, acknowledged_at, processed, processing_result,
  bus_topic, tags, size_bytes, processing_time_ms
) VALUES
  -- Initial task assignment
  (
    '550e8400-e29b-41d4-a716-446655442001',
    '550e8400-e29b-41d4-a716-446655443001', -- Correlation for conversation
    NULL,
    '123e4567-e89b-12d3-a456-426614174001',
    'orchestrator',
    'claude-001',
    'command',
    'high',
    'Task Assignment: Authentication Middleware',
    'You have been assigned to implement user authentication middleware for Express routes. Please review the requirements in the instructions and context files.',
    'text',
    '{"task_id": "AUTH-001", "estimated_duration": "30-60 minutes", "complexity": "medium"}',
    '[{"type": "file", "path": "src/auth/types.ts"}, {"type": "documentation", "url": "https://docs.example.com/auth"}]',
    false,
    NOW() - INTERVAL '2 hours 5 minutes',
    NOW() - INTERVAL '2 hours 4 minutes',
    true,
    '{"acknowledged": true, "estimated_completion": "45 minutes"}',
    'agent.tasks',
    ARRAY['task', 'assignment', 'authentication'],
    1024,
    15
  ),
  
  -- Agent acknowledgment
  (
    '550e8400-e29b-41d4-a716-446655442002',
    '550e8400-e29b-41d4-a716-446655443001',
    '550e8400-e29b-41d4-a716-446655442001',
    '123e4567-e89b-12d3-a456-426614174001',
    'claude-001',
    'orchestrator',
    'response',
    'normal',
    'Task Acknowledged',
    'Task received and understood. Beginning implementation of JWT-based authentication middleware. Will provide updates every 15 minutes.',
    'text',
    '{"confidence": 0.9, "approach": "JWT with session management", "dependencies": ["jsonwebtoken", "express-session"]}',
    '[]',
    false,
    NOW() - INTERVAL '2 hours 4 minutes',
    NOW() - INTERVAL '2 hours 3 minutes',
    true,
    '{"status": "acknowledged", "next_update": "15 minutes"}',
    'agent.responses',
    ARRAY['acknowledgment', 'planning'],
    512,
    8
  ),
  
  -- Progress update
  (
    '550e8400-e29b-41d4-a716-446655442003',
    '550e8400-e29b-41d4-a716-446655443001',
    NULL,
    '123e4567-e89b-12d3-a456-426614174001',
    'claude-001',
    'orchestrator',
    'status_update',
    'normal',
    'Progress Update: 30% Complete',
    'Implementation progressing well. Completed JWT token validation logic and session management. Currently working on middleware integration.',
    'text',
    '{"progress": 0.3, "files_created": 2, "tests_written": 3, "next_milestone": "middleware integration"}',
    '[{"type": "code_snippet", "file": "src/auth/middleware.ts", "lines": "1-45"}]',
    false,
    NOW() - INTERVAL '1 hour 50 minutes',
    NOW() - INTERVAL '1 hour 49 minutes',
    true,
    '{"status": "noted", "estimated_remaining": "30 minutes"}',
    'agent.status',
    ARRAY['progress', 'development'],
    768,
    12
  ),
  
  -- File change notification
  (
    '550e8400-e29b-41d4-a716-446655442004',
    '550e8400-e29b-41d4-a716-446655443002', -- Different conversation
    NULL,
    '123e4567-e89b-12d3-a456-426614174001',
    'claude-001',
    NULL, -- Broadcast
    'file_change',
    'low',
    'File Created: src/auth/middleware.ts',
    'Created new authentication middleware file with JWT validation and session management.',
    'json',
    '{"action": "create", "file_path": "src/auth/middleware.ts", "size_bytes": 2048, "language": "typescript"}',
    '[{"type": "diff", "content": "--- /dev/null\\n+++ b/src/auth/middleware.ts\\n@@ -0,0 +1,65 @@\\n+export class AuthMiddleware..."}]',
    true,
    NOW() - INTERVAL '1 hour 48 minutes',
    NULL, -- Not acknowledged (broadcast)
    true,
    '{"indexed": true, "notified_watchers": 2}',
    'files.changes',
    ARRAY['file', 'creation', 'authentication'],
    2048,
    25
  ),
  
  -- Error message
  (
    '550e8400-e29b-41d4-a716-446655442005',
    '550e8400-e29b-41d4-a716-446655443003', -- Error conversation
    NULL,
    '123e4567-e89b-12d3-a456-426614174003', -- Failed run
    'claude-002',
    'orchestrator',
    'error',
    'critical',
    'Memory Leak Investigation Failed',
    'Unable to identify the root cause of the database connection pool memory leak. Connection timeout occurred during debugging session.',
    'text',
    '{"error_code": "DEBUG_TIMEOUT", "attempted_solutions": ["connection pool analysis", "memory profiling", "query optimization"], "time_spent": "30 minutes"}',
    '[{"type": "log", "file": "debug_session.log"}, {"type": "memory_dump", "file": "heap_dump.hprof"}]',
    false,
    NOW() - INTERVAL '2 hours 30 minutes',
    NOW() - INTERVAL '2 hours 29 minutes',
    true,
    '{"escalated": true, "assigned_to": "senior_engineer", "priority": "critical"}',
    'agent.errors',
    ARRAY['error', 'memory-leak', 'debugging', 'timeout'],
    1536,
    45
  ),
  
  -- Competition notification
  (
    '550e8400-e29b-41d4-a716-446655442006',
    '550e8400-e29b-41d4-a716-446655443004',
    NULL,
    NULL, -- System message
    'orchestrator',
    NULL, -- Broadcast to all agents
    'notification',
    'normal',
    'Competition Results: Authentication Middleware Task',
    'Competition completed between claude-001 and gemini-001 for authentication middleware implementation. Winner: claude-001 (score: 0.92 vs 0.85)',
    'json',
    '{"competition_id": "550e8400-e29b-41d4-a716-446655441001", "winner": "claude-001", "participants": ["claude-001", "gemini-001"], "scores": {"claude-001": 0.92, "gemini-001": 0.85}}',
    '[{"type": "report", "title": "Competition Analysis", "url": "/reports/competition/550e8400-e29b-41d4-a716-446655441001"}]',
    true,
    NOW() - INTERVAL '1 hour 45 minutes',
    NULL,
    true,
    '{"notified_users": 2, "logged": true}',
    'competitions.results',
    ARRAY['competition', 'results', 'authentication'],
    896,
    18
  ),
  
  -- Real-time status from currently running agent
  (
    '550e8400-e29b-41d4-a716-446655442007',
    '550e8400-e29b-41d4-a716-446655443005',
    NULL,
    '123e4567-e89b-12d3-a456-426614174004', -- Currently running
    'gemini-002',
    'orchestrator',
    'status_update',
    'normal',
    'WebSocket Implementation: 60% Complete',
    'WebSocket server setup complete. Redis integration in progress. Currently implementing message queuing and connection management.',
    'text',
    '{"progress": 0.6, "current_task": "Redis integration", "websocket_connections": 0, "redis_status": "connecting"}',
    '[]',
    false,
    NOW() - INTERVAL '5 minutes',
    NOW() - INTERVAL '4 minutes',
    true,
    '{"status": "noted", "estimated_remaining": "15-20 minutes"}',
    'agent.status',
    ARRAY['progress', 'websocket', 'redis'],
    427,
    10
  );

-- Display summary of created agent data
SELECT 
  'Agent Runs Summary' as category,
  agent_type,
  status,
  COUNT(*) as count,
  AVG(evaluation_score) as avg_score,
  AVG(duration_ms) as avg_duration_ms
FROM agent_runs
GROUP BY agent_type, status

UNION ALL

SELECT 
  'Message Summary' as category,
  message_type::text,
  CASE WHEN processed THEN 'processed' ELSE 'pending' END,
  COUNT(*) as count,
  NULL as avg_score,
  AVG(processing_time_ms) as avg_processing_time
FROM agent_messages
GROUP BY message_type, processed

ORDER BY category, agent_type;

-- Show recent agent activity
SELECT 
  ar.agent_id,
  ar.agent_type,
  ar.status,
  ar.task_description,
  ar.evaluation_score,
  COUNT(am.id) as message_count
FROM agent_runs ar
LEFT JOIN agent_messages am ON ar.run_id = am.run_id
GROUP BY ar.agent_id, ar.agent_type, ar.status, ar.task_description, ar.evaluation_score, ar.started_at
ORDER BY ar.started_at DESC;

-- Test some of the utility functions
SELECT * FROM get_run_statistics(7) LIMIT 1;
SELECT * FROM get_recent_runs(5);