-- Add YAML and TOML chunk types
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'yaml_key';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'toml_section';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'toml_key';

-- Ensure metadata column exists (from previous migration)
ALTER TABLE maproom.chunks 
ADD COLUMN IF NOT EXISTS metadata JSONB DEFAULT '{}';

-- Add index on metadata if not exists
CREATE INDEX IF NOT EXISTS idx_chunks_metadata ON maproom.chunks USING gin(metadata);

-- Add indexed_at column for tracking when chunks were indexed
ALTER TABLE maproom.chunks
ADD COLUMN IF NOT EXISTS indexed_at TIMESTAMPTZ DEFAULT NOW();

-- Add index for finding recently indexed chunks
CREATE INDEX IF NOT EXISTS idx_chunks_indexed_at ON maproom.chunks(indexed_at DESC);