-- Sample User Preferences for Development
-- Creates realistic user preference data for testing the preferences system

-- Set default preferences for all development sessions
INSERT INTO web_ui_preferences (session_id, preference_key, preference_value, scope, context_id) VALUES
  -- Global preferences for dev_user_1
  ('550e8400-e29b-41d4-a716-446655440001', 'theme', '"dark"', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'layout.sidebar.collapsed', 'false', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'layout.sidebar.width', '320', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'search.resultsPerPage', '50', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'search.autoComplete', 'true', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'dashboard.refreshInterval', '3000', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'notifications.enabled', 'true', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'editor.theme', '"vs-code-dark"', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'editor.fontSize', '16', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'editor.tabSize', '2', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'terminal.fontSize', '14', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'git.autoFetch', 'true', 'global', NULL),
  
  -- Repository-specific preferences for dev_user_1
  ('550e8400-e29b-41d4-a716-446655440001', 'search.defaultFilters', '{"fileTypes": ["ts", "tsx", "js", "jsx"], "excludeTests": true}', 'repository', 'crewchief'),
  ('550e8400-e29b-41d4-a716-446655440001', 'dashboard.widgets', '["recent_runs", "active_agents", "git_status", "search_history"]', 'repository', 'crewchief'),
  ('550e8400-e29b-41d4-a716-446655440001', 'git.showUntracked', 'false', 'repository', 'crewchief'),
  
  -- Worktree-specific preferences
  ('550e8400-e29b-41d4-a716-446655440001', 'layout.panels.arrangement', '"horizontal"', 'worktree', 'main'),
  ('550e8400-e29b-41d4-a716-446655440001', 'search.defaultFilters', '{"language": "typescript", "maxResults": 25}', 'worktree', 'feature-branch'),
  
  -- Global preferences for dev_user_2 (different preferences)
  ('550e8400-e29b-41d4-a716-446655440002', 'theme', '"light"', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440002', 'layout.sidebar.collapsed', 'true', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440002', 'layout.sidebar.width', '250', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440002', 'search.resultsPerPage', '25', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440002', 'search.autoComplete', 'false', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440002', 'dashboard.refreshInterval', '10000', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440002', 'notifications.enabled', 'false', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440002', 'editor.theme', '"github-light"', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440002', 'editor.fontSize', '14', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440002', 'editor.tabSize', '4', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440002', 'terminal.fontSize', '12', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440002', 'git.autoFetch', 'false', 'global', NULL),
  
  -- Repository-specific for dev_user_2
  ('550e8400-e29b-41d4-a716-446655440002', 'dashboard.widgets', '["git_status", "recent_commits", "file_explorer"]', 'repository', 'crewchief'),
  ('550e8400-e29b-41d4-a716-446655440002', 'search.defaultFilters', '{"fileTypes": ["rs", "toml"], "includeComments": true}', 'repository', 'crewchief'),
  
  -- Anonymous session preferences (minimal)
  ('550e8400-e29b-41d4-a716-446655440003', 'theme', '"system"', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440003', 'search.resultsPerPage', '25', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440003', 'notifications.enabled', 'true', 'global', NULL),
  
  -- Custom preferences (demonstrating flexibility)
  ('550e8400-e29b-41d4-a716-446655440001', 'custom.code_review_mode', 'true', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'custom.favorite_agents', '["claude", "gemini"]', 'global', NULL),
  ('550e8400-e29b-41d4-a716-446655440001', 'custom.hotkeys', '{"search": "cmd+k", "newWorktree": "cmd+shift+w", "toggleSidebar": "cmd+b"}', 'global', NULL),
  
  -- Page-specific preferences
  ('550e8400-e29b-41d4-a716-446655440001', 'search.viewMode', '"list"', 'page', 'search'),
  ('550e8400-e29b-41d4-a716-446655440001', 'search.previewPanelWidth', '400', 'page', 'search'),
  ('550e8400-e29b-41d4-a716-446655440002', 'search.viewMode', '"grid"', 'page', 'search'),
  ('550e8400-e29b-41d4-a716-446655440002', 'agents.sortBy', '"status"', 'page', 'agents'),
  ('550e8400-e29b-41d4-a716-446655440002', 'agents.groupBy', '"worktree"', 'page', 'agents');

-- Test the preference functions by setting and getting preferences
SELECT set_preference(
  '550e8400-e29b-41d4-a716-446655440001',
  'test.dynamic_setting',
  '"test_value"'::jsonb,
  'global'
);

-- Display created preferences organized by user and scope
SELECT 
  ws.user_id,
  wup.scope,
  wup.context_id,
  wup.preference_key,
  wup.preference_value,
  wup.created_at
FROM web_ui_preferences wup
JOIN web_sessions ws ON wup.session_id = ws.session_id
WHERE ws.auth_token LIKE 'dev_token_%'
ORDER BY ws.user_id, wup.scope, wup.preference_key;

-- Show preferences by scope for better organization
SELECT 
  'Global Preferences' as category,
  preference_key,
  COUNT(*) as user_count,
  array_agg(DISTINCT preference_value) as values
FROM web_ui_preferences wup
JOIN web_sessions ws ON wup.session_id = ws.session_id
WHERE ws.auth_token LIKE 'dev_token_%' AND wup.scope = 'global'
GROUP BY preference_key

UNION ALL

SELECT 
  'Repository Preferences' as category,
  preference_key,
  COUNT(*) as user_count,
  array_agg(DISTINCT preference_value) as values
FROM web_ui_preferences wup
JOIN web_sessions ws ON wup.session_id = ws.session_id
WHERE ws.auth_token LIKE 'dev_token_%' AND wup.scope = 'repository'
GROUP BY preference_key

ORDER BY category, preference_key;

-- Test the get_session_preferences function
SELECT * FROM get_session_preferences('550e8400-e29b-41d4-a716-446655440001', 'global');

-- Show default preferences
SELECT * FROM get_default_preferences() LIMIT 5;