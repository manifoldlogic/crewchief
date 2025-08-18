-- OAuth Provider Accounts
-- Support for OAuth2 authentication with GitHub, Google, and other providers

CREATE TABLE IF NOT EXISTS auth_oauth_providers (
  id BIGSERIAL PRIMARY KEY,
  name VARCHAR(50) UNIQUE NOT NULL,
  display_name VARCHAR(100) NOT NULL,
  
  -- Provider configuration
  client_id VARCHAR(255) NOT NULL,
  client_secret VARCHAR(255) NOT NULL, -- Should be encrypted in production
  authorization_url TEXT NOT NULL,
  token_url TEXT NOT NULL,
  user_info_url TEXT NOT NULL,
  
  -- Scopes and configuration
  default_scopes TEXT[] NOT NULL DEFAULT '{}',
  config JSONB DEFAULT '{}', -- Additional provider-specific config
  
  -- Status
  is_enabled BOOLEAN NOT NULL DEFAULT true,
  
  -- Timestamps
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  
  -- Constraints
  CONSTRAINT valid_provider_name CHECK (name ~* '^[a-z0-9_-]+$')
);

-- OAuth account linkages
CREATE TABLE IF NOT EXISTS auth_oauth_accounts (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
  provider_id BIGINT NOT NULL REFERENCES auth_oauth_providers(id) ON DELETE CASCADE,
  
  -- OAuth account details
  provider_user_id VARCHAR(255) NOT NULL, -- ID from the OAuth provider
  provider_username VARCHAR(255),
  provider_email VARCHAR(255),
  provider_name VARCHAR(255),
  provider_avatar_url TEXT,
  
  -- OAuth tokens (should be encrypted in production)
  access_token TEXT,
  refresh_token TEXT,
  token_expires_at TIMESTAMPTZ,
  token_scope TEXT,
  
  -- Account metadata
  raw_profile JSONB DEFAULT '{}', -- Full profile data from provider
  
  -- Timestamps
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_login_at TIMESTAMPTZ,
  
  -- Constraints
  UNIQUE(provider_id, provider_user_id),
  UNIQUE(user_id, provider_id) -- One account per provider per user
);

-- Login attempts tracking for security
CREATE TABLE IF NOT EXISTS auth_login_attempts (
  id BIGSERIAL PRIMARY KEY,
  
  -- Attempt details
  email VARCHAR(255),
  user_id BIGINT REFERENCES auth_users(id) ON DELETE SET NULL,
  ip_address INET NOT NULL,
  user_agent TEXT,
  
  -- Attempt type and result
  attempt_type VARCHAR(20) NOT NULL CHECK (attempt_type IN ('password', 'oauth', '2fa')),
  provider_name VARCHAR(50), -- For OAuth attempts
  success BOOLEAN NOT NULL,
  failure_reason VARCHAR(100), -- e.g., 'invalid_password', 'account_locked', 'invalid_2fa'
  
  -- Security metadata
  country_code CHAR(2), -- From IP geolocation
  city VARCHAR(100),
  
  -- Timestamp
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  
  -- Constraints
  CONSTRAINT oauth_provider_required CHECK (
    (attempt_type != 'oauth') OR (provider_name IS NOT NULL)
  )
);

