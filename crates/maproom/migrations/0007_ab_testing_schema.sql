-- Migration: A/B Testing Schema
-- Description: Add tables for experiment tracking, shadow mode results, and user interaction events
-- Date: 2025-10-24

-- Experiments table: stores A/B test configurations
CREATE TABLE IF NOT EXISTS experiments (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    rollout_percentage INTEGER NOT NULL CHECK (rollout_percentage >= 0 AND rollout_percentage <= 100),
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ,
    status TEXT NOT NULL CHECK (status IN ('running', 'paused', 'completed', 'failed')),
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Shadow results table: logs parallel execution of old vs new search
CREATE TABLE IF NOT EXISTS shadow_results (
    id UUID PRIMARY KEY,
    experiment_id UUID NOT NULL REFERENCES experiments(id) ON DELETE CASCADE,
    query TEXT NOT NULL,
    old_results JSONB NOT NULL,
    new_results JSONB,
    old_latency_ms INTEGER NOT NULL,
    new_latency_ms INTEGER,
    new_error TEXT,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id TEXT,
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Interaction events table: captures user behavior during experiments
CREATE TABLE IF NOT EXISTS interaction_events (
    id UUID PRIMARY KEY,
    experiment_id UUID NOT NULL REFERENCES experiments(id) ON DELETE CASCADE,
    query TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK (event_type IN ('click', 'dwell', 'selection', 'abandon', 'reformulation')),
    result_position INTEGER,
    dwell_time_ms INTEGER,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id TEXT,
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Indexes for efficient querying

-- Experiments: lookup by status and date range
CREATE INDEX IF NOT EXISTS idx_experiments_status ON experiments(status);
CREATE INDEX IF NOT EXISTS idx_experiments_dates ON experiments(start_date, end_date);
CREATE INDEX IF NOT EXISTS idx_experiments_created_at ON experiments(created_at DESC);

-- Shadow results: query by experiment and timestamp
CREATE INDEX IF NOT EXISTS idx_shadow_results_experiment ON shadow_results(experiment_id);
CREATE INDEX IF NOT EXISTS idx_shadow_results_timestamp ON shadow_results(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_shadow_results_query ON shadow_results(query);
CREATE INDEX IF NOT EXISTS idx_shadow_results_user ON shadow_results(user_id) WHERE user_id IS NOT NULL;

-- Composite index for time-series analysis
CREATE INDEX IF NOT EXISTS idx_shadow_results_experiment_time ON shadow_results(experiment_id, timestamp DESC);

-- Interaction events: query by experiment, event type, and timestamp
CREATE INDEX IF NOT EXISTS idx_interaction_events_experiment ON interaction_events(experiment_id);
CREATE INDEX IF NOT EXISTS idx_interaction_events_timestamp ON interaction_events(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_interaction_events_type ON interaction_events(event_type);
CREATE INDEX IF NOT EXISTS idx_interaction_events_user ON interaction_events(user_id) WHERE user_id IS NOT NULL;

-- Composite index for aggregations
CREATE INDEX IF NOT EXISTS idx_interaction_events_experiment_type ON interaction_events(experiment_id, event_type);

-- Comments for documentation
COMMENT ON TABLE experiments IS 'A/B test experiment configurations and lifecycle tracking';
COMMENT ON TABLE shadow_results IS 'Shadow mode execution logs comparing old vs new search implementations';
COMMENT ON TABLE interaction_events IS 'User interaction events during A/B tests (clicks, dwell time, selections)';

COMMENT ON COLUMN experiments.rollout_percentage IS 'Percentage of traffic routed to new implementation (0-100)';
COMMENT ON COLUMN experiments.config IS 'Experiment configuration including quality gates and metadata';
COMMENT ON COLUMN experiments.status IS 'Experiment lifecycle status: running, paused, completed, or failed';

COMMENT ON COLUMN shadow_results.old_results IS 'Search results from production (old) implementation';
COMMENT ON COLUMN shadow_results.new_results IS 'Search results from experimental (new) implementation';
COMMENT ON COLUMN shadow_results.old_latency_ms IS 'Latency of old implementation in milliseconds';
COMMENT ON COLUMN shadow_results.new_latency_ms IS 'Latency of new implementation in milliseconds (NULL if timeout/error)';
COMMENT ON COLUMN shadow_results.new_error IS 'Error message from new implementation (NULL if successful)';

COMMENT ON COLUMN interaction_events.event_type IS 'Type of interaction: click, dwell, selection, abandon, or reformulation';
COMMENT ON COLUMN interaction_events.result_position IS 'Position of result in list (1-indexed), NULL for abandon/reformulation';
COMMENT ON COLUMN interaction_events.dwell_time_ms IS 'Time spent on result in milliseconds (for dwell events)';

-- Update trigger for experiments.updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_experiments_updated_at
    BEFORE UPDATE ON experiments
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Data retention policy helpers
-- Note: Actual cleanup should be run by scheduled job

COMMENT ON TABLE shadow_results IS 'Shadow mode execution logs comparing old vs new search implementations. Default retention: 90 days.';
COMMENT ON TABLE interaction_events IS 'User interaction events during A/B tests. Default retention: 90 days.';

-- Example cleanup queries (to be run by scheduled job):
-- DELETE FROM shadow_results WHERE timestamp < NOW() - INTERVAL '90 days';
-- DELETE FROM interaction_events WHERE timestamp < NOW() - INTERVAL '90 days';

-- Performance optimization: Consider partitioning for high-volume production
-- PARTITION BY RANGE (timestamp) for shadow_results and interaction_events
-- This migration creates base tables; partitioning can be added later if needed
