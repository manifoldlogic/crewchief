-- Web UI User Preferences Table
-- Stores user preferences, layouts, and UI customizations

CREATE TABLE IF NOT EXISTS web_ui_preferences (
  id BIGSERIAL PRIMARY KEY,
  session_id UUID REFERENCES web_sessions(session_id) ON DELETE CASCADE,
  user_id TEXT, -- Optional for multi-user support
  
  -- Preference categories
  preference_key TEXT NOT NULL, -- e.g., 'theme', 'layout.sidebar', 'search.defaultFilters'
  preference_value JSONB NOT NULL, -- Flexible storage for any preference value
  
  -- Metadata
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  
  -- Scope and context
  scope TEXT NOT NULL DEFAULT 'global', -- 'global', 'repository', 'worktree', 'page'
  context_id TEXT, -- repository ID, worktree ID, or page identifier when scoped
  
  -- Versioning for preferences migration
  version INTEGER NOT NULL DEFAULT 1,
  
  -- Constraints
  UNIQUE(session_id, preference_key, scope, context_id),
  CONSTRAINT valid_scope CHECK (scope IN ('global', 'repository', 'worktree', 'page'))
);

-- Create enum for common preference keys (for validation and consistency)
DO $$ BEGIN
  CREATE TYPE web_preference_key AS ENUM (
    'theme',
    'layout.sidebar.collapsed',
    'layout.sidebar.width',
    'layout.panels.arrangement',
    'search.defaultFilters',
    'search.resultsPerPage',
    'search.autoComplete',
    'dashboard.widgets',
    'dashboard.refreshInterval',
    'editor.theme',
    'editor.fontSize',
    'editor.tabSize',
    'notifications.enabled',
    'notifications.types',
    'terminal.fontSize',
    'terminal.scrollback',
    'git.autoFetch',
    'git.showUntracked'
  );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- Add constraint to validate known preference keys
-- (Optional - can be removed if too restrictive)
-- ALTER TABLE web_ui_preferences 
-- ADD CONSTRAINT valid_preference_key 
-- CHECK (preference_key::web_preference_key IS NOT NULL OR preference_key LIKE 'custom.%');

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_web_prefs_session_key ON web_ui_preferences(session_id, preference_key);
CREATE INDEX IF NOT EXISTS idx_web_prefs_user_key ON web_ui_preferences(user_id, preference_key) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_web_prefs_scope_context ON web_ui_preferences(scope, context_id, preference_key);
CREATE INDEX IF NOT EXISTS idx_web_prefs_updated ON web_ui_preferences(updated_at DESC);

-- Index on preference_value for common queries
CREATE INDEX IF NOT EXISTS idx_web_prefs_value ON web_ui_preferences USING gin(preference_value);

-- Function to get all preferences for a session
CREATE OR REPLACE FUNCTION get_session_preferences(
  p_session_id UUID,
  p_scope TEXT DEFAULT 'global',
  p_context_id TEXT DEFAULT NULL
)
RETURNS TABLE(
  preference_key TEXT,
  preference_value JSONB,
  scope TEXT,
  context_id TEXT,
  updated_at TIMESTAMPTZ
) AS $$
BEGIN
  RETURN QUERY
  SELECT 
    wp.preference_key,
    wp.preference_value,
    wp.scope,
    wp.context_id,
    wp.updated_at
  FROM web_ui_preferences wp
  WHERE wp.session_id = p_session_id
    AND (p_scope IS NULL OR wp.scope = p_scope)
    AND (p_context_id IS NULL OR wp.context_id = p_context_id)
  ORDER BY wp.scope, wp.preference_key;
END;
$$ LANGUAGE plpgsql;

-- Function to set a preference with upsert logic
CREATE OR REPLACE FUNCTION set_preference(
  p_session_id UUID,
  p_preference_key TEXT,
  p_preference_value JSONB,
  p_scope TEXT DEFAULT 'global',
  p_context_id TEXT DEFAULT NULL
)
RETURNS void AS $$
BEGIN
  INSERT INTO web_ui_preferences (
    session_id, 
    preference_key, 
    preference_value, 
    scope, 
    context_id,
    updated_at
  )
  VALUES (
    p_session_id, 
    p_preference_key, 
    p_preference_value, 
    p_scope, 
    p_context_id,
    NOW()
  )
  ON CONFLICT (session_id, preference_key, scope, context_id)
  DO UPDATE SET 
    preference_value = EXCLUDED.preference_value,
    updated_at = NOW(),
    version = web_ui_preferences.version + 1;
END;
$$ LANGUAGE plpgsql;

-- Function to get default preferences
CREATE OR REPLACE FUNCTION get_default_preferences()
RETURNS TABLE(
  preference_key TEXT,
  preference_value JSONB,
  description TEXT
) AS $$
BEGIN
  RETURN QUERY VALUES
    ('theme', '"system"'::jsonb, 'UI theme preference'),
    ('layout.sidebar.collapsed', 'false'::jsonb, 'Sidebar collapsed state'),
    ('layout.sidebar.width', '280'::jsonb, 'Sidebar width in pixels'),
    ('search.resultsPerPage', '25'::jsonb, 'Number of search results per page'),
    ('search.autoComplete', 'true'::jsonb, 'Enable search autocomplete'),
    ('dashboard.refreshInterval', '5000'::jsonb, 'Dashboard refresh interval in ms'),
    ('notifications.enabled', 'true'::jsonb, 'Enable notifications'),
    ('editor.theme', '"vs-code-dark"'::jsonb, 'Code editor theme'),
    ('editor.fontSize', '14'::jsonb, 'Code editor font size'),
    ('editor.tabSize', '2'::jsonb, 'Code editor tab size'),
    ('terminal.fontSize', '14'::jsonb, 'Terminal font size'),
    ('terminal.scrollback', '1000'::jsonb, 'Terminal scrollback lines'),
    ('git.autoFetch', 'true'::jsonb, 'Auto-fetch git changes'),
    ('git.showUntracked', 'true'::jsonb, 'Show untracked files in git status');
END;
$$ LANGUAGE plpgsql;

-- Function to reset preferences to defaults
CREATE OR REPLACE FUNCTION reset_preferences_to_defaults(
  p_session_id UUID
)
RETURNS void AS $$
DECLARE
  default_pref RECORD;
BEGIN
  FOR default_pref IN SELECT * FROM get_default_preferences() LOOP
    PERFORM set_preference(
      p_session_id,
      default_pref.preference_key,
      default_pref.preference_value
    );
  END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Trigger to update updated_at automatically
CREATE OR REPLACE FUNCTION update_web_prefs_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_web_prefs_updated_at
  BEFORE UPDATE ON web_ui_preferences
  FOR EACH ROW
  EXECUTE FUNCTION update_web_prefs_updated_at();

-- Comments for documentation
COMMENT ON TABLE web_ui_preferences IS 'User preferences and UI customizations for the CrewChief web interface';
COMMENT ON COLUMN web_ui_preferences.preference_key IS 'Hierarchical preference key (e.g., layout.sidebar.width)';
COMMENT ON COLUMN web_ui_preferences.preference_value IS 'JSON value for the preference (supports any data type)';
COMMENT ON COLUMN web_ui_preferences.scope IS 'Preference scope: global, repository, worktree, or page-specific';
COMMENT ON COLUMN web_ui_preferences.context_id IS 'Context identifier when scope is not global';
COMMENT ON COLUMN web_ui_preferences.version IS 'Preference version for migration tracking';