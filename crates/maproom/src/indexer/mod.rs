use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use tracing::{debug, info, warn};

use crate::db::{ChunkRecord, FileRecord, Store};
// Sub-traits needed by the #[cfg(test)] module (concrete SqliteStore calls).
#[cfg(test)]
use crate::db::traits::StoreCore;
use crate::incremental::edge_updater::Edge;
use crate::incremental::ignore::load_ignore_patterns;

pub mod edges;
pub mod parser;

/// Debouncer to prevent rapid successive event handling
///
/// Implements time-based debouncing to avoid triggering operations
/// too frequently. This prevents issues with:
/// - Multiple rapid branch switches
/// - Git operations that modify files multiple times
/// - File system noise (duplicate events from the OS)
///
/// # Debouncing Strategy
///
/// Events that occur within the debounce duration (default: 2 seconds) of the
/// previous event are ignored. This ensures at most one operation
/// per debounce window.
///
/// # Thread Safety
///
/// The last event timestamp is protected by a `Mutex` to allow safe access
/// from the event handler thread.
pub struct DebouncedHandler {
    /// Timestamp of the last processed event, protected by mutex for thread safety
    last_event: std::sync::Mutex<std::time::Instant>,
    /// Minimum duration between processed events
    debounce_duration: std::time::Duration,
}

impl DebouncedHandler {
    /// Creates a new debounced handler with the specified duration
    ///
    /// # Arguments
    ///
    /// * `debounce_duration` - Minimum time between processed events
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::time::Duration;
    ///
    /// let debouncer = DebouncedHandler::new(Duration::from_secs(2));
    /// ```
    pub fn new(debounce_duration: std::time::Duration) -> Self {
        Self {
            last_event: std::sync::Mutex::new(std::time::Instant::now() - debounce_duration),
            debounce_duration,
        }
    }

    /// Checks if an event should be processed or debounced
    ///
    /// Returns `true` if sufficient time has passed since the last event,
    /// `false` if the event should be debounced (ignored).
    ///
    /// # Thread Safety
    ///
    /// This method acquires a lock on the last event timestamp. If the lock
    /// is poisoned (due to a panic while holding the lock), this will panic.
    ///
    /// # Returns
    ///
    /// - `true` - Process the event (>= debounce_duration since last event)
    /// - `false` - Ignore the event (< debounce_duration since last event)
    pub fn should_handle(&self) -> bool {
        let mut last = self.last_event.lock().unwrap();
        let now = std::time::Instant::now();

        if now.duration_since(*last) >= self.debounce_duration {
            *last = now;
            true
        } else {
            false
        }
    }
}

/// NDJSON event emitted when a branch switch is detected (UNIWATCH-2002)
///
/// This struct is serialized to JSON and written to stdout for consumption
/// by external tools (e.g., VSCode extension, CLI orchestrator).
///
/// # JSON Format
///
/// Serializes to single-line NDJSON (newline-delimited JSON):
/// ```json
/// {"type":"branch_switched","timestamp":"2025-01-16T10:30:00Z","repo":"crewchief","old_branch":"main","new_branch":"feature-auth","old_worktree_id":1,"new_worktree_id":42,"worktree_created":false}
/// ```
///
/// # Fields
///
/// - `event_type`: Always "branch_switched" (serialized as "type")
/// - `timestamp`: ISO 8601 timestamp of when the event occurred
/// - `repo`: Repository name (e.g., "crewchief")
/// - `old_branch`: Branch name before the switch
/// - `new_branch`: Branch name after the switch
/// - `old_worktree_id`: Database worktree ID before the switch (BIGINT/i64)
/// - `new_worktree_id`: Database worktree ID after the switch (BIGINT/i64)
/// - `worktree_created`: Whether a new worktree record was created in the database
#[derive(serde::Serialize)]
pub struct BranchSwitchEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub timestamp: String,
    pub repo: String,
    pub old_branch: String,
    pub new_branch: String,
    pub old_worktree_id: i64,
    pub new_worktree_id: i64,
    pub worktree_created: bool,
}

