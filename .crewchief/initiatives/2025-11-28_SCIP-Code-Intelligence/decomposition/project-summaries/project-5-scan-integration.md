# Project: Maproom Scan Integration

## Project Summary

Integrate SCIP index generation into the `maproom scan` command so users get precise code intelligence automatically when they index a repository. This is the final integration step that creates a seamless zero-config experience.

After this project, running `maproom scan` will:
1. Index code with tree-sitter (existing)
2. Generate embeddings (existing)
3. Detect and run SCIP indexers (new)
4. Import SCIP data into the database (new)
5. Enable code intelligence MCP tools (from Project 3)

## Core Criteria Assessment

### Interface Stability ⚠️ Medium Risk

**External Interfaces:**
- **scip-typescript**: npm package, version controlled
- **rust-analyzer**: System binary, version may vary
- **scip-python**: npm package, version controlled
- **User's project**: May or may not have required configs (tsconfig.json, Cargo.toml)

**Stability Commitment:** ⚠️ Depends on user environment

**Risk Areas:**
- Users may not have indexers installed
- Projects may not have valid configs for indexers
- Indexer versions may differ from tested versions

### Context Coherence 📦

**Domain Concepts:** 5
1. **Indexer Detection** - Is scip-typescript/rust-analyzer/scip-python available?
2. **Project Detection** - Does this repo have TypeScript/Rust/Python code?
3. **Indexer Invocation** - How to run each indexer correctly
4. **SCIP Import** - Load generated index into database
5. **Graceful Degradation** - What to do when indexers fail/missing

**Core Modules:**
- `scan/scip_detection.rs` - Detect available indexers
- `scan/scip_runner.rs` - Execute indexers
- `scan/mod.rs` - Integration with existing scan

**Context Size:** ~300 words, single feature addition

### Testable Completion 🎯

**Success Criteria:**
- [ ] `scan` detects scip-typescript if installed
- [ ] `scan` runs indexer and imports results
- [ ] `scan` completes successfully if indexer missing (warning only)
- [ ] MCP tools work immediately after scan completes
- [ ] `--skip-scip` flag bypasses SCIP indexing
- [ ] Clear error messages when indexing fails

**Verification Method:**
- Integration tests with/without indexers
- End-to-end test: scan → query → correct results
- Manual test on real repositories

## Scope Definition

### In Scope
- Detect installed SCIP indexers (scip-typescript, rust-analyzer, scip-python)
- Detect project types (look for tsconfig.json, Cargo.toml, pyproject.toml)
- Run appropriate indexers during `maproom scan`
- Import generated `.scip` files into Maproom database
- `--skip-scip` flag to opt out of SCIP indexing
- `--scip-only` flag to only do SCIP (skip embeddings)
- Progress reporting during SCIP indexing
- Graceful handling of missing indexers or failed indexing

### Out of Scope
- Automatic indexer installation
- Watch mode / incremental SCIP updates
- CI/CD integration documentation (separate docs task)
- Custom indexer configuration (future enhancement)
- Parallel indexing of multiple projects

### Edge Cases
- Indexer installed but project not configured: Warn and skip
- Indexer times out on large project: Warn and continue
- Multiple TypeScript projects in monorepo: Index root only (for now)
- Mixed language project: Run all available indexers
- Indexer produces empty/invalid SCIP: Warn and skip import

## Technical Design

### Indexer Detection

