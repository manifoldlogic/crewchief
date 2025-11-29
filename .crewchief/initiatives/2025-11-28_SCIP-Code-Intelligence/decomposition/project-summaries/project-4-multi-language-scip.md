# Project: Multi-Language SCIP Support

## Project Summary

Extend the SCIP import pipeline to handle indexes from rust-analyzer (Rust) and scip-python (Python), validating that the schema and query layer work correctly across languages. This project is primarily validation and testing work, with minimal new code expected.

The goal is to ensure that the foundation built in Projects 1-2 is truly language-agnostic, as SCIP claims to be.

## Core Criteria Assessment

### Interface Stability 🔒

**External Interfaces:**
- **SCIP Format**: Language-agnostic protobuf (same for all languages)
- **rust-analyzer**: Native SCIP export (`rust-analyzer scip .`)
- **scip-python**: Official Sourcegraph indexer (Apache 2.0)
- **Your Schema** (Project 1): Should work unchanged

**Stability Commitment:** ✅ SCIP format is stable across indexers

**Risk Areas:** 
- Minor: Different indexers may populate optional fields differently
- Minor: Symbol string formats vary by language

### Context Coherence 📦

**Domain Concepts:** 3 (minimal new concepts)
1. **Language-specific symbol formats** - How Rust vs Python vs TS symbols look
2. **Indexer invocation** - How to run each indexer
3. **Test fixtures** - Sample code for each language

**Core Modules:**
- `scip/fixtures/rust/` - Rust test project
- `scip/fixtures/python/` - Python test project
- `scip/tests/multi_language_test.rs` - Cross-language tests

**Context Size:** ~200 words, mostly test code

### Testable Completion 🎯

**Success Criteria:**
- [ ] rust-analyzer SCIP imports without errors
- [ ] scip-python SCIP imports without errors
- [ ] `find_definition` works correctly for Rust symbols
- [ ] `find_references` works correctly for Python symbols
- [ ] No schema changes required (or minimal, documented changes)

**Verification Method:**
- Import real indexes from each language
- Run same query tests as TypeScript
- Verify results against IDE

## Scope Definition

### In Scope
- Test rust-analyzer SCIP output with import pipeline
- Test scip-python output with import pipeline
- Create test fixtures for each language
- Fix any schema gaps discovered
- Document language-specific quirks
- Update existing tests to be language-aware

### Out of Scope
- Automatic indexer invocation (Project 5)
- Language detection logic
- Language-specific query optimizations
- Adding more languages (Go, Java, etc.) - future work

### Edge Cases
- Rust: Macro-generated code may have unusual symbol locations
- Python: Dynamic typing means fewer symbols than TypeScript
- Mixed projects: Test repository with all three languages

## Technical Design

### Test Fixture Structure

```
crates/maproom/src/scip/fixtures/
├── typescript/
│   ├── src/
│   │   ├── index.ts
│   │   └── auth.ts
│   ├── tsconfig.json
│   ├── package.json
│   └── index.scip          # Pre-generated
│
├── rust/
│   ├── src/
│   │   ├── lib.rs
│   │   └── auth.rs
│   ├── Cargo.toml
│   └── index.scip          # Pre-generated
│
├── python/
│   ├── src/
│   │   ├── __init__.py
│   │   └── auth.py
│   ├── pyproject.toml
│   └── index.scip          # Pre-generated
│
└── mixed/
    ├── frontend/           # TypeScript
    ├── backend/            # Rust
    ├── scripts/            # Python
    └── combined.scip       # Merged index
```

### Sample Test Code

**Rust fixture (`rust/src/auth.rs`):**
```rust
/// Authenticates a user with a token.
/// 
/// # Arguments
/// * `token` - JWT token string
/// 
/// # Returns
/// * `Result<User, AuthError>` - The authenticated user or an error
pub fn authenticate(token: &str) -> Result<User, AuthError> {
    let claims = decode_jwt(token)?;
    let user = fetch_user(claims.user_id)?;
    Ok(user)
}

fn decode_jwt(token: &str) -> Result<Claims, AuthError> {
    // Implementation
}

pub struct User {
    pub id: u64,
    pub name: String,
}
```

