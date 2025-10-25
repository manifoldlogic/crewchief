-- Migration 0014: Add enhanced symbol kinds for markdown and multi-language support
--
-- Context: MD_ENHANCE-1001 through MD_ENHANCE-4002 enhanced the parser to extract
-- rich structural metadata from markdown files and added comprehensive support for
-- Rust and Go languages. This migration adds the corresponding enum values to the
-- database schema.
--
-- Background:
-- The enhanced markdown parser (MD_ENHANCE-2001, MD_ENHANCE-3001, MD_ENHANCE-3002)
-- now extracts structural elements like lists, tables, links, and images as
-- first-class searchable chunks. Multi-language support added comprehensive
-- symbol extraction for Rust (traits, impls, macros, async constructs) and
-- Go (packages, module requirements).
--
-- Note: This migration uses IF NOT EXISTS to handle cases where values may have
-- been added manually during development/debugging. All ADD VALUE operations are
-- idempotent and safe to run multiple times.

-- ============================================================================
-- Markdown Structural Elements
-- ============================================================================
-- Added by: MD_ENHANCE-2001 (section boundaries), MD_ENHANCE-3001 (code blocks),
--           MD_ENHANCE-3002 (links)

-- Markdown list items (bullet lists, numbered lists, task lists)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'list';

-- Markdown tables (entire table structures with headers and rows)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'table';

-- Markdown links (inline links, reference links)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'link';

-- Markdown images (inline images, reference images)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'image';

-- Markdown image links (images that are also hyperlinks)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'image_link';

-- ============================================================================
-- Rust Language Support
-- ============================================================================
-- Comprehensive Rust symbol extraction for traits, implementations, macros,
-- async constructs, and module system

-- Module and import system
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'use';        -- Rust use statements
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'import';     -- Import statements (multi-language)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'imports';    -- Import blocks

-- Type definitions
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'trait';      -- Rust trait definitions
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'impl';       -- Rust impl blocks (trait impls, inherent impls)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'struct';     -- Rust struct definitions
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'enum';       -- Rust enum definitions

-- Macros and meta-programming
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'macro';      -- Rust macro definitions and invocations

-- Functions and methods
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'async_method';  -- Async methods
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'async_func';    -- Async functions
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'method';        -- Regular methods

-- Variables and constants
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'static';     -- Static items
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'constant';   -- Constants (const items)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'variable';   -- Variables (let bindings)

-- ============================================================================
-- Go Language Support
-- ============================================================================
-- Go-specific symbols for package management and module system

-- Package declarations (package main, package foo)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'package';

-- Module requirements (go.mod require directives)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'require';

-- Go version declarations (go.mod go version)
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'go_version';

-- ============================================================================
-- End of Migration 0014
-- ============================================================================

-- Verification query (run manually to confirm all values were added):
-- SELECT enumlabel FROM pg_enum
-- WHERE enumtypid = (
--   SELECT oid FROM pg_type
--   WHERE typname = 'symbol_kind'
--   AND typnamespace = (SELECT oid FROM pg_namespace WHERE nspname = 'maproom')
-- )
-- ORDER BY enumsortorder;
