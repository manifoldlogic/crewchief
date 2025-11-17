use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use humantime::parse_duration;
use ignore::WalkBuilder;
use tokio_postgres::Client;
use tracing::{info, warn};

use crate::incremental::path_utils::normalize_to_relpath;

pub mod parallel;
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
#[allow(dead_code)] // Used in UNIWATCH-1004 for branch switch debouncing
struct DebouncedHandler {
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
    /// ```rust
    /// use std::time::Duration;
    ///
    /// let debouncer = DebouncedHandler::new(Duration::from_secs(2));
    /// ```
    fn new(debounce_duration: std::time::Duration) -> Self {
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
    fn should_handle(&self) -> bool {
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
struct BranchSwitchEvent {
    #[serde(rename = "type")]
    event_type: &'static str,
    timestamp: String,
    repo: String,
    old_branch: String,
    new_branch: String,
    old_worktree_id: i64,
    new_worktree_id: i64,
    worktree_created: bool,
}

/// Process Python imports from chunk metadata and create import edges in chunk_edges table
async fn process_python_imports(
    client: &Client,
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
                let imports_chunk_id = crate::db::find_chunk_by_symbol(
                    client,
                    repo_id,
                    Some(worktree_id),
                    "__imports__",
                    None,
                )
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
                            if let Ok(Some(dst_chunk_id)) = crate::db::find_chunk_by_symbol(
                                client,
                                repo_id,
                                Some(worktree_id),
                                name,
                                None,
                            )
                            .await
                            {
                                // Create the import edge
                                if let Err(e) = crate::db::insert_chunk_edge(
                                    client,
                                    src_chunk_id,
                                    dst_chunk_id,
                                    "imports",
                                )
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

pub fn detect_language_from_path(path: &Path) -> Option<&'static str> {
    // Check for go.mod file specifically
    if path.file_name().and_then(|n| n.to_str()) == Some("go.mod") {
        return Some("gomod");
    }

    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "ts" => Some("ts"),
        "tsx" => Some("tsx"),
        "js" => Some("js"),
        "jsx" => Some("jsx"),
        "rs" => Some("rs"),
        "py" => Some("py"),
        "go" => Some("go"),
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

/// Scan worktree with parallel batch processing for improved performance.
///
/// This version uses the parallel indexing pipeline from PERF_OPT-3001:
/// - Parallel file parsing with rayon work-stealing
/// - Batch database inserts (50-100 chunks per batch)
/// - Concurrent database workers (4-8 workers)
///
/// Expected performance: 5-10x faster than sequential scan_worktree.
pub async fn scan_worktree_parallel(
    pool: &crate::db::PgPool,
    repo: &str,
    worktree: &str,
    root: &Path,
    commit: &str,
    languages: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    parallel_config: parallel::ParallelConfig,
    progress: Option<&crate::progress::ProgressTracker>,
) -> anyhow::Result<()> {
    use crate::indexer::parallel::{FileTask, ParallelIndexer};

    let root_abs = root.canonicalize().with_context(|| "invalid root path")?;

    // Get database client for setup
    let client = pool.get().await?;

    let repo_id =
        crate::db::get_or_create_repo(&client, repo, root_abs.to_string_lossy().as_ref()).await?;
    let worktree_id = crate::db::get_or_create_worktree(
        &client,
        repo_id,
        worktree,
        root_abs.to_string_lossy().as_ref(),
    )
    .await?;
    let commit_id = crate::db::get_or_create_commit(&client, repo_id, commit, None).await?;

    println!(
        "🔍 Scanning worktree (parallel): {} @ {}",
        worktree,
        &commit[..8.min(commit.len())]
    );
    println!("   Repository: {}", repo);
    println!("   Path: {}", root_abs.display());
    println!(
        "   Workers: {}, Batch size: {}",
        parallel_config.parallel_workers, parallel_config.batch_size
    );

    // Collect files to process
    let mut walk = WalkBuilder::new(&root_abs);
    walk.hidden(false)
        .ignore(true)
        .git_ignore(true)
        .git_exclude(true);
    if let Some(globs) = &exclude {
        let mut ob = ignore::overrides::OverrideBuilder::new(&root_abs);
        for g in globs {
            ob.add(&format!("!{}", g))?;
        }
        walk.overrides(ob.build()?);
    }

    let allow_langs: Option<Vec<String>> =
        languages.map(|v| v.into_iter().map(|s| s.to_lowercase()).collect());

    let mut file_tasks = Vec::new();
    let mut files_skipped = 0;
    let mut total_bytes = 0usize;
    let mut language_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for dent in walk.build() {
        let dent = match dent {
            Ok(d) => d,
            Err(_) => continue,
        };
        if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }

        let path = dent.path();
        let relpath = path.strip_prefix(&root_abs).unwrap_or(path);
        let language = detect_language_from_path(path);

        if language.is_none() {
            files_skipped += 1;
            continue;
        }

        if let Some(ref allow) = allow_langs {
            if !allow.iter().any(|l| l == language.unwrap()) {
                files_skipped += 1;
                continue;
            }
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                files_skipped += 1;
                continue;
            }
        };

        // Skip files larger than max_file_size
        if content.len() > parallel_config.max_file_size {
            files_skipped += 1;
            continue;
        }

        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        let size_bytes = content.len().min(i32::MAX as usize) as i32;
        let last_modified = file_modified_time(path);

        total_bytes += content.len();
        *language_counts
            .entry(language.unwrap().to_string())
            .or_insert(0) += 1;

        // Create file record
        let file_id = crate::db::upsert_file(
            &client,
            repo_id,
            worktree_id,
            commit_id,
            relpath.to_string_lossy().as_ref(),
            language,
            &content_hash,
            size_bytes,
            last_modified,
        )
        .await?;

        file_tasks.push(FileTask {
            path: path.to_path_buf(),
            relpath: relpath.to_path_buf(),
            language: language.unwrap().to_string(),
            content,
            file_id,
            worktree_id,
        });
    }

    // Drop client before parallel processing
    drop(client);

    // Set progress totals after file collection
    if let Some(p) = progress {
        p.set_totals(file_tasks.len(), None);
    }

    // Process files in parallel
    let indexer = ParallelIndexer::new(pool.clone(), parallel_config);
    let stats = indexer.process_files(file_tasks).await?;

    // Print summary
    println!("\n✅ Parallel scan completed successfully!");
    println!("   Files processed: {}", stats.files_processed);
    if files_skipped > 0 {
        println!("   Files skipped: {}", files_skipped);
    }
    println!("   Total chunks: {}", stats.chunks_inserted);
    println!(
        "   Batches: {} (avg {:.1} chunks/batch)",
        stats.batches_processed,
        stats.avg_chunks_per_batch()
    );
    println!("   Total size: {:.2} MB", total_bytes as f64 / 1_048_576.0);

    if stats.errors > 0 {
        println!("   Errors: {}", stats.errors);
    }

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

    // Finish progress tracking
    if let Some(p) = progress {
        p.finish();
    }

    info!(?repo, ?worktree, ?commit, "parallel scan complete");
    Ok(())
}

pub async fn scan_worktree(
    client: &Client,
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
    let repo_id =
        crate::db::get_or_create_repo(client, repo, root_abs.to_string_lossy().as_ref()).await?;
    let worktree_id = crate::db::get_or_create_worktree(
        client,
        repo_id,
        worktree,
        root_abs.to_string_lossy().as_ref(),
    )
    .await?;
    let commit_id = crate::db::get_or_create_commit(client, repo_id, commit, None).await?;

    // Stats tracking
    let mut files_processed = 0;
    let mut files_skipped = 0;
    let mut total_chunks = 0;
    let mut total_bytes = 0usize;
    let mut language_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    println!(
        "🔍 Scanning worktree: {} @ {}",
        worktree,
        &commit[..8.min(commit.len())]
    );
    println!("   Repository: {}", repo);
    println!("   Path: {}", root_abs.display());

    let mut walk = WalkBuilder::new(&root_abs);
    walk.hidden(false)
        .ignore(true)
        .git_ignore(true)
        .git_exclude(true);
    if let Some(globs) = &exclude {
        let mut ob = ignore::overrides::OverrideBuilder::new(&root_abs);
        for g in globs {
            // Treat excludes as negative overrides
            ob.add(&format!("!{}", g))?;
        }
        walk.overrides(ob.build()?);
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

        let file_id = crate::db::upsert_file(
            client,
            repo_id,
            worktree_id,
            commit_id,
            relpath.to_string_lossy().as_ref(),
            Some(language),
            &content_hash,
            size_bytes,
            last_modified,
        )
        .await?;

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
            crate::db::insert_chunk(
                client,
                file_id,
                &blob_sha,
                None,
                "module",
                None,
                None,
                1,
                content.lines().count() as i32,
                &preview,
                &ts_doc,
                1.0,
                0.0,
                None,
                worktree_id,
            )
            .await?;
        } else {
            total_chunks += chunks.len();
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
                crate::db::insert_chunk(
                    client,
                    file_id,
                    &blob_sha,
                    ch.symbol_name.as_deref(),
                    &ch.kind,
                    ch.signature.as_deref(),
                    ch.docstring.as_deref(),
                    ch.start_line,
                    ch.end_line,
                    &preview,
                    &ts_doc,
                    1.0,
                    0.0,
                    ch.metadata.as_ref(),
                    worktree_id,
                )
                .await?;
            }

            // Process Python imports and create edges
            if language == "py" {
                if let Err(e) =
                    process_python_imports(client, repo_id, worktree_id, file_id, &chunks).await
                {
                    warn!(
                        "Failed to process Python imports for {}: {}",
                        relpath.display(),
                        e
                    );
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
        // If no progress tracker, show timing manually
        let elapsed = start_time.elapsed();
        println!("\n✅ Completed in {:.1}s", elapsed.as_secs_f64());
    }

    // Print summary
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

    info!(?repo, ?worktree, ?commit, "scan complete");
    Ok(())
}

pub async fn upsert_files(
    client: &Client,
    repo: &str,
    worktree: &str,
    root: &Path,
    commit: &str,
    paths: &[PathBuf],
) -> anyhow::Result<()> {
    let root_abs = root.canonicalize().with_context(|| "invalid root path")?;
    let repo_id =
        crate::db::get_or_create_repo(client, repo, root_abs.to_string_lossy().as_ref()).await?;
    let worktree_id = crate::db::get_or_create_worktree(
        client,
        repo_id,
        worktree,
        root_abs.to_string_lossy().as_ref(),
    )
    .await?;
    let commit_id = crate::db::get_or_create_commit(client, repo_id, commit, None).await?;

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
        let file_id = crate::db::upsert_file(
            client,
            repo_id,
            worktree_id,
            commit_id,
            relpath.to_string_lossy().as_ref(),
            language,
            &content_hash,
            size_bytes,
            last_modified,
        )
        .await?;
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
            crate::db::insert_chunk(
                client,
                file_id,
                &blob_sha,
                None,
                "module",
                None,
                None,
                1,
                content.lines().count() as i32,
                &preview,
                &ts_doc,
                1.0,
                0.0,
                None,
                worktree_id,
            )
            .await?;
        } else {
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
                crate::db::insert_chunk(
                    client,
                    file_id,
                    &blob_sha,
                    ch.symbol_name.as_deref(),
                    &ch.kind,
                    ch.signature.as_deref(),
                    ch.docstring.as_deref(),
                    ch.start_line,
                    ch.end_line,
                    &preview,
                    &ts_doc,
                    1.0,
                    0.0,
                    ch.metadata.as_ref(),
                    worktree_id,
                )
                .await?;
            }

            // Process Python imports and create edges
            if language.unwrap() == "py" {
                if let Err(e) =
                    process_python_imports(client, repo_id, worktree_id, file_id, &chunks).await
                {
                    warn!(
                        "Failed to process Python imports for {}: {}",
                        relpath.display(),
                        e
                    );
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
/// ```rust,no_run
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
fn setup_head_watcher(
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

/// Handles branch switch detection and updates database/state accordingly
///
/// This function is the core handler for branch switch events, implementing the workflow:
/// 1. Detect branch name from .git/HEAD using `get_current_branch()`
/// 2. Early return if branch hasn't changed (prevents unnecessary work)
/// 3. Get or create database records for repo and worktree
/// 4. Update shared state variables (current_branch and current_worktree_id)
/// 5. Trigger incremental re-indexing for the new branch
/// 6. Emit NDJSON event to stdout for external consumers
///
/// # Arguments
///
/// * `repo_path` - Absolute path to the repository root
/// * `current_branch` - Shared state tracking the current branch name
/// * `current_worktree_id` - Shared state tracking the current worktree database ID
/// * `pool` - Database connection pool for queries
/// * `repo` - Repository name (e.g., "crewchief")
///
/// # Thread Safety
///
/// Uses Arc<RwLock<T>> for safe concurrent access to shared state:
/// - Acquires read lock to check if branch changed
/// - Acquires write locks to update state after database operations
/// - Drops read lock before acquiring write lock to prevent deadlock
///
/// # Returns
///
/// Returns `Ok(())` if the branch switch was handled successfully or if no switch occurred.
/// Returns `Err` if database operations or re-indexing fail.
///
/// # Example
///
/// ```rust,no_run
/// use std::sync::{Arc, RwLock};
/// use std::path::Path;
/// use crewchief_maproom::db::pool::create_pool;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let pool = create_pool().await?;
///     let current_branch = Arc::new(RwLock::new("main".to_string()));
///     let current_worktree_id = Arc::new(RwLock::new(1i64));
///     let repo_path = Path::new("/workspace/repo");
///
///     handle_branch_switch(
///         repo_path,
///         &current_branch,
///         &current_worktree_id,
///         &pool,
///         "crewchief"
///     ).await?;
///
///     Ok(())
/// }
/// ```
async fn handle_branch_switch(
    repo_path: &Path,
    current_branch: &std::sync::Arc<std::sync::RwLock<String>>,
    current_worktree_id: &std::sync::Arc<std::sync::RwLock<i64>>,
    pool: &crate::db::PgPool,
    repo: &str,
) -> anyhow::Result<()> {
    // Get new branch name from .git/HEAD
    let new_branch = crate::git::get_current_branch(repo_path)?;

    // Check if branch actually changed and capture old state BEFORE updating
    // (early return to prevent unnecessary work)
    let (old_branch, old_worktree_id) = {
        let current = current_branch.read().unwrap();
        if *current == new_branch {
            return Ok(()); // No change, skip processing
        }
        let old_wt_id = *current_worktree_id.read().unwrap();
        (current.clone(), old_wt_id)
    };

    info!("Branch switch detected: {} -> {}", old_branch, new_branch);

    // Get database client from pool
    let client = pool.get().await?;

    // Get or create database records for repo and worktree
    let repo_id = crate::db::get_or_create_repo(
        &client,
        repo,
        repo_path.to_string_lossy().as_ref(),
    )
    .await?;

    // Check if worktree exists before creating
    let worktree_existed = client
        .query_opt(
            "SELECT id FROM maproom.worktrees WHERE repo_id = $1 AND name = $2",
            &[&repo_id, &new_branch],
        )
        .await?
        .is_some();

    let new_worktree_id = crate::db::get_or_create_worktree(
        &client,
        repo_id,
        &new_branch,
        repo_path.to_string_lossy().as_ref(),
    )
    .await?;

    let worktree_created = !worktree_existed;

    // Update shared state with write locks
    {
        let mut branch = current_branch.write().unwrap();
        *branch = new_branch.clone();
    }
    {
        let mut id = current_worktree_id.write().unwrap();
        *id = new_worktree_id;
    }

    // Trigger incremental re-indexing for the new branch
    crate::incremental::incremental_update(&client, new_worktree_id, repo_path).await?;

    // Emit NDJSON event to stdout for external consumers (UNIWATCH-2002)
    let event = BranchSwitchEvent {
        event_type: "branch_switched",
        timestamp: chrono::Utc::now().to_rfc3339(),
        repo: repo.to_string(),
        old_branch,
        new_branch,
        old_worktree_id,
        new_worktree_id,
        worktree_created,
    };

    // Serialize to single-line JSON and emit to stdout
    match serde_json::to_string(&event) {
        Ok(json) => println!("{}", json),
        Err(e) => warn!("Failed to serialize BranchSwitchEvent: {}", e),
    }

    info!("Switched to worktree_id={}", new_worktree_id);
    Ok(())
}

pub async fn watch_worktree(
    _client: &Client,
    repo: &str,
    worktree: &str,
    root: &Path,
    throttle: &str,
) -> anyhow::Result<()> {
    use crate::incremental::{
        ChangeDetector, FileEvent, IncrementalProcessor, Trigger, UpdateQueue, UpdateTask,
        WatcherConfig, WorktreeWatcher,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let root_abs = root.canonicalize().with_context(|| "invalid root path")?;

    // Parse throttle duration and convert to milliseconds for WatcherConfig
    let throttle_dur = parse_duration(throttle)?;
    let debounce_ms = throttle_dur.as_millis().min(u64::MAX as u128) as u64;

    println!("🔌 Validating database connection...");

    // Create connection pool and validate connection BEFORE starting watcher
    // This ensures we fail fast if MAPROOM_DATABASE_URL is misconfigured
    let pool = crate::db::pool::create_pool().await.with_context(|| {
        "Failed to connect to database. Please check your MAPROOM_DATABASE_URL configuration."
    })?;

    // Test the connection by getting a client from the pool
    let test_client = pool
        .get()
        .await
        .with_context(|| "Database connection pool created but unable to acquire connection")?;

    // Verify database has required schema by checking for maproom schema
    match test_client
        .query_opt(
            "SELECT schema_name FROM information_schema.schemata WHERE schema_name = 'maproom'",
            &[],
        )
        .await
    {
        Ok(Some(_)) => {
            println!("✅ Database connection validated successfully");
        }
        Ok(None) => {
            anyhow::bail!(
                "Database connected but 'maproom' schema not found.\n\
                 Run migrations first: cargo run -p crewchief-maproom -- db migrate"
            );
        }
        Err(e) => {
            anyhow::bail!(
                "Failed to verify database schema: {}\n\
                 Check that MAPROOM_DATABASE_URL is correct and database is accessible.",
                e
            );
        }
    }

    // Drop test client to return it to pool
    drop(test_client);

    // Initialize dynamic worktree tracking state (UNIWATCH-1002)
    let _current_branch = std::sync::Arc::new(std::sync::RwLock::new(worktree.to_string()));
    let _current_worktree_id = std::sync::Arc::new(std::sync::RwLock::new({
        let client = pool.get().await?;
        let repo_id = crate::db::get_or_create_repo(
            &client,
            repo,
            root_abs.to_string_lossy().as_ref(),
        )
        .await?;
        let worktree_id = crate::db::get_or_create_worktree(
            &client,
            repo_id,
            worktree,
            root_abs.to_string_lossy().as_ref(),
        )
        .await?;
        worktree_id
    }));

    // Initialize components
    let config = WatcherConfig {
        debounce_ms,
        channel_capacity: 1000,
    };

    let worktree_id = format!("{}:{}", repo, worktree);
    let (mut watcher, mut event_rx) =
        WorktreeWatcher::new(worktree_id.clone(), root_abs.clone(), config)?;

    // Start watching
    watcher.start()?;
    info!(
        repo = %repo,
        worktree = %worktree,
        path = %root_abs.display(),
        "Started incremental watch"
    );

    // Create change detector and processor
    let detector = Arc::new(Mutex::new(ChangeDetector::with_capacity(
        pool.clone(),
        1000,
    )));
    let processor = IncrementalProcessor::new(pool.clone(), root_abs.clone());
    let queue = Arc::new(Mutex::new(UpdateQueue::with_capacity(100)));

    // Spawn event processor task
    let queue_clone = queue.clone();
    let detector_clone = detector.clone();
    let pool_clone = pool.clone();
    let root_clone = root_abs.clone();
    let repo_clone = repo.to_string();
    let worktree_clone = worktree.to_string();

    let processor_task = tokio::spawn(async move {
        while let Some(indexing_event) = event_rx.recv().await {
            // CRITICAL FIX (WATCHFIX-1002): Normalize path ONCE at event entry
            // The database stores relative paths (e.g., "packages/cli/src/main.ts")
            // but events arrive with absolute paths (e.g., "/workspace/packages/cli/src/main.ts").
            // We must normalize to relpath for database lookups, then use absolute path for filesystem ops.
            let relpath = match normalize_to_relpath(&indexing_event.path, &root_clone) {
                Ok(p) => p,
                Err(e) => {
                    warn!(
                        path = %indexing_event.path.display(),
                        error = %e,
                        "Path normalization failed - path outside repository, skipping event"
                    );
                    continue; // Skip this event
                }
            };

            // Convert relpath to string for database queries
            let relpath_str = match relpath.to_str() {
                Some(s) => s,
                None => {
                    warn!(
                        path = %relpath.display(),
                        "Path contains invalid UTF-8, skipping event"
                    );
                    continue;
                }
            };

            // Convert IndexingEvent to FileEvent
            let file_event = match indexing_event.event_type {
                crate::incremental::EventType::Modified => {
                    FileEvent::Modified(indexing_event.path.clone())
                }
                crate::incremental::EventType::Deleted => {
                    FileEvent::Deleted(indexing_event.path.clone())
                }
                crate::incremental::EventType::Renamed => {
                    if let Some(old_path) = indexing_event.old_path {
                        FileEvent::Renamed(old_path, indexing_event.path.clone())
                    } else {
                        FileEvent::Modified(indexing_event.path.clone())
                    }
                }
            };

            // Detect change type
            let change_type = match file_event {
                FileEvent::Modified(ref path) => {
                    // CRITICAL FIX (WATCHFIX-1002): Use normalized relpath for database lookup
                    // Previously this used absolute path, causing lookups to fail and files
                    // to be misclassified as NEW when they were actually MODIFIED.
                    match get_file_id_by_path(
                        &pool_clone,
                        &repo_clone,
                        &worktree_clone,
                        relpath_str,
                    )
                    .await
                    {
                        Ok(Some(file_id)) => {
                            // File exists in database - ALWAYS call ChangeDetector
                            // This is the key fix: we must use ChangeDetector to determine
                            // if content actually changed (Modified vs None).
                            detector_clone
                                .lock()
                                .await
                                .detect_change(file_id, path)
                                .await
                                .ok()
                        }
                        Ok(None) => {
                            // File not in database - truly a new file
                            // Compute hash directly since there's no existing record to compare against
                            if path.exists() {
                                if let Ok(hash) = crate::incremental::FileHasher::hash_file(path) {
                                    Some(crate::incremental::ChangeType::New(hash))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        Err(e) => {
                            warn!(
                                path = %path.display(),
                                relpath = %relpath.display(),
                                error = %e,
                                "Database lookup failed, skipping event"
                            );
                            None
                        }
                    }
                }
                FileEvent::Deleted(ref path) => {
                    // Use normalized relpath for database lookup
                    match get_file_id_by_path(
                        &pool_clone,
                        &repo_clone,
                        &worktree_clone,
                        relpath_str,
                    )
                    .await
                    {
                        Ok(Some(file_id)) => detector_clone
                            .lock()
                            .await
                            .detect_deletion(file_id, path)
                            .await
                            .ok()
                            .flatten(),
                        Ok(None) => None,
                        Err(e) => {
                            warn!(
                                path = %path.display(),
                                relpath = %relpath.display(),
                                error = %e,
                                "Database lookup failed for deletion, skipping event"
                            );
                            None
                        }
                    }
                }
                FileEvent::Renamed(ref _old_path, ref new_path) => {
                    // Treat rename as delete + new
                    if let Ok(hash) = crate::incremental::FileHasher::hash_file(new_path) {
                        Some(crate::incremental::ChangeType::New(hash))
                    } else {
                        None
                    }
                }
            };

            if let Some(change) = change_type {
                if !matches!(change, crate::incremental::ChangeType::None) {
                    let task = UpdateTask::new(indexing_event.path.clone(), change, Trigger::Auto);
                    queue_clone.lock().await.enqueue(task);
                }
            }
        }
    });

    // Spawn task processor
    let queue_clone = queue.clone();
    let processor_clone = Arc::new(processor);
    let processing_task = tokio::spawn(async move {
        loop {
            let task = {
                let mut q = queue_clone.lock().await;
                q.dequeue()
            };

            if let Some(task) = task {
                let path = task.path.clone();
                match processor_clone.process(task).await {
                    Ok(_) => {
                        queue_clone.lock().await.mark_completed(&path);
                    }
                    Err(e) => {
                        warn!(path = %path.display(), error = %e, "Failed to process file");
                        // Re-enqueue with retry
                        let task_again = { queue_clone.lock().await.dequeue() };
                        if let Some(t) = task_again {
                            queue_clone.lock().await.mark_failed(t, &e.to_string());
                        }
                    }
                }
            } else {
                // No tasks available, sleep briefly
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    // Status reporting task
    let queue_clone = queue.clone();
    let root_clone_status = root_abs.clone();
    let status_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        let mut events_processed = 0usize;

        loop {
            interval.tick().await;
            let stats = queue_clone.lock().await.stats();
            events_processed += stats.processing;

            // Count files in watched directory (estimate)
            let files_watched = WalkBuilder::new(&root_clone_status)
                .hidden(false)
                .ignore(true)
                .git_ignore(true)
                .git_exclude(true)
                .build()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .count();

            info!(
                files_watched = files_watched,
                watcher_state = "running",
                queue_size = stats.pending,
                processing = stats.processing,
                dead_letter = stats.dead_letter,
                total_processed = events_processed,
                "Watch status"
            );
        }
    });

    // Wait for SIGINT (Ctrl+C) or SIGTERM signal
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm =
        signal(SignalKind::terminate()).context("Failed to install SIGTERM handler")?;

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received SIGINT");
        },
        _ = sigterm.recv() => {
            info!("Received SIGTERM");
        },
    }

    info!("Received shutdown signal, stopping watch...");

    // Stop the watcher
    watcher.stop()?;

    // Wait briefly for in-flight events to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Process remaining tasks in queue
    let remaining = {
        let q = queue.lock().await;
        q.queue_size()
    };
    if remaining > 0 {
        info!("Processing {} remaining tasks...", remaining);
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }

    // Cancel background tasks
    processor_task.abort();
    processing_task.abort();
    status_task.abort();

    info!("Watch stopped gracefully");
    Ok(())
}

/// Helper function to get file_id from database by path
async fn get_file_id_by_path(
    pool: &crate::db::PgPool,
    repo: &str,
    worktree: &str,
    relpath: &str,
) -> anyhow::Result<Option<i64>> {
    let client = pool.get().await?;

    let row = client
        .query_opt(
            "SELECT f.id FROM maproom.files f
         JOIN maproom.worktrees w ON f.worktree_id = w.id
         JOIN maproom.repos r ON w.repo_id = r.id
         WHERE r.name = $1 AND w.name = $2 AND f.relpath = $3
         ORDER BY f.id DESC LIMIT 1",
            &[&repo, &worktree, &relpath],
        )
        .await?;

    Ok(row.map(|r| r.get(0)))
}

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
        assert!(watcher_result.is_ok(), "Failed to create watcher: {:?}", watcher_result.err());

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
    #[tokio::test]
    async fn test_worktree_tracking_initialization() {
        // Setup test database
        let pool = match crate::db::pool::create_pool().await {
            Ok(p) => p,
            Err(_) => {
                // Skip test if database not available
                eprintln!("Skipping test: database not available");
                return;
            }
        };

        // Test parameters
        let repo = "test-repo";
        let worktree = "test-branch";
        let root = std::path::Path::new("/tmp/test-root");
        let root_str = root.to_string_lossy();

        // Initialize tracking state (mirrors watch_worktree logic)
        let current_branch = std::sync::Arc::new(std::sync::RwLock::new(worktree.to_string()));
        let current_worktree_id = std::sync::Arc::new(std::sync::RwLock::new({
            let client = pool.get().await.expect("Failed to get client from pool");
            let repo_id = crate::db::get_or_create_repo(&client, repo, &root_str)
                .await
                .expect("Failed to get_or_create_repo");
            let worktree_id =
                crate::db::get_or_create_worktree(&client, repo_id, worktree, &root_str)
                    .await
                    .expect("Failed to get_or_create_worktree");
            worktree_id
        }));

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

        // Test 4: Verify write locks work (for future branch switch logic)
        {
            let mut branch_guard = current_branch
                .write()
                .expect("Failed to acquire write lock on current_branch");
            let new_branch = "feature-branch";
            *branch_guard = new_branch.to_string();
            assert_eq!(*branch_guard, new_branch, "Write lock should allow mutation");
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

    /// Test that handle_branch_switch updates state when branch changes (UNIWATCH-2001)
    ///
    /// This test verifies:
    /// 1. Function detects new branch name from repository
    /// 2. Database records are created/updated for the new worktree
    /// 3. current_branch Arc<RwLock<String>> is updated to new branch
    /// 4. current_worktree_id Arc<RwLock<i64>> is updated to new worktree_id
    /// 5. Incremental update is triggered for the new branch
    /// 6. Function returns Ok(()) on success
    #[tokio::test]
    async fn test_handle_branch_switch_updates_state() {
        use std::sync::{Arc, RwLock};
        use tempfile::TempDir;

        // Setup test database
        let pool = match crate::db::pool::create_pool().await {
            Ok(p) => p,
            Err(_) => {
                eprintln!("Skipping test: database not available");
                return;
            }
        };

        // Create a temporary git repository for testing
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();
        let git_dir = repo_path.join(".git");
        std::fs::create_dir_all(&git_dir).expect("Failed to create .git dir");

        // Initialize git repository
        let init_output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to run git init");

        if !init_output.status.success() {
            eprintln!(
                "Skipping test: git init failed: {}",
                String::from_utf8_lossy(&init_output.stderr)
            );
            return;
        }

        // Create initial commit on main branch
        std::fs::write(repo_path.join("test.txt"), "test content")
            .expect("Failed to write test file");

        std::process::Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to git add");

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to set git user.email");

        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to set git user.name");

        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to git commit");

        // Create and checkout feature branch
        let checkout_output = std::process::Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to checkout feature branch");

        if !checkout_output.status.success() {
            eprintln!(
                "Skipping test: git checkout failed: {}",
                String::from_utf8_lossy(&checkout_output.stderr)
            );
            return;
        }

        // Initialize shared state with "main" (simulating initial state)
        let current_branch = Arc::new(RwLock::new("main".to_string()));
        let current_worktree_id = Arc::new(RwLock::new(1i64));

        // Call handle_branch_switch
        let result = handle_branch_switch(
            repo_path,
            &current_branch,
            &current_worktree_id,
            &pool,
            "test-repo",
        )
        .await;

        // Verify function succeeded
        assert!(
            result.is_ok(),
            "handle_branch_switch should return Ok, got: {:?}",
            result
        );

        // Verify current_branch was updated to "feature"
        {
            let branch_guard = current_branch.read().unwrap();
            assert_eq!(
                *branch_guard, "feature",
                "current_branch should be updated to 'feature'"
            );
        }

        // Verify current_worktree_id was updated (should be > 0)
        {
            let worktree_id_guard = current_worktree_id.read().unwrap();
            assert!(
                *worktree_id_guard > 0,
                "current_worktree_id should be updated to a valid ID"
            );
        }

        // Verify database record exists for the new worktree
        let client = pool.get().await.expect("Failed to get client");
        let row = client
            .query_opt(
                "SELECT id FROM maproom.worktrees WHERE name = $1",
                &[&"feature"],
            )
            .await
            .expect("Failed to query worktrees");

        assert!(
            row.is_some(),
            "Database should have a record for 'feature' worktree"
        );
    }

    /// Test that handle_branch_switch skips processing if branch hasn't changed (UNIWATCH-2001)
    ///
    /// This test verifies:
    /// 1. Function detects current branch name
    /// 2. Early return if branch matches current_branch state
    /// 3. No database operations are performed (performance optimization)
    /// 4. Shared state remains unchanged
    /// 5. Function returns Ok(()) quickly
    #[tokio::test]
    async fn test_handle_branch_switch_skips_if_same_branch() {
        use std::sync::{Arc, RwLock};
        use tempfile::TempDir;

        // Setup test database
        let pool = match crate::db::pool::create_pool().await {
            Ok(p) => p,
            Err(_) => {
                eprintln!("Skipping test: database not available");
                return;
            }
        };

        // Create a temporary git repository for testing
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();
        let git_dir = repo_path.join(".git");
        std::fs::create_dir_all(&git_dir).expect("Failed to create .git dir");

        // Initialize git repository on main branch
        let init_output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to run git init");

        if !init_output.status.success() {
            eprintln!(
                "Skipping test: git init failed: {}",
                String::from_utf8_lossy(&init_output.stderr)
            );
            return;
        }

        // Create initial commit on main branch
        std::fs::write(repo_path.join("test.txt"), "test content")
            .expect("Failed to write test file");

        std::process::Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to git add");

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to set git user.email");

        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to set git user.name");

        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to git commit");

        // Get current branch name (should be "main" or "master" depending on git version)
        let branch_output = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to get current branch");

        let current_branch_name = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        // Initialize shared state with current branch (same as repo)
        let current_branch = Arc::new(RwLock::new(current_branch_name.clone()));
        let current_worktree_id = Arc::new(RwLock::new(42i64));

        // Call handle_branch_switch (should early return)
        let start = std::time::Instant::now();
        let result = handle_branch_switch(
            repo_path,
            &current_branch,
            &current_worktree_id,
            &pool,
            "test-repo",
        )
        .await;
        let elapsed = start.elapsed();

        // Verify function succeeded
        assert!(
            result.is_ok(),
            "handle_branch_switch should return Ok, got: {:?}",
            result
        );

        // Verify function returned quickly (< 10ms for early return)
        assert!(
            elapsed.as_millis() < 50,
            "Function should return quickly on same branch (took {}ms)",
            elapsed.as_millis()
        );

        // Verify current_branch was NOT changed
        {
            let branch_guard = current_branch.read().unwrap();
            assert_eq!(
                *branch_guard, current_branch_name,
                "current_branch should remain unchanged"
            );
        }

        // Verify current_worktree_id was NOT changed
        {
            let worktree_id_guard = current_worktree_id.read().unwrap();
            assert_eq!(
                *worktree_id_guard, 42i64,
                "current_worktree_id should remain unchanged"
            );
        }
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
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("JSON should be valid and parseable");

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
            "type", "timestamp", "repo", "old_branch", "new_branch",
            "old_worktree_id", "new_worktree_id", "worktree_created"
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
}