```rust
pub struct IndexerAvailability {
    pub typescript: Option<IndexerInfo>,
    pub rust: Option<IndexerInfo>,
    pub python: Option<IndexerInfo>,
}

pub struct IndexerInfo {
    pub command: String,
    pub version: String,
    pub path: PathBuf,
}

impl IndexerAvailability {
    pub fn detect() -> Self {
        Self {
            typescript: detect_scip_typescript(),
            rust: detect_rust_analyzer(),
            python: detect_scip_python(),
        }
    }
}

fn detect_scip_typescript() -> Option<IndexerInfo> {
    // Try: npx @sourcegraph/scip-typescript --version
    // Or: which scip-typescript
    let output = Command::new("npx")
        .args(["@sourcegraph/scip-typescript", "--version"])
        .output()
        .ok()?;
    
    if output.status.success() {
        Some(IndexerInfo {
            command: "npx @sourcegraph/scip-typescript".into(),
            version: String::from_utf8_lossy(&output.stdout).trim().into(),
            path: PathBuf::from("npx"),
        })
    } else {
        None
    }
}

fn detect_rust_analyzer() -> Option<IndexerInfo> {
    // Try: rust-analyzer --version
    let output = Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .ok()?;
    
    if output.status.success() {
        Some(IndexerInfo {
            command: "rust-analyzer scip".into(),
            version: String::from_utf8_lossy(&output.stdout).trim().into(),
            path: which::which("rust-analyzer").ok()?,
        })
    } else {
        None
    }
}

fn detect_scip_python() -> Option<IndexerInfo> {
    // Try: npx @sourcegraph/scip-python --version
    let output = Command::new("npx")
        .args(["@sourcegraph/scip-python", "--version"])
        .output()
        .ok()?;
    
    if output.status.success() {
        Some(IndexerInfo {
            command: "npx @sourcegraph/scip-python".into(),
            version: String::from_utf8_lossy(&output.stdout).trim().into(),
            path: PathBuf::from("npx"),
        })
    } else {
        None
    }
}
```

### Project Detection

```rust
pub struct ProjectTypes {
    pub typescript: Option<TypeScriptProject>,
    pub rust: Option<RustProject>,
    pub python: Option<PythonProject>,
}

pub struct TypeScriptProject {
    pub tsconfig_path: PathBuf,
    pub package_json_path: Option<PathBuf>,
    pub is_monorepo: bool,
}

pub struct RustProject {
    pub cargo_toml_path: PathBuf,
    pub is_workspace: bool,
}

pub struct PythonProject {
    pub root: PathBuf,
    pub has_pyproject: bool,
    pub has_setup_py: bool,
}

impl ProjectTypes {
    pub fn detect(repo_root: &Path) -> Self {
        Self {
            typescript: detect_typescript_project(repo_root),
            rust: detect_rust_project(repo_root),
            python: detect_python_project(repo_root),
        }
    }
}

fn detect_typescript_project(root: &Path) -> Option<TypeScriptProject> {
    let tsconfig = root.join("tsconfig.json");
    if tsconfig.exists() {
        Some(TypeScriptProject {
            tsconfig_path: tsconfig,
            package_json_path: root.join("package.json").exists()
                .then(|| root.join("package.json")),
            is_monorepo: root.join("lerna.json").exists() 
                || root.join("pnpm-workspace.yaml").exists(),
        })
    } else {
        None
    }
}
```

### Indexer Runner

