-- Authentication Users Table
-- Core user accounts for the CrewChief Web UI authentication system

CREATE TABLE IF NOT EXISTS auth_users (
  id BIGSERIAL PRIMARY KEY,
  uuid UUID UNIQUE NOT NULL DEFAULT gen_random_uuid(),
  email VARCHAR(255) UNIQUE NOT NULL,
  username VARCHAR(50) UNIQUE,
  password_hash VARCHAR(255) NOT NULL,
  first_name VARCHAR(100),
  last_name VARCHAR(100),
  avatar_url TEXT,
  
  -- Account status
  is_active BOOLEAN NOT NULL DEFAULT true,
  is_verified BOOLEAN NOT NULL DEFAULT false,
  is_locked BOOLEAN NOT NULL DEFAULT false,
  failed_login_attempts INTEGER NOT NULL DEFAULT 0,
  locked_until TIMESTAMPTZ,
  
  -- Password management
  password_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  password_reset_token VARCHAR(255),
  password_reset_expires TIMESTAMPTZ,
  
  -- Email verification
  email_verification_token VARCHAR(255),
  email_verification_expires TIMESTAMPTZ,
  
  -- Security
  two_factor_enabled BOOLEAN NOT NULL DEFAULT false,
  two_factor_secret VARCHAR(255),
  backup_codes TEXT[], -- Encrypted backup codes for 2FA
  
  -- Timestamps
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_login_at TIMESTAMPTZ,
  last_login_ip INET,
  
  -- User preferences stored as JSONB
  preferences JSONB DEFAULT '{}',
  
  -- Constraints
  CONSTRAINT valid_email CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$'),
  CONSTRAINT valid_username CHECK (username IS NULL OR (username ~* '^[a-zA-Z0-9_-]{3,30}$')),
  CONSTRAINT valid_failed_attempts CHECK (failed_login_attempts >= 0),
  CONSTRAINT password_reset_token_expiry CHECK (password_reset_token IS NULL OR password_reset_expires IS NOT NULL),
  CONSTRAINT email_verification_token_expiry CHECK (email_verification_token IS NULL OR email_verification_expires IS NOT NULL)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_auth_users_email ON auth_users(email);
CREATE INDEX IF NOT EXISTS idx_auth_users_username ON auth_users(username);
CREATE INDEX IF NOT EXISTS idx_auth_users_uuid ON auth_users(uuid);
CREATE INDEX IF NOT EXISTS idx_auth_users_active ON auth_users(is_active, is_verified) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_auth_users_locked ON auth_users(is_locked, locked_until) WHERE is_locked = true;
CREATE INDEX IF NOT EXISTS idx_auth_users_login_attempts ON auth_users(failed_login_attempts) WHERE failed_login_attempts > 0;
CREATE INDEX IF NOT EXISTS idx_auth_users_last_login ON auth_users(last_login_at DESC);

-- Update timestamp trigger
CREATE OR REPLACE FUNCTION update_auth_users_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER auth_users_updated_at
  BEFORE UPDATE ON auth_users
  FOR EACH ROW
  EXECUTE FUNCTION update_auth_users_updated_at();

-- Account lockout cleanup function
CREATE OR REPLACE FUNCTION unlock_expired_accounts()
RETURNS void AS $$
BEGIN
  UPDATE auth_users 
  SET 
    is_locked = false,
    locked_until = NULL,
    failed_login_attempts = 0
  WHERE 
    is_locked = true 
    AND locked_until IS NOT NULL 
    AND locked_until < NOW();
END;
$$ LANGUAGE plpgsql;

-- Comments for documentation
COMMENT ON TABLE auth_users IS 'Core user accounts for CrewChief Web UI authentication';
COMMENT ON COLUMN auth_users.uuid IS 'External UUID for API references';
COMMENT ON COLUMN auth_users.email IS 'Primary login identifier and contact email';
COMMENT ON COLUMN auth_users.username IS 'Optional display name for users';
COMMENT ON COLUMN auth_users.password_hash IS 'Bcrypt hashed password';
COMMENT ON COLUMN auth_users.failed_login_attempts IS 'Counter for failed login attempts (for account lockout)';
COMMENT ON COLUMN auth_users.locked_until IS 'Account unlock timestamp';
COMMENT ON COLUMN auth_users.two_factor_secret IS 'Base32 encoded TOTP secret for 2FA';
COMMENT ON COLUMN auth_users.backup_codes IS 'Encrypted one-time backup codes for 2FA recovery';
COMMENT ON COLUMN auth_users.preferences IS 'User-specific UI and application preferences';