**Python fixture (`python/src/auth.py`):**
```python
"""Authentication module."""

from dataclasses import dataclass
from typing import Optional


@dataclass
class User:
    """Represents an authenticated user."""
    id: int
    name: str


def authenticate(token: str) -> Optional[User]:
    """
    Authenticates a user with a token.
    
    Args:
        token: JWT token string
        
    Returns:
        The authenticated user or None if invalid
    """
    claims = decode_jwt(token)
    if claims is None:
        return None
    return fetch_user(claims.user_id)


def decode_jwt(token: str) -> Optional[dict]:
    """Decode a JWT token."""
    pass
```

### Test Cases

```rust
#[cfg(test)]
mod multi_language_tests {
    use super::*;
    
    // ===== Rust Tests =====
    
    #[test]
    fn test_rust_import_succeeds() {
        let scip_path = fixture_path("rust/index.scip");
        let db_path = temp_db_path();
        
        let result = import_scip(&scip_path, &db_path);
        
        assert!(result.is_ok());
        
        // Verify expected counts
        let stats = get_import_stats(&db_path);
        assert!(stats.documents > 0);
        assert!(stats.symbols > 0);
        assert!(stats.occurrences > 0);
    }
    
    #[test]
    fn test_rust_find_definition() {
        let engine = load_rust_fixture();
        
        // Find definition of authenticate function
        let pos = Position {
            file: "src/lib.rs".into(),
            line: 15,  // Where authenticate is called
            column: 5,
        };
        
        let def = engine.goto_definition(&pos).unwrap();
        
        assert!(def.is_some());
        let loc = def.unwrap();
        assert_eq!(loc.file, "src/auth.rs");
        assert_eq!(loc.start_line, 8);  // pub fn authenticate
    }
    
    #[test]
    fn test_rust_find_references() {
        let engine = load_rust_fixture();
        
        let refs = engine.find_references(
            "rust-analyzer cargo myproject . auth authenticate",
            true
        ).unwrap();
        
        assert!(refs.len() >= 2);  // Definition + at least 1 call
    }
    
    #[test]
    fn test_rust_symbol_info() {
        let engine = load_rust_fixture();
        
        let info = engine.get_symbol_info(
            "rust-analyzer cargo myproject . auth authenticate"
        ).unwrap();
        
        assert!(info.is_some());
        let sym = info.unwrap();
        assert_eq!(sym.kind, SymbolKind::Function);
        assert!(sym.documentation.unwrap().contains("Authenticates"));
    }
    
    // ===== Python Tests =====
    
    #[test]
    fn test_python_import_succeeds() {
        let scip_path = fixture_path("python/index.scip");
        let db_path = temp_db_path();
        
        let result = import_scip(&scip_path, &db_path);
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_python_find_definition() {
        let engine = load_python_fixture();
        
        let pos = Position {
            file: "src/__init__.py".into(),
            line: 5,  // Where authenticate is imported/called
            column: 10,
        };
        
        let def = engine.goto_definition(&pos).unwrap();
        
        assert!(def.is_some());
        let loc = def.unwrap();
        assert_eq!(loc.file, "src/auth.py");
    }
    
    #[test]
    fn test_python_dataclass_symbol() {
        let engine = load_python_fixture();
        
        // Python dataclasses should be recognized as classes
        let info = engine.get_symbol_info("python User").unwrap();
        
        assert!(info.is_some());
        assert_eq!(info.unwrap().kind, SymbolKind::Class);
    }
    
    // ===== Cross-Language Validation =====
    
    #[test]
    fn test_all_languages_same_schema() {
        // Verify no schema changes needed across languages
        let ts_db = import_and_load("typescript/index.scip");
        let rust_db = import_and_load("rust/index.scip");
        let python_db = import_and_load("python/index.scip");
        
        // All should have same table structure
        assert_eq!(
            get_table_columns(&ts_db, "scip_symbols"),
            get_table_columns(&rust_db, "scip_symbols")
        );
        assert_eq!(
            get_table_columns(&rust_db, "scip_symbols"),
            get_table_columns(&python_db, "scip_symbols")
        );
    }
}
```