```rust
pub struct ScipIndexResult {
    pub language: String,
    pub scip_path: PathBuf,
    pub duration: Duration,
    pub stats: IndexStats,
}

pub struct IndexStats {
    pub documents: usize,
    pub symbols: usize,
    pub occurrences: usize,
}

pub async fn run_scip_indexers(
    repo_root: &Path,
    indexers: &IndexerAvailability,
    projects: &ProjectTypes,
    progress: &ProgressReporter,
) -> Vec<Result<ScipIndexResult, IndexError>> {
    let mut results = Vec::new();
    
    // TypeScript
    if let (Some(indexer), Some(project)) = (&indexers.typescript, &projects.typescript) {
        progress.report("Running scip-typescript...");
        let result = run_typescript_indexer(repo_root, indexer, project).await;
        results.push(result);
    }
    
    // Rust
    if let (Some(indexer), Some(project)) = (&indexers.rust, &projects.rust) {
        progress.report("Running rust-analyzer scip...");
        let result = run_rust_indexer(repo_root, indexer, project).await;
        results.push(result);
    }
    
    // Python
    if let (Some(indexer), Some(project)) = (&indexers.python, &projects.python) {
        progress.report("Running scip-python...");
        let result = run_python_indexer(repo_root, indexer, project).await;
        results.push(result);
    }
    
    results
}

async fn run_typescript_indexer(
    repo_root: &Path,
    indexer: &IndexerInfo,
    project: &TypeScriptProject,
) -> Result<ScipIndexResult, IndexError> {
    let start = Instant::now();
    let scip_path = repo_root.join("index.scip");
    
    // Ensure node_modules exists
    if !repo_root.join("node_modules").exists() {
        return Err(IndexError::MissingDependencies(
            "Run 'npm install' before indexing TypeScript".into()
        ));
    }
    
    let mut cmd = Command::new("npx");
    cmd.current_dir(repo_root)
        .args(["@sourcegraph/scip-typescript", "index"]);
    
    // Add workspace flag for monorepos
    if project.is_monorepo {
        cmd.arg("--pnpm-workspaces");  // or --yarn-workspaces
    }
    
    let output = cmd.output().await
        .map_err(|e| IndexError::ExecutionFailed(e.to_string()))?;
    
    if !output.status.success() {
        return Err(IndexError::IndexerFailed(
            String::from_utf8_lossy(&output.stderr).into()
        ));
    }
    
    // Parse stats from output (scip-typescript prints them)
    let stats = parse_scip_stats(&scip_path)?;
    
    Ok(ScipIndexResult {
        language: "typescript".into(),
        scip_path,
        duration: start.elapsed(),
        stats,
    })
}
```

### Integration with Scan Command

```rust
// In crates/maproom/src/commands/scan.rs

pub struct ScanOptions {
    pub repo: String,
    pub path: PathBuf,
    
    // Existing options
    pub provider: EmbeddingProvider,
    pub concurrency: usize,
    
    // New SCIP options
    pub skip_scip: bool,
    pub scip_only: bool,
}

pub async fn scan(opts: ScanOptions) -> Result<ScanResult> {
    let repo_root = &opts.path;
    
    // === Existing: Tree-sitter indexing ===
    if !opts.scip_only {
        info!("Parsing source files...");
        let chunks = parse_repository(repo_root).await?;
        
        info!("Generating embeddings...");
        generate_embeddings(&chunks, opts.provider).await?;
    }
    
    // === New: SCIP indexing ===
    if !opts.skip_scip {
        info!("Detecting SCIP indexers...");
        let indexers = IndexerAvailability::detect();
        let projects = ProjectTypes::detect(repo_root);
        
        if indexers.any_available() && projects.any_detected() {
            info!("Running SCIP indexers for code intelligence...");
            let scip_results = run_scip_indexers(
                repo_root, 
                &indexers, 
                &projects,
                &progress
            ).await;
            
            for result in scip_results {
                match result {
                    Ok(index) => {
                        info!(
                            "Importing {} SCIP index ({} symbols)...",
                            index.language,
                            index.stats.symbols
                        );
                        import_scip(&index.scip_path, &db_path)?;
                        
                        // Clean up .scip file
                        std::fs::remove_file(&index.scip_path).ok();
                    }
                    Err(e) => {
                        warn!("SCIP indexing failed for {}: {}", e.language(), e);
                        // Continue with other indexers
                    }
                }
            }
        } else {
            if !indexers.any_available() {
                info!(
                    "No SCIP indexers found. Install scip-typescript, rust-analyzer, \
                    or scip-python for code intelligence features."
                );
            }
            if !projects.any_detected() {
                info!(
                    "No supported project types found (tsconfig.json, Cargo.toml, \
                    pyproject.toml)."
                );
            }
        }
    }
    
    Ok(ScanResult { /* ... */ })
}
```

### CLI Interface

