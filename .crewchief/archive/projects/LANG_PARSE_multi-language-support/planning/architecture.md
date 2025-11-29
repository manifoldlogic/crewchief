# LANG_PARSE Architecture: Multi-Language Parser Support

## High-Level Design

### Core Architecture Pattern
```
┌─────────────────────────────────────────────────┐
│             File Scanner                        │
│         (Language Detection)                    │
└────────────┬────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────┐
│          Parser Factory                         │
│    (Language → Parser Instance)                 │
└────────────┬────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────┐
│       Language-Specific Parsers                 │
│  ┌──────────┬──────────┬──────────┬──────────┐ │
│  │TypeScript│  Python  │   Rust   │    Go    │ │
│  │ Parser   │  Parser  │  Parser  │  Parser  │ │
│  └──────────┴──────────┴──────────┴──────────┘ │
└────────────┬────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────┐
│          Symbol Normalizer                      │
│    (Language-specific → Common Schema)          │
└────────────┬────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────┐
│           Database Writer                       │
│         (Chunks, Edges, Metadata)               │
└─────────────────────────────────────────────────┘
```

## Component Specifications

### 1. Language Detection Module
```rust
pub struct LanguageDetector {
    extension_map: HashMap<String, Language>,
    shebang_patterns: Vec<(Regex, Language)>,
    content_patterns: Vec<(Regex, Language)>,
}

impl LanguageDetector {
    pub fn detect(&self, path: &Path, content: &[u8]) -> Language {
        // 1. Check extension
        // 2. Check shebang line
        // 3. Check content patterns
        // 4. Default fallback
    }
}
```

