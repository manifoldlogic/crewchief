-- Authentication Roles and Permissions
-- Role-based access control (RBAC) system for CrewChief Web UI

CREATE TABLE IF NOT EXISTS auth_roles (
  id BIGSERIAL PRIMARY KEY,
  name VARCHAR(50) UNIQUE NOT NULL,
  display_name VARCHAR(100) NOT NULL,
  description TEXT,
  
  -- Role configuration
  is_default BOOLEAN NOT NULL DEFAULT false,
  is_system BOOLEAN NOT NULL DEFAULT false, -- System roles cannot be deleted
  permissions JSONB NOT NULL DEFAULT '[]', -- Array of permission strings
  
  -- Timestamps
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  
  -- Constraints
  CONSTRAINT valid_role_name CHECK (name ~* '^[a-z0-9_-]+$'),
  CONSTRAINT single_default_role EXCLUDE (is_default WITH =) WHERE (is_default = true)
);

-- User roles junction table (many-to-many)
CREATE TABLE IF NOT EXISTS auth_user_roles (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
  role_id BIGINT NOT NULL REFERENCES auth_roles(id) ON DELETE CASCADE,
  
  -- Assignment metadata
  assigned_by BIGINT REFERENCES auth_users(id) ON DELETE SET NULL,
  assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ, -- Optional role expiration
  
  -- Constraints
  UNIQUE(user_id, role_id),
  CONSTRAINT valid_expiry CHECK (expires_at IS NULL OR expires_at > assigned_at)
);

-- Refresh tokens table (for JWT token management)
CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
  id BIGSERIAL PRIMARY KEY,
  token_hash VARCHAR(255) UNIQUE NOT NULL, -- SHA-256 hash of the actual token
  user_id BIGINT NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
  
  -- Token metadata
  family_id UUID NOT NULL DEFAULT gen_random_uuid(), -- For token rotation
  device_info JSONB DEFAULT '{}', -- Browser, OS, etc.
  ip_address INET,
  
  -- Expiration and revocation
  expires_at TIMESTAMPTZ NOT NULL,
  revoked_at TIMESTAMPTZ,
  revoked_by BIGINT REFERENCES auth_users(id) ON DELETE SET NULL,
  revoked_reason VARCHAR(100),
  
  -- Timestamps
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  
  -- Constraints
  CONSTRAINT valid_expiry CHECK (expires_at > created_at)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_auth_roles_name ON auth_roles(name);
CREATE INDEX IF NOT EXISTS idx_auth_roles_default ON auth_roles(is_default) WHERE is_default = true;
CREATE INDEX IF NOT EXISTS idx_auth_roles_system ON auth_roles(is_system);

CREATE INDEX IF NOT EXISTS idx_auth_user_roles_user ON auth_user_roles(user_id);
CREATE INDEX IF NOT EXISTS idx_auth_user_roles_role ON auth_user_roles(role_id);
CREATE INDEX IF NOT EXISTS idx_auth_user_roles_expires ON auth_user_roles(expires_at) WHERE expires_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_hash ON auth_refresh_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_user ON auth_refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_family ON auth_refresh_tokens(family_id);
CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_expires ON auth_refresh_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_active ON auth_refresh_tokens(user_id, expires_at) WHERE revoked_at IS NULL;

-- Update timestamp triggers
CREATE OR REPLACE FUNCTION update_auth_roles_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER auth_roles_updated_at
  BEFORE UPDATE ON auth_roles
  FOR EACH ROW
  EXECUTE FUNCTION update_auth_roles_updated_at();

-- Cleanup expired tokens and roles
CREATE OR REPLACE FUNCTION cleanup_expired_auth_tokens()
RETURNS void AS $$
BEGIN
  -- Mark expired refresh tokens as revoked
  UPDATE auth_refresh_tokens 
  SET 
    revoked_at = NOW(),
    revoked_reason = 'expired'
  WHERE 
    expires_at < NOW() 
    AND revoked_at IS NULL;
  
  -- Remove very old revoked tokens (older than 30 days)
  DELETE FROM auth_refresh_tokens 
  WHERE 
    revoked_at IS NOT NULL 
    AND revoked_at < NOW() - INTERVAL '30 days';
END;
$$ LANGUAGE plpgsql;

-- Cleanup expired user role assignments
CREATE OR REPLACE FUNCTION cleanup_expired_user_roles()
RETURNS void AS $$
BEGIN
  DELETE FROM auth_user_roles
  WHERE 
    expires_at IS NOT NULL 
    AND expires_at < NOW();
END;
$$ LANGUAGE plpgsql;

-- Insert default roles
INSERT INTO auth_roles (name, display_name, description, is_default, is_system, permissions) VALUES
  ('admin', 'Administrator', 'Full system access with all permissions', false, true, '["*"]'),
  ('user', 'User', 'Standard user access with basic permissions', true, true, '[
    "agents:read",
    "agents:create",
    "agents:update",
    "worktrees:read",
    "worktrees:create",
    "runs:read",
    "runs:create",
    "maproom:search",
    "profile:read",
    "profile:update"
  ]'),
  ('viewer', 'Viewer', 'Read-only access to view system status', false, true, '[
    "agents:read",
    "worktrees:read",
    "runs:read",
    "maproom:search",
    "profile:read"
  ]')
ON CONFLICT (name) DO NOTHING;

-- Comments for documentation
COMMENT ON TABLE auth_roles IS 'System roles for role-based access control';
COMMENT ON COLUMN auth_roles.permissions IS 'JSON array of permission strings (e.g., ["agents:read", "worktrees:create"])';
COMMENT ON COLUMN auth_roles.is_default IS 'Role automatically assigned to new users';
COMMENT ON COLUMN auth_roles.is_system IS 'System-defined roles that cannot be deleted';

COMMENT ON TABLE auth_user_roles IS 'Many-to-many relationship between users and roles';
COMMENT ON COLUMN auth_user_roles.expires_at IS 'Optional expiration date for temporary role assignments';

COMMENT ON TABLE auth_refresh_tokens IS 'Refresh tokens for JWT authentication with rotation support';
COMMENT ON COLUMN auth_refresh_tokens.token_hash IS 'SHA-256 hash of the actual refresh token';
COMMENT ON COLUMN auth_refresh_tokens.family_id IS 'Token family ID for rotation detection';
COMMENT ON COLUMN auth_refresh_tokens.device_info IS 'JSON object with browser, OS, and device information';