-- Rate limiting tracking
CREATE TABLE IF NOT EXISTS auth_rate_limits (
  id BIGSERIAL PRIMARY KEY,
  identifier_type VARCHAR(20) NOT NULL CHECK (identifier_type IN ('ip', 'email', 'user_id')),
  identifier_value VARCHAR(255) NOT NULL,
  endpoint VARCHAR(100) NOT NULL, -- e.g., 'login', 'register', 'password_reset'
  
  -- Rate limiting data
  attempt_count INTEGER NOT NULL DEFAULT 1,
  window_start TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  blocked_until TIMESTAMPTZ, -- When the block expires
  
  -- Constraints
  UNIQUE(identifier_type, identifier_value, endpoint)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_auth_oauth_providers_name ON auth_oauth_providers(name);
CREATE INDEX IF NOT EXISTS idx_auth_oauth_providers_enabled ON auth_oauth_providers(is_enabled) WHERE is_enabled = true;

CREATE INDEX IF NOT EXISTS idx_auth_oauth_accounts_user ON auth_oauth_accounts(user_id);
CREATE INDEX IF NOT EXISTS idx_auth_oauth_accounts_provider ON auth_oauth_accounts(provider_id);
CREATE INDEX IF NOT EXISTS idx_auth_oauth_accounts_provider_user ON auth_oauth_accounts(provider_id, provider_user_id);
CREATE INDEX IF NOT EXISTS idx_auth_oauth_accounts_email ON auth_oauth_accounts(provider_email);

CREATE INDEX IF NOT EXISTS idx_auth_login_attempts_email ON auth_login_attempts(email, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_auth_login_attempts_ip ON auth_login_attempts(ip_address, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_auth_login_attempts_user ON auth_login_attempts(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_auth_login_attempts_time ON auth_login_attempts(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_auth_rate_limits_identifier ON auth_rate_limits(identifier_type, identifier_value, endpoint);
CREATE INDEX IF NOT EXISTS idx_auth_rate_limits_blocked ON auth_rate_limits(blocked_until) WHERE blocked_until IS NOT NULL;

-- Update timestamp triggers
CREATE OR REPLACE FUNCTION update_auth_oauth_providers_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER auth_oauth_providers_updated_at
  BEFORE UPDATE ON auth_oauth_providers
  FOR EACH ROW
  EXECUTE FUNCTION update_auth_oauth_providers_updated_at();

CREATE OR REPLACE FUNCTION update_auth_oauth_accounts_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER auth_oauth_accounts_updated_at
  BEFORE UPDATE ON auth_oauth_accounts
  FOR EACH ROW
  EXECUTE FUNCTION update_auth_oauth_accounts_updated_at();

-- Cleanup functions
CREATE OR REPLACE FUNCTION cleanup_old_login_attempts()
RETURNS void AS $$
BEGIN
  -- Delete login attempts older than 90 days
  DELETE FROM auth_login_attempts 
  WHERE created_at < NOW() - INTERVAL '90 days';
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION cleanup_expired_rate_limits()
RETURNS void AS $$
BEGIN
  -- Remove expired rate limit blocks
  DELETE FROM auth_rate_limits 
  WHERE 
    blocked_until IS NOT NULL 
    AND blocked_until < NOW();
  
  -- Clean up old rate limit windows (older than 24 hours)
  DELETE FROM auth_rate_limits 
  WHERE window_start < NOW() - INTERVAL '24 hours';
END;
$$ LANGUAGE plpgsql;

-- Insert default OAuth providers (disabled by default)
INSERT INTO auth_oauth_providers (
  name, display_name, client_id, client_secret, 
  authorization_url, token_url, user_info_url, 
  default_scopes, is_enabled
) VALUES
  (
    'github',
    'GitHub',
    COALESCE(current_setting('app.github_client_id', true), 'GITHUB_CLIENT_ID_PLACEHOLDER'),
    COALESCE(current_setting('app.github_client_secret', true), 'GITHUB_CLIENT_SECRET_PLACEHOLDER'),
    'https://github.com/login/oauth/authorize',
    'https://github.com/login/oauth/access_token',
    'https://api.github.com/user',
    '{"user:email", "read:user"}',
    false
  ),
  (
    'google',
    'Google',
    COALESCE(current_setting('app.google_client_id', true), 'GOOGLE_CLIENT_ID_PLACEHOLDER'),
    COALESCE(current_setting('app.google_client_secret', true), 'GOOGLE_CLIENT_SECRET_PLACEHOLDER'),
    'https://accounts.google.com/o/oauth2/v2/auth',
    'https://oauth2.googleapis.com/token',
    'https://www.googleapis.com/oauth2/v2/userinfo',
    '{"openid", "email", "profile"}',
    false
  )
ON CONFLICT (name) DO NOTHING;

-- Comments for documentation
COMMENT ON TABLE auth_oauth_providers IS 'OAuth2 provider configurations for third-party authentication';
COMMENT ON COLUMN auth_oauth_providers.client_secret IS 'OAuth client secret (should be encrypted in production)';
COMMENT ON COLUMN auth_oauth_providers.config IS 'Provider-specific configuration as JSON';

COMMENT ON TABLE auth_oauth_accounts IS 'User accounts linked to OAuth providers';
COMMENT ON COLUMN auth_oauth_accounts.provider_user_id IS 'User ID from the OAuth provider';
COMMENT ON COLUMN auth_oauth_accounts.access_token IS 'OAuth access token (should be encrypted in production)';
COMMENT ON COLUMN auth_oauth_accounts.raw_profile IS 'Complete profile data returned by the OAuth provider';

COMMENT ON TABLE auth_login_attempts IS 'Audit trail of all authentication attempts for security monitoring';
COMMENT ON COLUMN auth_login_attempts.attempt_type IS 'Type of authentication: password, oauth, or 2fa';

COMMENT ON TABLE auth_rate_limits IS 'Rate limiting enforcement for authentication endpoints';
COMMENT ON COLUMN auth_rate_limits.identifier_type IS 'What is being rate limited: ip, email, or user_id';
COMMENT ON COLUMN auth_rate_limits.blocked_until IS 'When the rate limit block expires';