**Detection Priority:**
1. Explicit overrides (config file)
2. File extension mapping
3. Shebang line (#! /usr/bin/env python)
4. Content heuristics (imports, syntax)
5. Default to plaintext

### 2. Parser Factory Pattern
```rust
pub trait LanguageParser: Send + Sync {
    fn parse(&self, content: &str) -> Result<ParsedFile>;
    fn extract_symbols(&self, tree: &Tree, source: &str) -> Vec<Symbol>;
    fn extract_imports(&self, tree: &Tree, source: &str) -> Vec<Import>;
    fn extract_edges(&self, tree: &Tree, source: &str) -> Vec<Edge>;
}

pub struct ParserFactory {
    parsers: HashMap<Language, Box<dyn LanguageParser>>,
}

impl ParserFactory {
    pub fn get_parser(&self, language: Language) -> &dyn LanguageParser {
        self.parsers.get(&language)
            .unwrap_or(&self.parsers[&Language::PlainText])
    }
}
```

### 3. Language-Specific Implementations

#### Python Parser
```rust
pub struct PythonParser {
    parser: Parser,
    symbol_query: Query,
    import_query: Query,
    docstring_query: Query,
}

impl PythonParser {
    const SYMBOL_QUERY: &'static str = r#"
        (function_definition name: (identifier) @function)
        (class_definition name: (identifier) @class)
        (assignment left: (identifier) @variable)
        (decorated_definition) @decorator
    "#;

    const IMPORT_QUERY: &'static str = r#"
        (import_statement) @import
        (import_from_statement) @import_from
    "#;
}
```

#### Rust Parser
```rust
pub struct RustParser {
    parser: Parser,
    queries: RustQueries,
}

impl RustParser {
    const SYMBOL_QUERY: &'static str = r#"
        (function_item name: (identifier) @function)
        (struct_item name: (type_identifier) @struct)
        (enum_item name: (type_identifier) @enum)
        (impl_item type: (type_identifier) @impl)
        (trait_item name: (type_identifier) @trait)
        (mod_item name: (identifier) @module)
    "#;
}
```

#### Go Parser
```rust
pub struct GoParser {
    parser: Parser,
    queries: GoQueries,
}

impl GoParser {
    const SYMBOL_QUERY: &'static str = r#"
        (function_declaration name: (identifier) @function)
        (method_declaration name: (field_identifier) @method)
        (type_declaration (type_spec name: (type_identifier) @type))
        (short_var_declaration left: (identifier_list) @variable)
    "#;
}
```

### 4. Symbol Normalization Layer

```rust
pub struct SymbolNormalizer {
    kind_mappings: HashMap<(Language, String), SymbolKind>,
}

impl SymbolNormalizer {
    pub fn normalize(&self, language: Language, raw_symbol: RawSymbol) -> Symbol {
        Symbol {
            name: self.clean_name(language, raw_symbol.name),
            kind: self.map_kind(language, raw_symbol.kind),
            signature: self.extract_signature(language, raw_symbol),
            docstring: self.extract_docstring(language, raw_symbol),
            metadata: self.build_metadata(language, raw_symbol),
        }
    }

    fn map_kind(&self, language: Language, raw_kind: &str) -> SymbolKind {
        match (language, raw_kind) {
            (Language::Python, "def") => SymbolKind::Function,
            (Language::Python, "class") => SymbolKind::Class,
            (Language::Rust, "fn") => SymbolKind::Function,
            (Language::Rust, "struct") => SymbolKind::Class,
            (Language::Rust, "impl") => SymbolKind::Class,
            (Language::Go, "func") => SymbolKind::Function,
            (Language::Go, "type") => SymbolKind::Type,
            _ => SymbolKind::Other,
        }
    }
}
```

### 5. Configuration Schema

```yaml
# maproom-languages.yml
languages:
  python:
    enabled: true
    extensions: [.py, .pyw, .pyi]
    parser_config:
      extract_docstrings: true
      docstring_style: google  # google|numpy|sphinx
      type_inference: basic    # none|basic|full

  rust:
    enabled: true
    extensions: [.rs]
    parser_config:
      expand_macros: false
      extract_tests: true
      extract_docs: true

  go:
    enabled: true
    extensions: [.go]
    parser_config:
      parse_build_tags: true
      extract_tests: true
```

### 6. Database Schema Extensions

```sql
-- Already exists, but documenting for completeness
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'trait';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'interface';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'enum';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'decorator';
ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'macro';

-- Language-specific metadata storage
-- Uses existing metadata JSONB column in chunks table
-- Examples:
-- Python: {"decorators": ["@property", "@staticmethod"], "is_async": true}
-- Rust: {"is_unsafe": true, "generics": ["T", "U"], "lifetimes": ["'a"]}
-- Go: {"receiver": "UserService", "is_exported": true}
```

### 7. Query Optimization Strategy

```rust
pub struct QueryCache {
    queries: HashMap<(Language, QueryType), Query>,
}

impl QueryCache {
    pub fn get_or_create(&mut self,
                        language: Language,
                        query_type: QueryType) -> &Query {
        self.queries.entry((language, query_type))
            .or_insert_with(|| self.compile_query(language, query_type))
    }
}
```

## Performance Considerations

### Memory Management
- **Parser Pooling**: Reuse parser instances across files
- **Lazy Loading**: Only load language parsers when needed
- **Query Caching**: Compile tree-sitter queries once
- **Bounded Buffers**: Stream large files rather than loading entirely

### Parallelization Strategy
```rust
pub struct ParallelIndexer {
    thread_pool: ThreadPool,
    parser_pool: Arc<Mutex<ParserPool>>,
}

impl ParallelIndexer {
    pub fn index_directory(&self, path: &Path) -> Result<IndexStats> {
        let files = discover_files(path)?;
        let chunks = AtomicUsize::new(0);

        files.par_iter()
            .map(|file| {
                let parser = self.parser_pool.checkout(file.language);
                let result = parser.parse_file(file);
                self.parser_pool.checkin(parser);
                result
            })
            .collect()
    }
}
```

### Incremental Parsing
- Use tree-sitter's incremental parsing for edits
- Cache parse trees with content hashes
- Only reparse changed regions
- Update affected symbols only

## Error Handling Strategy

### Graceful Degradation
```rust
pub enum ParseResult {
    Success(ParsedFile),
    PartialSuccess(ParsedFile, Vec<ParseWarning>),
    Fallback(BasicChunks),  // Line-based chunking
    Failed(ParseError),
}
```

### Recovery Mechanisms
1. **Syntax Errors**: Extract what's parseable, mark rest as "other"
2. **Unsupported Constructs**: Log and skip, continue parsing
3. **Parser Crashes**: Fallback to basic chunking
4. **Unknown Language**: Treat as plaintext

## Testing Architecture

### Test Categories
1. **Grammar Tests**: Verify tree-sitter grammar compatibility
2. **Extraction Tests**: Symbol extraction accuracy
3. **Edge Case Tests**: Malformed code, large files
4. **Performance Tests**: Parsing speed benchmarks
5. **Integration Tests**: End-to-end indexing flow

### Test Data Sources
```
test-fixtures/
├── python/
│   ├── django/          # Real-world Django code
│   ├── flask/           # Flask application
│   ├── ml-project/      # NumPy/Pandas code
│   └── edge-cases/      # Syntax edge cases
├── rust/
│   ├── tokio/           # Async runtime code
│   ├── serde/           # Serialization library
│   └── edge-cases/
└── go/
    ├── kubernetes/      # K8s code samples
    ├── gin/            # Web framework
    └── edge-cases/
```

## Migration Path

### Phase 1: Python Only
1. Implement PythonParser
2. Add to ParserFactory
3. Test on real Python projects
4. Deploy with feature flag

### Phase 2: Rust & Go
1. Implement both parsers
2. Unified testing framework
3. Performance optimization
4. Production rollout

### Phase 3: Additional Languages
1. Java, C++, C# based on demand
2. Community-contributed parsers
3. Plugin architecture

## Monitoring & Observability

### Metrics to Track
- Parse success rate per language
- Parsing speed (files/second)
- Symbol extraction accuracy
- Memory usage per parser
- Error rates by language

### Logging Strategy
```rust
info!("Parsing {} file: {}", language, path);
debug!("Extracted {} symbols from {}", symbols.len(), path);
warn!("Partial parse failure in {}: {}", path, error);
error!("Parser crash for {}: {}", language, error);
```

## Security Considerations

### Input Validation
- File size limits (default 10MB)
- Parsing timeout (default 5s per file)
- Malicious code pattern detection
- Resource consumption limits

### Sandboxing
- Run parsers in restricted environment
- No file system access beyond read
- No network access
- Memory limits enforced