-- Sample Search History for Development
-- Creates realistic search queries and results for testing the search interface

-- Get session IDs for referencing
-- Note: This assumes the sample sessions from 001_sample_sessions.sql exist

-- Insert sample search history entries
INSERT INTO web_search_history (
  session_id, user_id, query, search_type, filters, result_count, 
  execution_time_ms, relevance_threshold, top_results, performance_metrics,
  searched_at, clicked_results, saved, repo_id, worktree_id
) VALUES
  -- Recent semantic searches
  (
    '550e8400-e29b-41d4-a716-446655440001',
    'dev_user_1',
    'authentication middleware',
    'semantic',
    '{"fileTypes": ["ts", "js"], "worktree": "main"}',
    25,
    156,
    0.7,
    '[
      {"id": 1, "file": "src/auth/middleware.ts", "score": 0.95, "preview": "Authentication middleware for Express routes"},
      {"id": 2, "file": "src/auth/session.ts", "score": 0.89, "preview": "Session management utilities"},
      {"id": 3, "file": "src/middleware/auth.ts", "score": 0.85, "preview": "JWT token validation middleware"}
    ]',
    '{"indexUsed": "vector", "cacheHit": false, "scanType": "ivfflat"}',
    NOW() - INTERVAL '5 minutes',
    ARRAY[1, 3],
    true,
    1,
    1
  ),
  (
    '550e8400-e29b-41d4-a716-446655440001',
    'dev_user_1',
    'database connection pool',
    'semantic',
    '{"language": "typescript"}',
    18,
    203,
    0.6,
    '[
      {"id": 4, "file": "src/db/connection.ts", "score": 0.92, "preview": "PostgreSQL connection pool setup"},
      {"id": 5, "file": "src/db/pool.ts", "score": 0.88, "preview": "Database pool configuration"},
      {"id": 6, "file": "crates/maproom/src/db.rs", "score": 0.82, "preview": "Rust database connection handling"}
    ]',
    '{"indexUsed": "vector", "cacheHit": true, "scanType": "ivfflat"}',
    NOW() - INTERVAL '15 minutes',
    ARRAY[4],
    false,
    1,
    1
  ),
  
  -- Full-text searches
  (
    '550e8400-e29b-41d4-a716-446655440002',
    'dev_user_2',
    'TODO FIXME hack',
    'fulltext',
    '{"dateRange": {"from": "2024-01-01", "to": "2024-12-31"}}',
    42,
    89,
    NULL,
    '[
      {"id": 7, "file": "src/components/Dashboard.tsx", "score": 0.75, "preview": "// TODO: Add loading states"},
      {"id": 8, "file": "src/utils/helpers.ts", "score": 0.71, "preview": "// FIXME: Handle edge case"},
      {"id": 9, "file": "src/api/client.ts", "score": 0.68, "preview": "// HACK: Temporary workaround"}
    ]',
    '{"indexUsed": "gin_tsvector", "cacheHit": false, "scanType": "bitmap"}',
    NOW() - INTERVAL '1 hour',
    ARRAY[7, 8],
    true,
    1,
    2
  ),
  
  -- Symbol searches
  (
    '550e8400-e29b-41d4-a716-446655440001',
    'dev_user_1',
    'class DatabaseConnection',
    'symbol',
    '{"fileTypes": ["ts"]}',
    3,
    45,
    NULL,
    '[
      {"id": 10, "file": "src/db/connection.ts", "score": 1.0, "preview": "export class DatabaseConnection {"},
      {"id": 11, "file": "src/db/connection.test.ts", "score": 0.9, "preview": "import { DatabaseConnection }"},
      {"id": 12, "file": "docs/api.md", "score": 0.7, "preview": "## DatabaseConnection Class"}
    ]',
    '{"indexUsed": "symbol_name", "cacheHit": false, "scanType": "btree"}',
    NOW() - INTERVAL '2 hours',
    ARRAY[10],
    false,
    1,
    1
  ),
  
  -- Path searches
  (
    '550e8400-e29b-41d4-a716-446655440002',
    'dev_user_2',
    'src/components/**/Button*',
    'path',
    '{}',
    8,
    23,
    NULL,
    '[
      {"id": 13, "file": "src/components/ui/Button.tsx", "score": 1.0, "preview": "Primary button component"},
      {"id": 14, "file": "src/components/ui/ButtonGroup.tsx", "score": 0.95, "preview": "Button group container"},
      {"id": 15, "file": "src/components/forms/SubmitButton.tsx", "score": 0.9, "preview": "Form submission button"}
    ]',
    '{"indexUsed": "path_gin_trgm", "cacheHit": false, "scanType": "gin"}',
    NOW() - INTERVAL '3 hours',
    ARRAY[13, 14],
    false,
    1,
    2
  ),
  
  -- Anonymous session searches
  (
    '550e8400-e29b-41d4-a716-446655440003',
    NULL,
    'React hooks useState',
    'semantic',
    '{"fileTypes": ["tsx", "jsx"]}',
    31,
    234,
    0.5,
    '[
      {"id": 16, "file": "src/hooks/useLocalStorage.ts", "score": 0.88, "preview": "Custom hook for localStorage"},
      {"id": 17, "file": "src/components/Counter.tsx", "score": 0.85, "preview": "const [count, setCount] = useState(0)"},
      {"id": 18, "file": "src/hooks/useApi.ts", "score": 0.82, "preview": "API data fetching hook"}
    ]',
    '{"indexUsed": "vector", "cacheHit": false, "scanType": "ivfflat"}',
    NOW() - INTERVAL '4 hours',
    ARRAY[16],
    false,
    1,
    1
  ),
  
  -- Popular/repeated searches for analytics
  (
    '550e8400-e29b-41d4-a716-446655440001',
    'dev_user_1',
    'authentication middleware',
    'semantic',
    '{"fileTypes": ["ts", "js"]}',
    25,
    142,
    0.7,
    '[
      {"id": 1, "file": "src/auth/middleware.ts", "score": 0.95, "preview": "Authentication middleware for Express routes"}
    ]',
    '{"indexUsed": "vector", "cacheHit": true, "scanType": "ivfflat"}',
    NOW() - INTERVAL '1 day',
    ARRAY[1],
    true,
    1,
    1
  ),
  (
    '550e8400-e29b-41d4-a716-446655440002',
    'dev_user_2',
    'authentication middleware',
    'semantic',
    '{"language": "typescript"}',
    23,
    158,
    0.6,
    '[
      {"id": 1, "file": "src/auth/middleware.ts", "score": 0.95, "preview": "Authentication middleware for Express routes"}
    ]',
    '{"indexUsed": "vector", "cacheHit": true, "scanType": "ivfflat"}',
    NOW() - INTERVAL '2 days',
    ARRAY[1],
    false,
    1,
    2
  );

-- Add some edge cases and error scenarios
INSERT INTO web_search_history (
  session_id, user_id, query, search_type, result_count, 
  execution_time_ms, searched_at, saved
) VALUES
  -- Empty results
  (
    '550e8400-e29b-41d4-a716-446655440001',
    'dev_user_1',
    'nonexistent_function_xyz123',
    'semantic',
    0,
    67,
    NOW() - INTERVAL '6 hours',
    false
  ),
  -- Very slow query
  (
    '550e8400-e29b-41d4-a716-446655440002',
    'dev_user_2',
    'complex regex pattern .*[a-zA-Z]+.*',
    'fulltext',
    156,
    2847,
    NOW() - INTERVAL '12 hours',
    false
  );

-- Display summary of created search history
SELECT 
  wsh.query,
  wsh.search_type,
  wsh.result_count,
  wsh.execution_time_ms,
  wsh.saved,
  ws.user_id
FROM web_search_history wsh
JOIN web_sessions ws ON wsh.session_id = ws.session_id
WHERE ws.auth_token LIKE 'dev_token_%'
ORDER BY wsh.searched_at DESC;

-- Show popular queries
SELECT * FROM get_popular_searches('7 days', 5);