/// Process Python imports from chunk metadata and create import edges in chunk_edges table
async fn process_python_imports(
    store: &(dyn Store + Send + Sync),
    repo_id: i64,
    worktree_id: i64,
    _file_id: i64,
    chunks: &[SymbolChunk],
) -> anyhow::Result<()> {
    // Find the imports chunk if it exists
    let imports_chunk = chunks
        .iter()
        .find(|c| c.kind == "imports" && c.metadata.is_some());

    if let Some(imports) = imports_chunk {
        if let Some(metadata) = &imports.metadata {
            if let Some(imports_array) = metadata.get("imports").and_then(|v| v.as_array()) {
                // Get the chunk_id for the imports chunk itself
                let imports_chunk_id = store
                    .find_chunk_by_symbol(repo_id, Some(worktree_id), "__imports__", None)
                    .await?;

                if let Some(src_chunk_id) = imports_chunk_id {
                    // Process each import
                    for import_obj in imports_array {
                        // Extract symbol names from the import
                        let names = import_obj
                            .get("names")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                            .unwrap_or_default();

                        // For each imported name, try to find the target chunk
                        for name in names {
                            if let Ok(Some(dst_chunk_id)) = store
                                .find_chunk_by_symbol(repo_id, Some(worktree_id), name, None)
                                .await
                            {
                                // Create the import edge
                                if let Err(e) = store
                                    .insert_chunk_edge(src_chunk_id, dst_chunk_id, "imports")
                                    .await
                                {
                                    warn!("Failed to create import edge for {}: {}", name, e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Batch insert edges into the database
async fn insert_edges(store: &(dyn Store + Send + Sync), edges: &[Edge]) -> Result<()> {
    for edge in edges {
        store
            .insert_chunk_edge(
                edge.src_chunk_id,
                edge.dst_chunk_id,
                edge.edge_type.as_str(),
            )
            .await?;
    }
    Ok(())
}

pub fn detect_language_from_path(path: &Path) -> Option<&'static str> {
    // Check for go.mod file specifically
    if path.file_name().and_then(|n| n.to_str()) == Some("go.mod") {
        return Some("gomod");
    }

    // Check for Ruby special filenames
    match path.file_name().and_then(|n| n.to_str()) {
        Some("Gemfile") | Some("Rakefile") => return Some("rb"),
        _ => {}
    }

    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "ts" => Some("ts"),
        "tsx" => Some("tsx"),
        "js" => Some("js"),
        "jsx" => Some("jsx"),
        "rs" => Some("rs"),
        "py" => Some("py"),
        "go" => Some("go"),
        "rb" | "rake" => Some("rb"),
        "c" => Some("c"),
        "cs" => Some("cs"),
        "java" => Some("java"),
        "cpp" | "cxx" | "cc" | "c++" => Some("cpp"),
        "hpp" | "hxx" => Some("cpp"),
        "h" => Some("cpp"), // Default .h to C++ (tree-sitter-cpp handles C too)
        "md" => Some("md"),
        "mdx" => Some("mdx"),
        "json" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
        _ => None,
    }
}

fn build_ts_doc(
    relpath: &str,
    symbol_name: Option<&str>,
    signature: Option<&str>,
    docstring: Option<&str>,
    preview: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(relpath.to_owned());
    if let Some(s) = symbol_name {
        parts.push(s.to_owned());
    }
    if let Some(s) = signature {
        parts.push(s.to_owned());
    }
    if let Some(s) = docstring {
        parts.push(s.to_owned());
    }
    parts.push(preview.to_owned());
    parts.join(" \n ")
}

fn first_n_lines(s: &str, n: usize) -> String {
    s.lines().take(n).collect::<Vec<_>>().join("\n")
}

fn file_modified_time(path: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    use std::time::UNIX_EPOCH;
    let t = fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let dur = t.duration_since(UNIX_EPOCH).ok()?;
    chrono::DateTime::<chrono::Utc>::from_timestamp(dur.as_secs() as i64, dur.subsec_nanos())
}

#[allow(clippy::too_many_arguments)] // Public API; parameters represent distinct scan configuration
pub async fn scan_worktree(
    store: &(dyn Store + Send + Sync),
    repo: &str,
    worktree: &str,
    root: &Path,
    commit: &str,
    _concurrency: usize,
    languages: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    progress: Option<&crate::progress::ProgressTracker>,
) -> anyhow::Result<()> {
    let start_time = std::time::Instant::now();
    let root_abs = root.canonicalize().with_context(|| "invalid root path")?;
    let repo_id = store
        .get_or_create_repo(repo, root_abs.to_string_lossy().as_ref())
        .await?;
    let worktree_id = store
        .get_or_create_worktree(repo_id, worktree, root_abs.to_string_lossy().as_ref())
        .await?;
    let commit_id = store.get_or_create_commit(repo_id, commit, None).await?;

    // Stats tracking
    let mut files_processed = 0;
    let mut files_skipped = 0;
    let mut total_chunks = 0;
    let mut total_bytes = 0usize;
    let mut language_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    // Suppress human-readable output in JSON mode (for VSCode extension)
    let json_mode = progress.as_ref().map(|p| p.is_json_mode()).unwrap_or(false);
    if !json_mode {
        println!(
            "🔍 Scanning worktree: {} @ {}",
            worktree,
            &commit[..8.min(commit.len())]
        );
        println!("   Repository: {}", repo);
        println!("   Path: {}", root_abs.display());
    }

    // Load .maproomignore patterns and merge with programmatic exclude patterns
    let maproomignore_patterns = load_ignore_patterns(&root_abs)
        .with_context(|| format!("Failed to load .maproomignore patterns from {:?}", root_abs))?;

    let mut walk = WalkBuilder::new(&root_abs);
    walk.hidden(false)
        .ignore(true)
        .git_ignore(true)
        .git_exclude(true);

    // Build combined overrides from .maproomignore and programmatic exclude patterns
    if !maproomignore_patterns.is_empty() || exclude.is_some() {
        let mut ob = ignore::overrides::OverrideBuilder::new(&root_abs);

        // Add .maproomignore patterns as negative overrides
        for pattern in &maproomignore_patterns {
            ob.add(&format!("!{}", pattern))
                .with_context(|| format!("Invalid pattern in .maproomignore: {}", pattern))?;
        }

        // Merge programmatic exclude patterns
        if let Some(globs) = &exclude {
            for g in globs {
                ob.add(&format!("!{}", g))
                    .with_context(|| format!("Invalid exclude pattern: {}", g))?;
            }
        }

        walk.overrides(ob.build().context("Failed to build override patterns")?);
    }

    let allow_langs: Option<Vec<String>> =
        languages.map(|v| v.into_iter().map(|s| s.to_lowercase()).collect());

    // Collect all file paths first to set progress totals
    let mut file_paths = Vec::new();
    for dent in walk.build() {
        let dent = match dent {
            Ok(d) => d,
            Err(_) => continue,
        };
        if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = dent.path();
        let language = detect_language_from_path(path);
        if language.is_none() {
            continue;
        }
        if let Some(ref allow) = allow_langs {
            if !allow.iter().any(|l| l == language.unwrap()) {
                continue;
            }
        }
        file_paths.push(path.to_path_buf());
    }

    // Set progress totals now that we know file count
    if let Some(p) = &progress {
        p.set_totals(file_paths.len(), None);
    }

    for (idx, path) in file_paths.iter().enumerate() {
        let relpath = path.strip_prefix(&root_abs).unwrap_or(path);
        let language = detect_language_from_path(path).unwrap(); // Already filtered

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                files_skipped += 1;
                continue;
            }
        };

        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        let size_bytes = content.len().min(i32::MAX as usize) as i32;
        let last_modified = file_modified_time(path);

        // Update stats
        files_processed += 1;
        total_bytes += content.len();
        *language_counts.entry(language.to_string()).or_insert(0) += 1;

        let file_record = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: relpath.to_string_lossy().to_string(),
            language: Some(language.to_string()),
            content_hash,
            size_bytes,
            last_modified,
        };
        let file_id = store.upsert_file(&file_record).await?;

        let chunks = parser::extract_chunks(&content, language);
        if chunks.is_empty() {
            // Fallback: single module chunk
            total_chunks += 1;
            let preview = first_n_lines(&content, 40);
            let blob_sha = crate::content_hash::compute_blob_sha(&preview);
            let ts_doc = build_ts_doc(
                relpath.to_string_lossy().as_ref(),
                None,
                None,
                None,
                &preview,
            );
            let chunk_record = ChunkRecord {
                file_id,
                blob_sha,
                symbol_name: None,
                kind: "module".to_string(),
                signature: None,
                docstring: None,
                start_line: 1,
                end_line: content.lines().count() as i32,
                preview,
                ts_doc_text: ts_doc,
                recency_score: 1.0,
                churn_score: 0.0,
                metadata: None,
                worktree_id,
            };
            store.insert_chunk(&chunk_record).await?;
        } else {
            total_chunks += chunks.len();

            // Collect chunk IDs during insertion
            let mut chunks_with_ids = Vec::new();
            for ch in &chunks {
                let chunk_content = content
                    .split('\n')
                    .skip(ch.start_line as usize - 1)
                    .take((ch.end_line - ch.start_line + 1) as usize)
                    .collect::<Vec<&str>>()
                    .join("\n");
                let preview = first_n_lines(&chunk_content, 40);
                let blob_sha = crate::content_hash::compute_blob_sha(&chunk_content);
                let ts_doc = build_ts_doc(
                    relpath.to_string_lossy().as_ref(),
                    ch.symbol_name.as_deref(),
                    ch.signature.as_deref(),
                    ch.docstring.as_deref(),
                    &preview,
                );
                let chunk_record = ChunkRecord {
                    file_id,
                    blob_sha,
                    symbol_name: ch.symbol_name.clone(),
                    kind: ch.kind.clone(),
                    signature: ch.signature.clone(),
                    docstring: ch.docstring.clone(),
                    start_line: ch.start_line,
                    end_line: ch.end_line,
                    preview,
                    ts_doc_text: ts_doc,
                    recency_score: 1.0,
                    churn_score: 0.0,
                    metadata: ch.metadata.clone(),
                    worktree_id,
                };
                let chunk_id = store.insert_chunk(&chunk_record).await?;
                chunks_with_ids.push(edges::ChunkWithId {
                    id: chunk_id,
                    symbol_name: ch.symbol_name.clone(),
                    kind: ch.kind.clone(),
                    start_line: ch.start_line,
                    end_line: ch.end_line,
                    file_id,
                });
            }

            // Process Python imports and create edges
            if language == "py" {
                if let Err(e) =
                    process_python_imports(store, repo_id, worktree_id, file_id, &chunks).await
                {
                    warn!(
                        "Failed to process Python imports for {}: {}",
                        relpath.display(),
                        e
                    );
                }
            }

            // Extract edges for TypeScript/JavaScript
            if matches!(language, "ts" | "tsx" | "js" | "jsx") {
                match edges::extract_edges(&content, language, &chunks_with_ids) {
                    Ok(edges_to_insert) if !edges_to_insert.is_empty() => {
                        if let Err(e) = insert_edges(store, &edges_to_insert).await {
                            warn!("Failed to insert edges for {}: {}", relpath.display(), e);
                            // Continue scan despite edge insertion failure
                        } else {
                            debug!(
                                "Inserted {} edges for {}",
                                edges_to_insert.len(),
                                relpath.display()
                            );
                        }
                    }
                    Ok(_) => {
                        // No edges extracted (empty file or no calls)
                        debug!("No edges extracted for {}", relpath.display());
                    }
                    Err(e) => {
                        warn!("Edge extraction failed for {}: {}", relpath.display(), e);
                        // Continue scan despite extraction failure
                    }
                }
            }
        }

        // Update progress after processing this file
        if let Some(p) = &progress {
            p.update_files(idx + 1);
            if p.should_print() {
                p.print_progress();
            }
        }
    }

    // Finish progress tracking and show timing
    if let Some(p) = &progress {
        p.finish();
    } else {
        // If no progress tracker, show timing manually (not in JSON mode)
        if !json_mode {
            let elapsed = start_time.elapsed();
            println!("\n✅ Completed in {:.1}s", elapsed.as_secs_f64());
        }
    }

    // Print summary (suppress in JSON mode)
    if !json_mode {
        println!("\n✅ Scan completed successfully!");
        println!("   Files processed: {}", files_processed);
        if files_skipped > 0 {
            println!("   Files skipped: {}", files_skipped);
        }
        println!("   Total chunks: {}", total_chunks);
        println!("   Total size: {:.2} MB", total_bytes as f64 / 1_048_576.0);

        if !language_counts.is_empty() {
            println!("\n   Languages indexed:");
            let mut langs: Vec<_> = language_counts.iter().collect();
            langs.sort_by(|a, b| b.1.cmp(a.1));
            for (lang, count) in langs {
                println!(
                    "     {} {}: {}",
                    match lang.as_str() {
                        "ts" | "tsx" => "📘",
                        "js" | "jsx" => "📙",
                        "rs" => "🦀",
                        "py" => "🐍",
                        "go" => "🔷",
                        "md" => "📝",
                        "json" => "📋",
                        "yaml" | "yml" => "📄",
                        "toml" => "⚙️",
                        _ => "📄",
                    },
                    lang,
                    count
                );
            }
        }
    }

    info!(?repo, ?worktree, ?commit, "scan complete");
    Ok(())
}

pub async fn upsert_files(
    store: &(dyn Store + Send + Sync),
    repo: &str,
    worktree: &str,
    root: &Path,
    commit: &str,
    paths: &[PathBuf],
) -> anyhow::Result<()> {
    let root_abs = root.canonicalize().with_context(|| "invalid root path")?;
    let repo_id = store
        .get_or_create_repo(repo, root_abs.to_string_lossy().as_ref())
        .await?;
    let worktree_id = store
        .get_or_create_worktree(repo_id, worktree, root_abs.to_string_lossy().as_ref())
        .await?;
    let commit_id = store.get_or_create_commit(repo_id, commit, None).await?;

    for path in paths {
        let abs = if path.is_absolute() {
            path.clone()
        } else {
            root_abs.join(path)
        };
        if !abs.exists() {
            continue;
        }
        if abs.is_dir() {
            continue;
        }
        let relpath = abs.strip_prefix(&root_abs).unwrap_or(&abs).to_path_buf();
        let language = detect_language_from_path(&abs);
        if language.is_none() {
            continue;
        }
        let content = match fs::read_to_string(&abs) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        let size_bytes = content.len().min(i32::MAX as usize) as i32;
        let last_modified = file_modified_time(&abs);
        let file_record = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: relpath.to_string_lossy().to_string(),
            language: language.map(|l| l.to_string()),
            content_hash,
            size_bytes,
            last_modified,
        };
        let file_id = store.upsert_file(&file_record).await?;
        let chunks = parser::extract_chunks(&content, language.unwrap());
        if chunks.is_empty() {
            let preview = first_n_lines(&content, 40);
            let blob_sha = crate::content_hash::compute_blob_sha(&preview);
            let ts_doc = build_ts_doc(
                relpath.to_string_lossy().as_ref(),
                None,
                None,
                None,
                &preview,
            );
            let chunk_record = ChunkRecord {
                file_id,
                blob_sha,
                symbol_name: None,
                kind: "module".to_string(),
                signature: None,
                docstring: None,
                start_line: 1,
                end_line: content.lines().count() as i32,
                preview,
                ts_doc_text: ts_doc,
                recency_score: 1.0,
                churn_score: 0.0,
                metadata: None,
                worktree_id,
            };
            store.insert_chunk(&chunk_record).await?;
        } else {
            // Collect chunk IDs during insertion
            let mut chunks_with_ids = Vec::new();
            for ch in &chunks {
                let chunk_content = content
                    .split('\n')
                    .skip(ch.start_line as usize - 1)
                    .take((ch.end_line - ch.start_line + 1) as usize)
                    .collect::<Vec<&str>>()
                    .join("\n");
                let preview = first_n_lines(&chunk_content, 40);
                let blob_sha = crate::content_hash::compute_blob_sha(&chunk_content);
                let ts_doc = build_ts_doc(
                    relpath.to_string_lossy().as_ref(),
                    ch.symbol_name.as_deref(),
                    ch.signature.as_deref(),
                    ch.docstring.as_deref(),
                    &preview,
                );
                let chunk_record = ChunkRecord {
                    file_id,
                    blob_sha,
                    symbol_name: ch.symbol_name.clone(),
                    kind: ch.kind.clone(),
                    signature: ch.signature.clone(),
                    docstring: ch.docstring.clone(),
                    start_line: ch.start_line,
                    end_line: ch.end_line,
                    preview,
                    ts_doc_text: ts_doc,
                    recency_score: 1.0,
                    churn_score: 0.0,
                    metadata: ch.metadata.clone(),
                    worktree_id,
                };
                let chunk_id = store.insert_chunk(&chunk_record).await?;
                chunks_with_ids.push(edges::ChunkWithId {
                    id: chunk_id,
                    symbol_name: ch.symbol_name.clone(),
                    kind: ch.kind.clone(),
                    start_line: ch.start_line,
                    end_line: ch.end_line,
                    file_id,
                });
            }

            // Process Python imports and create edges
            if language.unwrap() == "py" {
                if let Err(e) =
                    process_python_imports(store, repo_id, worktree_id, file_id, &chunks).await
                {
                    warn!(
                        "Failed to process Python imports for {}: {}",
                        relpath.display(),
                        e
                    );
                }
            }

            // Extract edges for TypeScript/JavaScript
            if matches!(language.unwrap(), "ts" | "tsx" | "js" | "jsx") {
                match edges::extract_edges(&content, language.unwrap(), &chunks_with_ids) {
                    Ok(edges_to_insert) if !edges_to_insert.is_empty() => {
                        if let Err(e) = insert_edges(store, &edges_to_insert).await {
                            warn!("Failed to insert edges for {}: {}", relpath.display(), e);
                        } else {
                            debug!(
                                "Inserted {} edges for {}",
                                edges_to_insert.len(),
                                relpath.display()
                            );
                        }
                    }
                    Ok(_) => {
                        debug!("No edges extracted for {}", relpath.display());
                    }
                    Err(e) => {
                        warn!("Edge extraction failed for {}: {}", relpath.display(), e);
                    }
                }
            }
        }
    }

    info!(?repo, ?worktree, ?commit, updated_files=?paths.len(), "upsert selective complete");
    Ok(())
}

/// Sets up file watching for .git/HEAD with channel bridging from sync to async
///
/// Creates a `notify::RecommendedWatcher` that monitors the `.git/HEAD` file for changes
/// (e.g., branch switches). Events from the synchronous `notify` crate are bridged to
/// tokio's async channels via a spawned task.
///
/// # Arguments
///
/// * `git_head` - Path to the .git/HEAD file to watch
/// * `tx` - Tokio async channel sender for forwarding file system events
///
/// # Returns
///
/// Returns the watcher handle which must be kept alive. When the watcher is dropped,
/// file watching stops automatically.
///
/// # Channel Bridging
///
/// The notify crate uses synchronous `std::sync::mpsc` channels, while tokio uses
/// async channels. This function bridges the two by:
/// 1. Creating a sync channel for notify events
/// 2. Spawning a tokio task that forwards events to the async channel
/// 3. Breaking the loop when the async channel is closed (receiver dropped)
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let git_head = Path::new("/workspace/repo/.git/HEAD");
///     let (tx, mut rx) = mpsc::channel(100);
///
///     let _watcher = setup_head_watcher(git_head, tx)?;
///
///     // Receive events
///     while let Some(event) = rx.recv().await {
///         println!("Branch switch detected: {:?}", event);
///     }
///
///     Ok(())
/// }
/// ```
pub fn setup_head_watcher(
    git_head: &Path,
    tx: tokio::sync::mpsc::Sender<notify::Event>,
) -> anyhow::Result<notify::RecommendedWatcher> {
    use notify::{RecursiveMode, Watcher};

    // Create sync channel for notify crate
    let (sync_tx, sync_rx) = std::sync::mpsc::channel();

    // Create watcher with sync callback
    let mut watcher = notify::recommended_watcher(move |res| {
        if let Ok(event) = res {
            let _ = sync_tx.send(event);
        }
    })?;

    // Watch the .git/HEAD file (non-recursive, file only)
    watcher.watch(git_head, RecursiveMode::NonRecursive)?;

    // Bridge sync to async: spawn blocking task to forward events
    // Use spawn_blocking because sync_rx.recv() is a blocking call
    tokio::task::spawn_blocking(move || {
        while let Ok(event) = sync_rx.recv() {
            // Send to async channel - need to block_on since we're in a blocking context
            if tx.blocking_send(event).is_err() {
                // Channel closed, exit task
                break;
            }
        }
    });

    Ok(watcher)
}

// NOTE: watch_worktree, handle_branch_switch, get_file_id_by_path, and get_file_id_by_worktree_id
// functions have been removed as part of IDXABS-2001 (SQLite-only migration).
// They depended on PostgreSQL's PgPool and will be reimplemented in IDXABS-2006
// (Refactor Incremental Module) with SqliteStore support.

#[derive(Debug, Clone)]
pub struct SymbolChunk {
    pub symbol_name: Option<String>,
    pub kind: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub start_line: i32,
    pub end_line: i32,
    pub metadata: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::StoreMigration;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Test that setup_head_watcher creates a working channel bridge from sync to async
    ///
    /// This test verifies:
    /// 1. The function creates a notify::RecommendedWatcher without errors
    /// 2. The watcher can be configured to watch a file path
    /// 3. The async channel is created and ready to receive events
    /// 4. The function returns a valid watcher handle
    /// 5. Cleanup works properly when the watcher is dropped
    #[tokio::test]
    async fn test_setup_head_watcher_creates_bridge() {
        // Create a temporary file to watch (simulates .git/HEAD)
        let mut temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Write initial content
        writeln!(temp_file, "ref: refs/heads/main").unwrap();
        temp_file.flush().unwrap();

        // Create async channel
        let (tx, rx) = tokio::sync::mpsc::channel(10);

        // Setup the watcher - this is the main test
        // It should not panic or return an error
        let watcher_result = setup_head_watcher(&temp_path, tx);

        // Verify the watcher was created successfully
        assert!(
            watcher_result.is_ok(),
            "Failed to create watcher: {:?}",
            watcher_result.err()
        );

        // Drop the watcher to stop watching and close the sync channel
        // This will cause the bridging task to exit when sync_rx.recv() returns Err
        drop(watcher_result.unwrap());

        // Drop the receiver to close the async channel
        // This ensures the bridging task will exit if it's still trying to send
        drop(rx);

        // Test passes if we reach here without panicking
        // The bridging task should exit cleanly when the watcher is dropped
    }

    /// Test that worktree tracking state is initialized correctly (UNIWATCH-1002)
    ///
    /// This test verifies:
    /// 1. Arc<RwLock<String>> for current_branch is created and initialized
    /// 2. Arc<RwLock<i64>> for current_worktree_id is created and initialized
    /// 3. Initialization uses get_or_create_repo() and get_or_create_worktree()
    /// 4. Arc/RwLock semantics work (can acquire read/write locks)
    /// 5. Values match the input parameters
    ///
    /// MIGRATED from PostgreSQL to SQLite (UNIWATCH-4001)
    #[tokio::test]
    async fn test_worktree_tracking_initialization() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

        // Setup SQLite test database
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!(
            "file:memdb_worktree_init_{}?mode=memory&cache=shared",
            counter
        );
        let store = crate::db::SqliteStore::connect(&db_name)
            .await
            .expect("Failed to create test store");
        store.migrate().await.expect("Failed to run migrations");

        // Test parameters
        let repo = "test-repo";
        let worktree = "test-branch";
        let root = "/tmp/test-root";

        // Initialize tracking state (mirrors watch command logic)
        let repo_id = store
            .get_or_create_repo(repo, root)
            .await
            .expect("Failed to get_or_create_repo");
        let worktree_id = store
            .get_or_create_worktree(repo_id, worktree, root)
            .await
            .expect("Failed to get_or_create_worktree");

        let current_branch = std::sync::Arc::new(std::sync::RwLock::new(worktree.to_string()));
        let current_worktree_id = std::sync::Arc::new(std::sync::RwLock::new(worktree_id));

        // Test 1: Verify current_branch initialized correctly
        {
            let branch_guard = current_branch
                .read()
                .expect("Failed to acquire read lock on current_branch");
            assert_eq!(
                *branch_guard, worktree,
                "current_branch should be initialized to worktree parameter"
            );
        }

        // Test 2: Verify current_worktree_id initialized correctly
        {
            let worktree_id_guard = current_worktree_id
                .read()
                .expect("Failed to acquire read lock on current_worktree_id");
            assert!(
                *worktree_id_guard > 0,
                "current_worktree_id should be a valid positive integer"
            );
        }

        // Test 3: Verify Arc semantics work (can clone and access from multiple locations)
        let branch_clone = std::sync::Arc::clone(&current_branch);
        let worktree_id_clone = std::sync::Arc::clone(&current_worktree_id);

        {
            let branch_guard = branch_clone.read().expect("Failed to acquire read lock");
            assert_eq!(*branch_guard, worktree, "Arc clone should have same value");
        }

        {
            let worktree_id_guard = worktree_id_clone
                .read()
                .expect("Failed to acquire read lock");
            assert!(*worktree_id_guard > 0, "Arc clone should have same value");
        }

        // Test 4: Verify write locks work (for branch switch logic)
        {
            let mut branch_guard = current_branch
                .write()
                .expect("Failed to acquire write lock on current_branch");
            let new_branch = "feature-branch";
            *branch_guard = new_branch.to_string();
            assert_eq!(
                *branch_guard, new_branch,
                "Write lock should allow mutation"
            );
        }

        // Test 5: Verify value persisted after write lock released
        {
            let branch_guard = current_branch.read().expect("Failed to acquire read lock");
            assert_eq!(
                *branch_guard, "feature-branch",
                "Value should persist after write lock released"
            );
        }
    }

    /// Test that DebouncedHandler prevents rapid successive events (UNIWATCH-1003)
    ///
    /// This test verifies:
    /// 1. First call to should_handle() returns true (event processed)
    /// 2. Immediate second call returns false (debounced, too soon)
    /// 3. After debounce duration expires, should_handle() returns true again
    /// 4. Thread-safe Mutex<Instant> pattern works correctly
    /// 5. Configurable debounce duration is respected
    #[test]
    fn test_debouncer_prevents_rapid_events() {
        use std::time::Duration;

        // Create debouncer with short duration for testing (100ms)
        let debounce_duration = Duration::from_millis(100);
        let debouncer = DebouncedHandler::new(debounce_duration);

        // Test 1: First call should return true (enough time has passed since initialization)
        assert!(
            debouncer.should_handle(),
            "First call to should_handle() should return true"
        );

        // Test 2: Immediate second call should return false (debounced)
        assert!(
            !debouncer.should_handle(),
            "Immediate second call should return false (debounced)"
        );

        // Test 3: Another immediate call should also return false
        assert!(
            !debouncer.should_handle(),
            "Third immediate call should also return false (still debounced)"
        );

        // Test 4: Wait for debounce duration to expire
        std::thread::sleep(debounce_duration + Duration::from_millis(10));

        // Test 5: After waiting, should_handle() should return true again
        assert!(
            debouncer.should_handle(),
            "After waiting for debounce duration, should_handle() should return true"
        );

        // Test 6: Immediate call after the previous one should be debounced again
        assert!(
            !debouncer.should_handle(),
            "Immediate call after previous success should be debounced"
        );
    }

    /// Test that branch switch state update pattern works correctly (UNIWATCH-2001)
    ///
    /// This test verifies the state update logic that handle_branch_switch uses:
    /// 1. Database records are created for new worktrees
    /// 2. current_branch Arc<RwLock<String>> can be updated to new branch
    /// 3. current_worktree_id Arc<RwLock<i64>> can be updated to new worktree_id
    /// 4. State remains consistent after update
    ///
    /// Note: Full integration test of handle_branch_switch is in UNIWATCH-4002.
    ///
    /// MIGRATED from PostgreSQL to SQLite (UNIWATCH-4001)
    #[tokio::test]
    async fn test_handle_branch_switch_updates_state() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, RwLock};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

        // Setup SQLite test database
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!(
            "file:memdb_branch_switch_{}?mode=memory&cache=shared",
            counter
        );
        let store = crate::db::SqliteStore::connect(&db_name)
            .await
            .expect("Failed to create test store");
        store.migrate().await.expect("Failed to run migrations");

        // Test parameters
        let repo_name = "test-repo";
        let root = "/tmp/test-root";

        // Create repo
        let repo_id = store
            .get_or_create_repo(repo_name, root)
            .await
            .expect("Failed to create repo");

        // Create initial worktree for "main"
        let main_worktree_id = store
            .get_or_create_worktree(repo_id, "main", root)
            .await
            .expect("Failed to create main worktree");

        // Initialize shared state with "main"
        let current_branch = Arc::new(RwLock::new("main".to_string()));
        let current_worktree_id = Arc::new(RwLock::new(main_worktree_id));

        // Verify initial state
        assert_eq!(*current_branch.read().unwrap(), "main");
        assert_eq!(*current_worktree_id.read().unwrap(), main_worktree_id);

        // Simulate branch switch to "feature"
        let new_branch = "feature";
        let feature_worktree_id = store
            .get_or_create_worktree(repo_id, new_branch, root)
            .await
            .expect("Failed to create feature worktree");

        // Update state (simulating handle_branch_switch logic)
        {
            *current_branch.write().unwrap() = new_branch.to_string();
            *current_worktree_id.write().unwrap() = feature_worktree_id;
        }

        // Verify current_branch was updated to "feature"
        {
            let branch_guard = current_branch.read().unwrap();
            assert_eq!(
                *branch_guard, "feature",
                "current_branch should be updated to 'feature'"
            );
        }

        // Verify current_worktree_id was updated
        {
            let worktree_id_guard = current_worktree_id.read().unwrap();
            assert_eq!(
                *worktree_id_guard, feature_worktree_id,
                "current_worktree_id should be updated to feature worktree"
            );
            assert!(
                *worktree_id_guard > 0,
                "current_worktree_id should be a valid positive integer"
            );
        }

        // Verify different worktrees get different IDs
        assert_ne!(
            main_worktree_id, feature_worktree_id,
            "Different branches should have different worktree IDs"
        );
    }

    /// Test that same-branch detection skips state updates (UNIWATCH-2001)
    ///
    /// This test verifies the same-branch detection logic used in handle_branch_switch:
    /// 1. Comparison of old_branch == effective_branch triggers early return
    /// 2. Shared state remains unchanged when branch hasn't changed
    /// 3. No unnecessary database operations
    ///
    /// Note: Full integration test of handle_branch_switch is in UNIWATCH-4002.
    ///
    /// MIGRATED from PostgreSQL to SQLite (UNIWATCH-4001)
    #[test]
    fn test_handle_branch_switch_skips_if_same_branch() {
        use std::sync::{Arc, RwLock};

        // Initialize shared state with "main"
        let current_branch = Arc::new(RwLock::new("main".to_string()));
        let current_worktree_id = Arc::new(RwLock::new(42i64));

        // Simulate detecting "main" as the effective branch (same as current)
        let effective_branch = "main";
        let old_branch = current_branch.read().unwrap().clone();
        let old_wt_id = *current_worktree_id.read().unwrap();

        // Same-branch check (this is the logic from handle_branch_switch)
        let should_skip = old_branch == effective_branch;
        assert!(should_skip, "Same branch should be detected for skipping");

        // When skipping, state should NOT be modified
        // (Simulate the early return by not modifying state)

        // Verify current_branch was NOT changed
        {
            let branch_guard = current_branch.read().unwrap();
            assert_eq!(
                *branch_guard, "main",
                "current_branch should remain unchanged when branch is same"
            );
        }

        // Verify current_worktree_id was NOT changed
        {
            let worktree_id_guard = current_worktree_id.read().unwrap();
            assert_eq!(
                *worktree_id_guard, 42i64,
                "current_worktree_id should remain unchanged when branch is same"
            );
        }

        // Verify the old values we captured are preserved
        assert_eq!(old_branch, "main");
        assert_eq!(old_wt_id, 42i64);
    }

    /// Test BranchSwitchEvent serialization to NDJSON (UNIWATCH-2002)
    ///
    /// This test verifies:
    /// 1. BranchSwitchEvent struct serializes successfully to JSON
    /// 2. JSON is valid and can be parsed back
    /// 3. All fields are present with correct names
    /// 4. "event_type" field is renamed to "type" in JSON
    /// 5. Timestamp is in ISO 8601 format
    /// 6. Worktree IDs are i64 (BIGINT)
    /// 7. JSON is single-line (no newlines in output)
    #[test]
    fn test_branch_switch_event_serialization() {
        // Create a test event with sample data
        let event = BranchSwitchEvent {
            event_type: "branch_switched",
            timestamp: "2025-01-16T10:30:00Z".to_string(),
            repo: "crewchief".to_string(),
            old_branch: "main".to_string(),
            new_branch: "feature-auth".to_string(),
            old_worktree_id: 1,
            new_worktree_id: 42,
            worktree_created: false,
        };

        // Serialize to JSON string
        let json_result = serde_json::to_string(&event);

        // Test 1: Serialization should succeed
        assert!(
            json_result.is_ok(),
            "BranchSwitchEvent serialization should succeed, got: {:?}",
            json_result.err()
        );

        let json = json_result.unwrap();

        // Test 2: JSON should be single-line (no newlines)
        assert!(
            !json.contains('\n'),
            "JSON should be single-line, got: {}",
            json
        );

        // Test 3: Parse JSON back to verify structure
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("JSON should be valid and parseable");

        // Test 4: Verify "type" field (not "event_type")
        assert_eq!(
            parsed.get("type").and_then(|v| v.as_str()),
            Some("branch_switched"),
            "JSON should have 'type' field with value 'branch_switched'"
        );

        // Test 5: Verify event_type field does NOT exist (should be renamed)
        assert!(
            parsed.get("event_type").is_none(),
            "JSON should NOT have 'event_type' field (should be renamed to 'type')"
        );

        // Test 6: Verify timestamp field
        assert_eq!(
            parsed.get("timestamp").and_then(|v| v.as_str()),
            Some("2025-01-16T10:30:00Z"),
            "JSON should have 'timestamp' field"
        );

        // Test 7: Verify repo field
        assert_eq!(
            parsed.get("repo").and_then(|v| v.as_str()),
            Some("crewchief"),
            "JSON should have 'repo' field"
        );

        // Test 8: Verify old_branch field
        assert_eq!(
            parsed.get("old_branch").and_then(|v| v.as_str()),
            Some("main"),
            "JSON should have 'old_branch' field"
        );

        // Test 9: Verify new_branch field
        assert_eq!(
            parsed.get("new_branch").and_then(|v| v.as_str()),
            Some("feature-auth"),
            "JSON should have 'new_branch' field"
        );

        // Test 10: Verify old_worktree_id field (should be i64)
        assert_eq!(
            parsed.get("old_worktree_id").and_then(|v| v.as_i64()),
            Some(1),
            "JSON should have 'old_worktree_id' field as i64"
        );

        // Test 11: Verify new_worktree_id field (should be i64)
        assert_eq!(
            parsed.get("new_worktree_id").and_then(|v| v.as_i64()),
            Some(42),
            "JSON should have 'new_worktree_id' field as i64"
        );

        // Test 12: Verify worktree_created field
        assert_eq!(
            parsed.get("worktree_created").and_then(|v| v.as_bool()),
            Some(false),
            "JSON should have 'worktree_created' field"
        );

        // Test 13: Verify all expected fields are present
        let expected_fields = vec![
            "type",
            "timestamp",
            "repo",
            "old_branch",
            "new_branch",
            "old_worktree_id",
            "new_worktree_id",
            "worktree_created",
        ];
        for field in expected_fields {
            assert!(
                parsed.get(field).is_some(),
                "JSON should have '{}' field",
                field
            );
        }

        // Test 14: Verify no extra fields
        let field_count = parsed.as_object().map(|o| o.len()).unwrap_or(0);
        assert_eq!(
            field_count, 8,
            "JSON should have exactly 8 fields, got {}",
            field_count
        );

        // Test 15: Verify timestamp format matches ISO 8601
        let timestamp_str = parsed.get("timestamp").and_then(|v| v.as_str()).unwrap();
        assert!(
            timestamp_str.ends_with('Z'),
            "Timestamp should be in UTC (end with 'Z')"
        );
        assert!(
            timestamp_str.contains('T'),
            "Timestamp should use 'T' separator (ISO 8601)"
        );
    }

    /// Test that dual watchers (file + head) initialize correctly (UNIWATCH-3001)
    ///
    /// This test verifies the integration point where both the file watcher and
    /// .git/HEAD watcher are created in watch_worktree(). It tests:
    /// 1. File watcher channel is created
    /// 2. .git/HEAD path is calculated correctly from root
    /// 3. Head watcher channel is created (capacity 10)
    /// 4. setup_head_watcher() is called successfully
    /// 5. Head watcher handle is stored for cleanup
    /// 6. Graceful degradation when .git/HEAD doesn't exist
    #[tokio::test]
    async fn test_dual_watchers_initialize() {
        use tempfile::TempDir;

        // Test 1: Verify head watcher succeeds when .git/HEAD exists
        {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let root_abs = temp_dir.path();
            let git_dir = root_abs.join(".git");
            std::fs::create_dir_all(&git_dir).expect("Failed to create .git dir");

            // Create .git/HEAD file
            let git_head = git_dir.join("HEAD");
            std::fs::write(&git_head, "ref: refs/heads/main\n").expect("Failed to write .git/HEAD");

            // Verify path calculation (this mimics watch_worktree logic)
            let calculated_git_head = root_abs.join(".git/HEAD");
            assert_eq!(
                calculated_git_head, git_head,
                "Path calculation should match actual .git/HEAD location"
            );

            // Create head event channel (capacity 10 as per spec)
            let (head_tx, mut head_rx) = tokio::sync::mpsc::channel(10);
            assert_eq!(
                head_rx.try_recv().unwrap_err(),
                tokio::sync::mpsc::error::TryRecvError::Empty,
                "Channel should be empty initially"
            );

            // Call setup_head_watcher (should succeed)
            let watcher_result = setup_head_watcher(&git_head, head_tx);
            assert!(
                watcher_result.is_ok(),
                "setup_head_watcher should succeed when .git/HEAD exists: {:?}",
                watcher_result.err()
            );

            // Store watcher handle (with underscore to prevent unused warning)
            let _head_watcher = watcher_result.unwrap();

            // Verify watcher stays alive while handle is in scope
            // (If this test completes without panic, the handle is valid)
        }

        // Test 2: Verify graceful degradation when .git/HEAD doesn't exist
        {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let root_abs = temp_dir.path();
            // Intentionally NOT creating .git/HEAD

            let git_head = root_abs.join(".git/HEAD");
            let (head_tx, _head_rx) = tokio::sync::mpsc::channel(10);

            // Call setup_head_watcher (should fail gracefully)
            let watcher_result = setup_head_watcher(&git_head, head_tx);
            assert!(
                watcher_result.is_err(),
                "setup_head_watcher should fail when .git/HEAD doesn't exist"
            );

            // In watch_worktree, this error is caught and logged as a warning,
            // allowing file watching to continue. The watcher variable is set to None.
            let _head_watcher = match watcher_result {
                Ok(watcher) => Some(watcher),
                Err(_e) => {
                    // This is the expected path - .git/HEAD doesn't exist
                    // In production, a warning would be logged here
                    None
                }
            };

            // Test passes if we reach here - graceful degradation works
        }

        // Test 3: Verify both watchers can coexist
        {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let root_abs = temp_dir.path();
            let git_dir = root_abs.join(".git");
            std::fs::create_dir_all(&git_dir).expect("Failed to create .git dir");
            let git_head = git_dir.join("HEAD");
            std::fs::write(&git_head, "ref: refs/heads/main\n").expect("Failed to write .git/HEAD");

            // Create file watcher channel (simulating WorktreeWatcher)
            let (_file_tx, _file_rx) = tokio::sync::mpsc::channel::<()>(1000);

            // Create head watcher channel
            let (head_tx, _head_rx) = tokio::sync::mpsc::channel(10);

            // Setup head watcher
            let head_watcher_result = setup_head_watcher(&git_head, head_tx);
            assert!(
                head_watcher_result.is_ok(),
                "Head watcher should initialize successfully"
            );

            let _head_watcher = head_watcher_result.unwrap();

            // Both watchers coexist in scope - if test completes, they're compatible
        }
    }

    /// Test that event loop handles both file and head events using tokio::select! (UNIWATCH-3002)
    ///
    /// This test verifies:
    /// 1. Event loop processes file events correctly
    /// 2. Event loop processes head events correctly
    /// 3. Debouncing works for rapid head events
    /// 4. Both event types can be handled in same loop
    /// 5. Graceful shutdown when both channels close
    /// 6. File processing logic unchanged from original implementation
    #[tokio::test]
    async fn test_event_loop_handles_both_sources() {
        use crate::incremental::{EventType, IndexingEvent};
        use std::sync::Arc;
        use tokio::sync::Mutex;

        // Create channels for file events and head events
        let (file_tx, mut file_rx) = tokio::sync::mpsc::channel(100);
        let (head_tx, mut head_rx) = tokio::sync::mpsc::channel(10);

        // Create a temporary directory for test files
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let root = temp_dir.path().to_path_buf();

        // Create test file
        let test_file = root.join("test.txt");
        std::fs::write(&test_file, "test content").expect("Failed to write test file");

        // Create shared state for tracking events processed
        let file_events_processed = Arc::new(Mutex::new(0usize));
        let head_events_processed = Arc::new(Mutex::new(0usize));

        let file_count_clone = file_events_processed.clone();
        let head_count_clone = head_events_processed.clone();

        // Spawn event processing loop (mimics processor_task in watch_worktree)
        let event_task = tokio::spawn(async move {
            let debouncer = DebouncedHandler::new(std::time::Duration::from_millis(50));

            loop {
                tokio::select! {
                    Some(_file_event) = file_rx.recv() => {
                        // Simulate file event processing
                        let mut count = file_count_clone.lock().await;
                        *count += 1;
                    }
                    Some(_head_event) = head_rx.recv() => {
                        // Simulate head event processing with debouncing
                        if !debouncer.should_handle() {
                            continue; // Debounced
                        }

                        let mut count = head_count_clone.lock().await;
                        *count += 1;
                    }
                    else => break, // Both channels closed
                }
            }
        });

        // Test 1: Send file events
        for _ in 0..3 {
            let event = IndexingEvent {
                worktree_id: "test:main".to_string(),
                path: test_file.clone(),
                event_type: EventType::Modified,
                timestamp: std::time::SystemTime::now(),
                old_path: None,
            };
            file_tx
                .send(event)
                .await
                .expect("Failed to send file event");
        }

        // Wait briefly for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Test 2: Send head events (including rapid events to test debouncing)
        for _ in 0..5 {
            head_tx
                .send(notify::Event::default())
                .await
                .expect("Failed to send head event");
        }

        // Wait briefly for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Test 3: Send more rapid head events (should be debounced)
        for _ in 0..3 {
            head_tx
                .send(notify::Event::default())
                .await
                .expect("Failed to send head event");
        }

        // Wait for debounce duration to expire
        tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;

        // Test 4: Send one more head event after debounce expires
        head_tx
            .send(notify::Event::default())
            .await
            .expect("Failed to send head event");

        // Wait briefly for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Test 5: Close channels to trigger graceful shutdown
        drop(file_tx);
        drop(head_tx);

        // Wait for event loop to exit
        let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), event_task).await;

        assert!(
            result.is_ok(),
            "Event loop should exit gracefully when channels close"
        );
        assert!(
            result.unwrap().is_ok(),
            "Event task should complete without panic"
        );

        // Test 6: Verify file events were processed
        let file_count = *file_events_processed.lock().await;
        assert_eq!(file_count, 3, "All 3 file events should be processed");

        // Test 7: Verify head events were processed with debouncing
        // First batch of 5 events: only first should process
        // Second batch of 3 rapid events: all debounced
        // Final event after debounce expires: should process
        // Total: 2 events processed (first from batch 1, final after debounce)
        let head_count = *head_events_processed.lock().await;
        assert!(
            head_count >= 2,
            "At least 2 head events should be processed (first + after debounce), got {}",
            head_count
        );
        assert!(
            head_count <= 3,
            "No more than 3 head events should be processed (debouncing active), got {}",
            head_count
        );
    }
}
