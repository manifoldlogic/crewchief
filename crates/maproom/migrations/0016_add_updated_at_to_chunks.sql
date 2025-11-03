-- Migration: Add updated_at column to chunks table
-- Ticket: PROVFIX-2001
-- Purpose: Fix "column updated_at does not exist" errors during embedding updates

-- Add updated_at column to chunks table
ALTER TABLE maproom.chunks
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();

-- Create trigger function for auto-update
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE 'plpgsql';

-- Create trigger on chunks table
DROP TRIGGER IF EXISTS update_chunks_updated_at ON maproom.chunks;
CREATE TRIGGER update_chunks_updated_at
    BEFORE UPDATE ON maproom.chunks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