### Indexer Commands

**TypeScript:**
```bash
cd fixtures/typescript
npm install
npx @sourcegraph/scip-typescript index
# Produces: index.scip
```

**Rust:**
```bash
cd fixtures/rust
cargo build  # Ensure project compiles
rust-analyzer scip .
# Produces: index.scip
```

**Python:**
```bash
cd fixtures/python
python -m venv venv
source venv/bin/activate
pip install -e .
npx @sourcegraph/scip-python index . --project-name=fixture
# Produces: index.scip
```

### Potential Schema Adjustments

Based on research, likely no changes needed, but watch for:

| Field | TypeScript | Rust | Python | Action |
|-------|------------|------|--------|--------|
| `symbol` format | npm package path | Cargo crate path | Python module path | No change (opaque string) |
| `kind` values | function, class, method | function, struct, impl | function, class, method | May need to expand enum |
| `documentation` | JSDoc | Rustdoc | Docstrings | No change (string) |
| `signature` | TypeScript syntax | Rust syntax | Python syntax | No change (string) |

## Implementation Plan

### Ticket 1: Rust Fixture Setup
- Create `fixtures/rust/` with simple Rust project
- Generate `index.scip` using rust-analyzer
- Commit pre-generated index to repo
- Document how to regenerate

### Ticket 2: Python Fixture Setup
- Create `fixtures/python/` with simple Python project
- Generate `index.scip` using scip-python
- Commit pre-generated index to repo
- Document how to regenerate

### Ticket 3: Import Validation Tests
- Add tests for Rust import
- Add tests for Python import
- Verify row counts and data integrity
- Document any indexer-specific quirks

### Ticket 4: Query Validation Tests
- Add find_definition tests for Rust
- Add find_definition tests for Python
- Add find_references tests for both
- Add symbol_info tests for both

### Ticket 5: Schema Compatibility Review
- Verify schema handles all symbol kinds
- Document any language-specific patterns
- Update schema if absolutely necessary
- Add cross-language test

## Dependencies

**Requires:**
- Project 1 (Schema & Import) - Must be complete
- Project 2 (Query Layer) - Must be complete
- rust-analyzer installed for Rust fixtures
- scip-python installed for Python fixtures

**Required By:**
- Project 5 (Scan Integration) - Validates we can support multiple languages

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Indexer output differs from TypeScript | Medium | Test early, adjust schema if needed |
| rust-analyzer version incompatibility | Low | Pin version in docs |
| scip-python setup complexity | Low | Pre-generate fixtures, document setup |
| Symbol format differences break queries | Medium | Treat symbols as opaque, don't parse |

## Estimated Effort

- **Duration:** 2-3 days
- **Tickets:** 5
- **Files Created:** Mostly test files and fixtures
- **New Code:** Minimal (tests only, unless schema changes needed)

## Validation Checkpoint

After this project, you can confidently claim:
- "SCIP code intelligence works for TypeScript, Rust, and Python"
- "The schema is language-agnostic"
- "Adding new languages is straightforward"

If significant issues are found:
- Document what broke
- Decide if schema changes are worth it
- Consider language-specific handling (last resort)

## Success Metrics

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Import success | 100% for all 3 languages | Import tests pass |
| Query accuracy | Same as TypeScript | Compare to IDE results |
| Schema changes | 0 (ideally) | Code diff review |
| Documentation | All quirks noted | Review fixture READMEs |