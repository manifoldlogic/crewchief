-- Add support for markdown and documentation chunk types

-- Add new chunk kinds for markdown headings and documentation
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'heading_1';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'heading_2';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'heading_3';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'heading_4';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'heading_5';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'heading_6';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'markdown_section';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'code_block';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'json_key';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'yaml_section';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'toml_section';

-- Add metadata column for additional context (parent heading, language for code blocks, etc)
ALTER TABLE maproom.chunks 
ADD COLUMN IF NOT EXISTS metadata JSONB DEFAULT '{}';

-- Add index on metadata for filtering
CREATE INDEX IF NOT EXISTS idx_chunks_metadata ON maproom.chunks USING gin(metadata);

-- Add index on kind for filtering by document type
CREATE INDEX IF NOT EXISTS idx_chunks_kind ON maproom.chunks(kind);

-- Update indexed_at tracking for worktrees
ALTER TABLE maproom.worktrees 
ADD COLUMN IF NOT EXISTS indexed_at TIMESTAMPTZ DEFAULT NOW();