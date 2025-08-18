-- Migration: System Configuration Table
-- Description: Add system_config table for managing application-wide configuration settings
-- Date: 2024-01-17

-- Create system_config table
CREATE TABLE IF NOT EXISTS system_config (
    id BIGSERIAL PRIMARY KEY,
    config_key VARCHAR(255) NOT NULL UNIQUE,
    config_value JSONB NOT NULL,
    description TEXT,
    is_sensitive BOOLEAN NOT NULL DEFAULT FALSE,
    requires_restart BOOLEAN NOT NULL DEFAULT FALSE,
    category VARCHAR(100) NOT NULL DEFAULT 'general',
    validation_schema TEXT,
    default_value JSONB,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for system_config
CREATE INDEX IF NOT EXISTS idx_system_config_category ON system_config(category);
CREATE INDEX IF NOT EXISTS idx_system_config_sensitive ON system_config(is_sensitive);
CREATE INDEX IF NOT EXISTS idx_system_config_restart ON system_config(requires_restart);
CREATE INDEX IF NOT EXISTS idx_system_config_updated_at ON system_config(updated_at);

-- Insert default system configuration values
INSERT INTO system_config (config_key, config_value, description, category, default_value) VALUES
    ('app_name', '"CrewChief Web UI"', 'Application display name', 'general', '"CrewChief Web UI"'),
    ('max_concurrent_agents', '5', 'Maximum number of agents that can run concurrently', 'agents', '5'),
    ('default_agent_timeout', '3600', 'Default timeout for agent runs in seconds', 'agents', '3600'),
    ('enable_auto_merge', 'true', 'Enable automatic merging of successful agent runs', 'agents', 'true'),
    ('min_evaluation_score', '80', 'Minimum evaluation score required for auto-merge', 'agents', '80'),
    ('session_timeout_minutes', '1440', 'Session timeout in minutes (24 hours)', 'auth', '1440'),
    ('rate_limit_per_minute', '100', 'API rate limit per minute per user', 'api', '100'),
    ('max_search_results', '100', 'Maximum number of search results to return', 'search', '100'),
    ('log_retention_days', '30', 'Number of days to retain application logs', 'system', '30'),
    ('backup_enabled', 'true', 'Enable automatic database backups', 'system', 'true'),
    ('maintenance_mode', 'false', 'Enable maintenance mode', 'system', 'false')
ON CONFLICT (config_key) DO NOTHING;

-- Create function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_system_config_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    NEW.version = OLD.version + 1;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger for updated_at
DROP TRIGGER IF EXISTS tr_system_config_updated_at ON system_config;
CREATE TRIGGER tr_system_config_updated_at
    BEFORE UPDATE ON system_config
    FOR EACH ROW
    EXECUTE FUNCTION update_system_config_updated_at();

-- Add comments for documentation
COMMENT ON TABLE system_config IS 'System-wide configuration settings';
COMMENT ON COLUMN system_config.config_key IS 'Unique configuration key identifier';
COMMENT ON COLUMN system_config.config_value IS 'Configuration value stored as JSONB';
COMMENT ON COLUMN system_config.description IS 'Human-readable description of the configuration';
COMMENT ON COLUMN system_config.is_sensitive IS 'Whether this configuration contains sensitive data';
COMMENT ON COLUMN system_config.requires_restart IS 'Whether changing this config requires application restart';
COMMENT ON COLUMN system_config.category IS 'Configuration category for grouping';
COMMENT ON COLUMN system_config.validation_schema IS 'JSON schema for validating configuration values';
COMMENT ON COLUMN system_config.default_value IS 'Default value for this configuration';
COMMENT ON COLUMN system_config.version IS 'Version number, incremented on each update';