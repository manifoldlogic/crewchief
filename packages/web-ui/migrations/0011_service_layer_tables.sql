-- Migration 0011: Service Layer Tables
-- Creates tables needed for the service layer including audit logs and system monitoring

-- Audit log table for tracking all service operations
CREATE TABLE IF NOT EXISTS audit_log (
    id SERIAL PRIMARY KEY,
    correlation_id VARCHAR(36) NOT NULL,
    service VARCHAR(100) NOT NULL,
    operation VARCHAR(100) NOT NULL,
    user_id VARCHAR(100),
    resource VARCHAR(255),
    action VARCHAR(100) NOT NULL,
    success BOOLEAN NOT NULL,
    error TEXT,
    metadata JSONB DEFAULT '{}',
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    ip_address INET,
    user_agent TEXT
);

-- Indexes for audit log
CREATE INDEX IF NOT EXISTS idx_audit_log_correlation_id ON audit_log(correlation_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_service ON audit_log(service);
CREATE INDEX IF NOT EXISTS idx_audit_log_operation ON audit_log(operation);
CREATE INDEX IF NOT EXISTS idx_audit_log_user_id ON audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_log_success ON audit_log(success);

-- System metrics table for monitoring
CREATE TABLE IF NOT EXISTS system_metrics (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    cpu_usage DECIMAL(5,2) NOT NULL,
    cpu_cores INTEGER NOT NULL,
    load_average JSONB DEFAULT '[]',
    memory_total BIGINT NOT NULL,
    memory_used BIGINT NOT NULL,
    memory_free BIGINT NOT NULL,
    memory_usage DECIMAL(5,2) NOT NULL,
    disk_total BIGINT NOT NULL,
    disk_used BIGINT NOT NULL,
    disk_free BIGINT NOT NULL,
    disk_usage DECIMAL(5,2) NOT NULL,
    processes_total INTEGER DEFAULT 0,
    processes_running INTEGER DEFAULT 0
);

-- Indexes for system metrics
CREATE INDEX IF NOT EXISTS idx_system_metrics_timestamp ON system_metrics(timestamp);

-- Service health table
CREATE TABLE IF NOT EXISTS service_health (
    id SERIAL PRIMARY KEY,
    service VARCHAR(100) NOT NULL,
    healthy BOOLEAN NOT NULL,
    last_check TIMESTAMP WITH TIME ZONE NOT NULL,
    response_time INTEGER NOT NULL,
    details JSONB DEFAULT '{}',
    error TEXT
);

-- Indexes for service health
CREATE INDEX IF NOT EXISTS idx_service_health_service ON service_health(service);
CREATE INDEX IF NOT EXISTS idx_service_health_last_check ON service_health(last_check);
CREATE INDEX IF NOT EXISTS idx_service_health_healthy ON service_health(healthy);

-- System alerts table
CREATE TABLE IF NOT EXISTS system_alerts (
    id VARCHAR(100) PRIMARY KEY,
    type VARCHAR(20) NOT NULL CHECK (type IN ('info', 'warning', 'error', 'critical')),
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    source VARCHAR(100) NOT NULL,
    severity INTEGER NOT NULL CHECK (severity BETWEEN 1 AND 5),
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    acknowledged BOOLEAN DEFAULT FALSE,
    acknowledged_by VARCHAR(100),
    acknowledged_at TIMESTAMP WITH TIME ZONE,
    resolved BOOLEAN DEFAULT FALSE,
    resolved_by VARCHAR(100),
    resolved_at TIMESTAMP WITH TIME ZONE,
    metadata JSONB DEFAULT '{}'
);

-- Indexes for system alerts
CREATE INDEX IF NOT EXISTS idx_system_alerts_type ON system_alerts(type);
CREATE INDEX IF NOT EXISTS idx_system_alerts_source ON system_alerts(source);
CREATE INDEX IF NOT EXISTS idx_system_alerts_timestamp ON system_alerts(timestamp);
CREATE INDEX IF NOT EXISTS idx_system_alerts_acknowledged ON system_alerts(acknowledged);
CREATE INDEX IF NOT EXISTS idx_system_alerts_resolved ON system_alerts(resolved);
CREATE INDEX IF NOT EXISTS idx_system_alerts_severity ON system_alerts(severity);

-- Config backups table
CREATE TABLE IF NOT EXISTS config_backups (
    id VARCHAR(100) PRIMARY KEY,
    config_id VARCHAR(100) NOT NULL,
    version VARCHAR(50) NOT NULL,
    data JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_by VARCHAR(100),
    reason VARCHAR(100)
);

-- Indexes for config backups
CREATE INDEX IF NOT EXISTS idx_config_backups_config_id ON config_backups(config_id);
CREATE INDEX IF NOT EXISTS idx_config_backups_created_at ON config_backups(created_at);

-- Update system_config table to add missing columns if they don't exist
DO $$
BEGIN
    -- Add encrypted column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'system_config' AND column_name = 'encrypted') THEN
        ALTER TABLE system_config ADD COLUMN encrypted BOOLEAN DEFAULT FALSE;
    END IF;
    
    -- Add schema column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'system_config' AND column_name = 'schema') THEN
        ALTER TABLE system_config ADD COLUMN schema VARCHAR(100);
    END IF;
