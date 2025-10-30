-- Add blake3_hash column to files table for incremental indexing
--
-- This migration adds a BYTEA column to store blake3 content hashes,
-- enabling fast change detection for incremental indexing.
--
-- Note: The existing content_hash TEXT column is preserved for backward compatibility.
-- The new blake3_hash column stores the binary hash directly for efficiency.

-- Add blake3_hash column (nullable to support existing rows)
ALTER TABLE maproom.files
ADD COLUMN IF NOT EXISTS blake3_hash BYTEA;

-- Create index on blake3_hash for fast lookups
-- Using CONCURRENTLY to avoid locking the table during index creation
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_files_blake3_hash
ON maproom.files(blake3_hash);

-- Add comment to document the column purpose
COMMENT ON COLUMN maproom.files.blake3_hash IS
'Blake3 content hash for incremental indexing change detection. Binary format for efficiency.';