```bash
# Default: runs tree-sitter + embeddings + SCIP
maproom scan

# Skip SCIP indexing (faster, no code intelligence)
maproom scan --skip-scip

# Only SCIP (skip embeddings, useful for quick refresh)
maproom scan --scip-only

# Verbose output showing indexer detection
maproom scan --verbose

# Output example:
Scanning repository: crewchief
  Path: /workspace
  
Parsing source files...
  Files: 245
  Chunks: 3,847
  Duration: 12.3s

Generating embeddings...
  Provider: openai
  Chunks embedded: 3,847
  Duration: 45.2s

Detecting SCIP indexers...
  ✓ scip-typescript v0.3.10 found
  ✓ rust-analyzer v0.3.1800 found
  ✗ scip-python not installed

Running SCIP indexers...
  TypeScript: 2,847 symbols indexed (8.3s)
  Rust: 1,293 symbols indexed (12.1s)

Scan complete!
  Total duration: 78.2s
  Code intelligence: enabled (TypeScript, Rust)
```

## Implementation Plan

### Ticket 1: Indexer Detection
- Create `crates/maproom/src/scan/scip_detection.rs`
- Implement detection for scip-typescript
- Implement detection for rust-analyzer
- Implement detection for scip-python
- Unit tests with mocked commands

### Ticket 2: Project Detection
- Create project type detection logic
- Handle tsconfig.json, Cargo.toml, pyproject.toml
- Detect monorepos (lerna, pnpm workspaces)
- Unit tests with fixture directories

### Ticket 3: Indexer Runner
- Create `crates/maproom/src/scan/scip_runner.rs`
- Implement TypeScript indexer execution
- Implement Rust indexer execution
- Implement Python indexer execution
- Handle timeouts and errors gracefully

### Ticket 4: Scan Integration
- Add SCIP stage to scan command
- Add `--skip-scip` and `--scip-only` flags
- Integrate with progress reporting
- Import SCIP results into database

### Ticket 5: End-to-End Testing
- Test on TypeScript-only repo
- Test on Rust-only repo
- Test on mixed language repo
- Test with missing indexers
- Test with invalid project configs

## Dependencies

**Requires:**
- Project 1 (Schema & Import) - `import_scip()` function
- Project 2 (Query Layer) - For verification testing
- Project 3 (MCP Tools) - For end-to-end verification
- Project 4 (Multi-Language) - Validates all languages work

**Required By:**
- End users wanting zero-config code intelligence

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Indexer not installed | Low | Clear message, graceful skip |
| Project config invalid | Medium | Detect and warn, skip language |
| Indexer hangs/times out | Medium | 5-minute timeout, warn and continue |
| node_modules missing | Medium | Check upfront, clear error message |
| Monorepo complexity | Medium | Start with root-only, improve later |

## Estimated Effort

- **Duration:** 3-4 days
- **Tickets:** 5
- **Files Created/Modified:** 4-6 files
- **Dependencies:** `which` crate for binary detection

## User Experience

### Before (no SCIP)
```
User: Find where authenticate is defined
Claude: *runs grep, guesses based on filename*
Claude: "I found what might be the definition in src/auth.ts"
```

### After (with SCIP)
```
User: Find where authenticate is defined
Claude: *uses scip_goto_definition*
Claude: "The authenticate function is defined at src/auth/authenticate.ts:42"
```

### First-Time User Experience
```bash
$ maproom scan
Scanning repository: my-app

Detecting SCIP indexers...
  ✗ scip-typescript not installed
  
TIP: Install SCIP indexers for precise code intelligence:
  npm install -g @sourcegraph/scip-typescript
  
Continuing without SCIP (grep-based navigation still available)...
```

## Success Metrics

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Detection accuracy | 99%+ correct detection | Test on diverse repos |
| Graceful degradation | 0 crashes from missing indexers | Error handling tests |
| User clarity | Users understand what happened | Review output messages |
| End-to-end latency | < 2min for 100k LOC with SCIP | Benchmark tests |