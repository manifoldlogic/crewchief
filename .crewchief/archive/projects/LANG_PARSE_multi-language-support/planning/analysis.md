# LANG_PARSE Analysis: Multi-Language Parser Support

## Problem Space

### Current Limitations
Maproom currently only supports TypeScript/JavaScript parsing, limiting its usefulness for multi-language repositories. Modern codebases are increasingly polyglot, with teams using:
- Python for ML/data processing
- Rust for performance-critical components
- Go for backend services
- Multiple languages within single projects

Without multi-language support, agents working on these codebases have incomplete context, missing critical relationships between components written in different languages.

### Industry Context

#### Tree-sitter Dominance
Tree-sitter has become the de facto standard for incremental parsing in development tools:
- **GitHub**: Uses tree-sitter for syntax highlighting and code navigation
- **Neovim/Helix**: Built-in tree-sitter integration
- **Zed Editor**: Tree-sitter for all language features
- **Performance**: 36x faster than traditional parsers like JavaParser
- **Consistency**: Uniform API across 40+ languages

#### Language-Specific Challenges

**Python:**
- Dynamic typing requires inference for meaningful symbols
- Docstring conventions (Google, NumPy, Sphinx) need parsing
- Import system complexity (relative imports, namespace packages)
- Decorators and metaclasses affect symbol extraction

**Rust:**
- Complex macro system generates code
- Module system with mod.rs conventions
- Trait implementations scattered across files
- Generic constraints provide semantic information

**Go:**
- Interface satisfaction is implicit
- Package-level visibility rules
- Embedded structs create inheritance-like patterns
- Build tags affect which code is active

### Competitive Analysis

**Sourcegraph (Zoekt + LSIF):**
- Uses Language Server Index Format for precise navigation
- Falls back to tree-sitter for unsupported languages
- Separate indexing pipelines per language

**GitHub Copilot:**
- Unified embedding model across languages
- Language-specific tokenization strategies
- 37.6% improvement with specialized embeddings

**Tabnine:**
- AST-based semantic understanding
- Language-specific ranking adjustments
- Cross-language reference detection

### Current State Assessment

**What We Have:**
- Working TypeScript/JavaScript parser
- Tree-sitter infrastructure in place
- Symbol extraction patterns established
- Database schema supports language field

**What's Missing:**
- Language-specific query patterns
- Parser configuration per language
- Symbol kind mappings for each language
- Import/dependency extraction logic
- Docstring/comment parsing variations

### User Impact

**Without This Project:**
- Agents miss 60-80% of codebase context in polyglot repos
- Cannot trace dependencies across language boundaries
- Search results incomplete for multi-language queries
- Context assembly misses critical related code

**With This Project:**
- Complete codebase understanding
- Cross-language relationship tracking
- Unified search across all languages
- Better agent decision-making with full context

## Key Insights

### 1. Uniformity Over Perfection
Rather than perfect extraction for each language, consistent "good enough" extraction across all languages provides more value. An 80% solution for Python/Rust/Go is better than 100% for TypeScript alone.

### 2. Tree-sitter Query Reuse
Many patterns transfer across languages:
- Function definitions have similar structures
- Class/struct patterns are comparable
- Import statements follow patterns
Core queries can be adapted rather than rewritten.

### 3. Language Detection Critical
Accurate language detection prevents parser failures:
- File extensions insufficient (`.h` could be C or C++)
- Shebang lines for scripts
- Content-based detection as fallback
- Build file analysis for context

### 4. Incremental Addition Strategy
Languages should be added based on popularity and demand:
1. Python (ML, scripts, data)
2. Rust (systems, performance)
3. Go (backend services)
4. Java (enterprise)
5. C++ (systems, games)

### 5. Symbol Kind Normalization
Map language-specific constructs to common kinds:
- Python `def` → function
- Rust `impl` → class-like
- Go `type` → type definition
- Maintain language-specific metadata

## Success Criteria

### Functional Requirements
- [ ] Parse Python, Rust, Go files without errors
- [ ] Extract symbols with >90% accuracy
- [ ] Preserve existing TypeScript/JavaScript functionality
- [ ] Handle mixed-language repositories

### Performance Requirements
- [ ] Parsing speed within 2x of TypeScript baseline
- [ ] Memory usage <50MB per language parser
- [ ] Incremental parsing for large files
- [ ] Parallel processing across languages

### Quality Metrics
- [ ] Symbol extraction accuracy >90%
- [ ] Import resolution success >85%
- [ ] Docstring extraction for documented symbols
- [ ] Cross-language reference detection

## Risk Assessment

### Technical Risks
1. **Parser Version Conflicts**: Different tree-sitter grammar versions may conflict
   - *Mitigation*: Pin versions, test compatibility matrix

2. **Memory Overhead**: Multiple parsers increase memory usage
   - *Mitigation*: Lazy loading, parser pooling

3. **Query Complexity**: Language-specific queries hard to maintain
   - *Mitigation*: Extensive testing, query documentation

### Implementation Risks
1. **Scope Creep**: Temptation to add "just one more language"
   - *Mitigation*: Strict phase boundaries, clear prioritization

2. **Testing Coverage**: Hard to test all language constructs
   - *Mitigation*: Use popular open-source projects as test cases

## Recommendations

### MVP Scope (Phase 1)
- Python support only
- Basic symbol extraction
- Function and class detection
- Simple import tracking

### Production Scope (Phase 2)
- Add Rust and Go
- Enhanced extraction patterns
- Cross-language references
- Docstring parsing

### Future Enhancements (Phase 3+)
- Java, C++, C# support
- Language-specific optimizations
- Semantic analysis via LSP
- Type inference integration