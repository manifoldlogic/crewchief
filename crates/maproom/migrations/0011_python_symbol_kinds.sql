-- Migration: Add Python-specific symbol kinds
-- LANG_PARSE-1007: Python Database Integration

-- Add Python-specific symbol kinds to the enum
-- These support Python's unique symbol types

ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'method';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'async_func';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'async_method';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'variable';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'constant';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'imports';

-- Add comment documenting the new values
COMMENT ON TYPE maproom.symbol_kind IS
'Symbol kinds for code chunks:
- func: regular function
- async_func: Python async function
- method: class method
- async_method: Python async method
- class: class definition
- component: React/UI component
- hook: React hook
- module: module/file-level chunk
- var: variable (legacy)
- variable: module-level variable
- constant: module-level constant (uppercase convention)
- type: type definition
- imports: special chunk for import statements
- heading_1-6: markdown headings
- json_key, yaml_key, toml_key, toml_section: config file keys
- other: catch-all for unclassified symbols';