END $$;

-- Update agent_runs table to add missing columns if they don't exist
DO $$
BEGIN
    -- Add working_directory column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'agent_runs' AND column_name = 'working_directory') THEN
        ALTER TABLE agent_runs ADD COLUMN working_directory TEXT;
    END IF;
    
    -- Add log_path column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'agent_runs' AND column_name = 'log_path') THEN
        ALTER TABLE agent_runs ADD COLUMN log_path TEXT;
    END IF;
END $$;

-- Performance tracking view (materialized for better performance)
CREATE MATERIALIZED VIEW IF NOT EXISTS performance_metrics AS
SELECT 
    'audit_log' as source_table,
    service,
    operation,
    COUNT(*) as total_operations,
    AVG(EXTRACT(EPOCH FROM (timestamp - LAG(timestamp) OVER (PARTITION BY service, operation ORDER BY timestamp)))) as avg_response_time,
    COUNT(*) FILTER (WHERE success = false) as error_count,
    ROUND((COUNT(*) FILTER (WHERE success = false)::decimal / COUNT(*)) * 100, 2) as error_rate,
    DATE_TRUNC('hour', timestamp) as time_bucket
FROM audit_log
WHERE timestamp > NOW() - INTERVAL '24 hours'
GROUP BY service, operation, DATE_TRUNC('hour', timestamp)
ORDER BY time_bucket DESC;

-- Create unique index for the materialized view
CREATE UNIQUE INDEX IF NOT EXISTS idx_performance_metrics_unique 
ON performance_metrics(source_table, service, operation, time_bucket);

-- Function to refresh performance metrics
CREATE OR REPLACE FUNCTION refresh_performance_metrics()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY performance_metrics;
END;
$$ LANGUAGE plpgsql;

-- Create a function to clean up old data
CREATE OR REPLACE FUNCTION cleanup_old_service_data(retention_days INTEGER DEFAULT 30)
RETURNS void AS $$
BEGIN
    -- Clean up old audit logs
    DELETE FROM audit_log WHERE timestamp < NOW() - INTERVAL '1 day' * retention_days;
    
    -- Clean up old system metrics
    DELETE FROM system_metrics WHERE timestamp < NOW() - INTERVAL '1 day' * retention_days;
    
    -- Clean up old service health records (keep only latest for each service)
    DELETE FROM service_health 
    WHERE id NOT IN (
        SELECT DISTINCT ON (service) id
        FROM service_health
        ORDER BY service, last_check DESC
    ) AND last_check < NOW() - INTERVAL '1 day' * retention_days;
    
    -- Clean up old resolved alerts
    DELETE FROM system_alerts 
    WHERE resolved = true 
    AND resolved_at < NOW() - INTERVAL '1 day' * retention_days;
    
    -- Clean up old config backups (keep only last 10 per config)
    DELETE FROM config_backups cb1
    WHERE EXISTS (
        SELECT 1 FROM (
            SELECT id, ROW_NUMBER() OVER (PARTITION BY config_id ORDER BY created_at DESC) as rn
            FROM config_backups cb2
            WHERE cb2.config_id = cb1.config_id
        ) ranked
        WHERE ranked.id = cb1.id AND ranked.rn > 10
    );
    
    -- Refresh performance metrics
    PERFORM refresh_performance_metrics();
END;
$$ LANGUAGE plpgsql;

-- Add comments for documentation
COMMENT ON TABLE audit_log IS 'Tracks all service operations for security and debugging';
COMMENT ON TABLE system_metrics IS 'Stores system performance metrics over time';
COMMENT ON TABLE service_health IS 'Tracks health status of all services';
COMMENT ON TABLE system_alerts IS 'Stores system alerts and notifications';
COMMENT ON TABLE config_backups IS 'Stores configuration backups for rollback capabilities';
COMMENT ON MATERIALIZED VIEW performance_metrics IS 'Aggregated performance metrics for monitoring dashboards';
COMMENT ON FUNCTION cleanup_old_service_data IS 'Cleans up old service data to manage database size';

-- Grant permissions (adjust as needed for your security model)
-- GRANT SELECT, INSERT ON audit_log TO web_app_user;
-- GRANT SELECT, INSERT ON system_metrics TO web_app_user;
-- GRANT SELECT, INSERT, UPDATE ON service_health TO web_app_user;
-- GRANT SELECT, INSERT, UPDATE ON system_alerts TO web_app_user;
-- GRANT SELECT, INSERT ON config_backups TO web_app_user;
-- GRANT SELECT ON performance_metrics TO web_app_user;