-- Initialize CrewChief database
-- This script runs when PostgreSQL container is first created

-- Create database if not exists (handled by POSTGRES_DB env var)
-- Just ensure we're using the right database
\c crewchief;

-- Create schema for Maproom if not exists
CREATE SCHEMA IF NOT EXISTS maproom;

-- Grant permissions
GRANT ALL PRIVILEGES ON DATABASE crewchief TO postgres;
GRANT ALL ON SCHEMA maproom TO postgres;
GRANT ALL ON SCHEMA public TO postgres;

-- Create extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";  -- For similarity searches

-- Set default search path
ALTER DATABASE crewchief SET search_path TO public, maproom;

-- Create a simple health check table
CREATE TABLE IF NOT EXISTS health_check (
    id SERIAL PRIMARY KEY,
    checked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status TEXT DEFAULT 'healthy'
);

-- Insert initial health check
INSERT INTO health_check (status) VALUES ('initialized');

-- Output success message
DO $$
BEGIN
    RAISE NOTICE 'CrewChief database initialized successfully';
END $$;