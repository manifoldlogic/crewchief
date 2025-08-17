-- Sample Web Sessions for Development
-- Creates test user sessions with various states for development and testing

-- Insert sample active sessions
INSERT INTO web_sessions (session_id, user_id, auth_token, expires_at, last_accessed, ip_address, user_agent, session_data) VALUES
  (
    '550e8400-e29b-41d4-a716-446655440001',
    'dev_user_1',
    'dev_token_1_' || encode(gen_random_bytes(16), 'hex'),
    NOW() + INTERVAL '24 hours',
    NOW() - INTERVAL '5 minutes',
    '127.0.0.1',
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/120.0.0.0',
    '{"theme": "dark", "last_repo": "crewchief", "dashboard_layout": "compact"}'
  ),
  (
    '550e8400-e29b-41d4-a716-446655440002',
    'dev_user_2',
    'dev_token_2_' || encode(gen_random_bytes(16), 'hex'),
    NOW() + INTERVAL '12 hours',
    NOW() - INTERVAL '30 minutes',
    '127.0.0.1',
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Safari/605.1.15',
    '{"theme": "light", "notifications_enabled": true, "auto_refresh": 5000}'
  ),
  (
    '550e8400-e29b-41d4-a716-446655440003',
    NULL, -- Anonymous session
    'dev_token_anon_' || encode(gen_random_bytes(16), 'hex'),
    NOW() + INTERVAL '8 hours',
    NOW() - INTERVAL '2 hours',
    '127.0.0.1',
    'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0.0.0',
    '{"theme": "system", "first_visit": true}'
  );

-- Insert expired session for testing cleanup
INSERT INTO web_sessions (session_id, user_id, auth_token, expires_at, last_accessed, ip_address, user_agent, is_active) VALUES
  (
    '550e8400-e29b-41d4-a716-446655440004',
    'dev_user_1',
    'expired_token_' || encode(gen_random_bytes(16), 'hex'),
    NOW() - INTERVAL '1 hour',
    NOW() - INTERVAL '2 hours',
    '127.0.0.1',
    'Test User Agent',
    false
  );

-- Add comments for clarity
COMMENT ON TABLE web_sessions IS 'Sample sessions created for development - includes active and expired sessions for testing';

-- Display created sessions
SELECT 
  session_id,
  user_id,
  LEFT(auth_token, 20) || '...' as token_preview,
  expires_at,
  is_active,
  session_data->>'theme' as theme
FROM web_sessions 
WHERE auth_token LIKE 'dev_token_%' OR auth_token LIKE 'expired_token_%'
ORDER BY created_at;