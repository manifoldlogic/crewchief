# Reference Documentation Test Fixtures

This directory contains manually verified markdown documents for testing parser accuracy.

## Test Documents

### test_hierarchy.md
- Tests: Heading hierarchy, parent paths, nested sections
- Expected:
  - 8 headings (1 h1, 3 h2, 3 h3, 1 h4)
  - 2 code blocks (typescript, rust)
  - 1 table
  - 1 list

### test_code_blocks.md
- Tests: Code block detection with various languages
- Expected:
  - 2 headings (1 h1, 8 h2)
  - 9 code blocks (2 rust, 1 typescript, 1 python, 1 bash, 1 plain, 1 json)

### test_mixed_content.md
- Tests: Mixed markdown elements in realistic document
- Expected:
  - 15+ headings (various levels)
  - 7 code blocks (bash, typescript)
  - 1 table
  - 1 list
  - Multiple links

## Manual Verification Process

1. Count headings manually (lines starting with #, ##, etc.)
2. Count code blocks (fenced with ```)
3. Count tables (lines with | delimiters)
4. Count lists (unordered: -, ordered: 1.)
5. Count links ([text](url) pattern)

## Usage in Tests

These documents are used in:
- `crates/maproom/tests/integration/quality_test.rs`
- Parser accuracy validation
- Hierarchy tracking validation
- Element counting verification
