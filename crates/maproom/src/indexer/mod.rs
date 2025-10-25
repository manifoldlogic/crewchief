use std::{fs, path::{Path, PathBuf}};

use anyhow::Context;
use humantime::parse_duration;
use ignore::WalkBuilder;
use tokio_postgres::Client;
use tracing::{info, warn};

pub mod parser;

/// Process Python imports from chunk metadata and create import edges in chunk_edges table
async fn process_python_imports(
    client: &Client,
    repo_id: i64,
    worktree_id: i64,
    _file_id: i64,
    chunks: &[SymbolChunk],
) -> anyhow::Result<()> {
    // Find the imports chunk if it exists
    let imports_chunk = chunks.iter()
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
                        let names = import_obj.get("names")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();

                        // For each imported name, try to find the target chunk
                        for name in names {
                            if let Ok(Some(dst_chunk_id)) = crate::db::find_chunk_by_symbol(
                                client,
                                repo_id,
                                Some(worktree_id),
                                name,
                                None,
                            ).await {
                                // Create the import edge
                                if let Err(e) = crate::db::insert_chunk_edge(
                                    client,
                                    src_chunk_id,
                                    dst_chunk_id,
                                    "imports",
                                ).await {
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

fn detect_language_from_path(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "ts" => Some("ts"),
        "tsx" => Some("tsx"),
        "js" => Some("js"),
        "jsx" => Some("jsx"),
        "rs" => Some("rs"),
        "py" => Some("py"),
        "md" => Some("md"),
        "mdx" => Some("mdx"),
        "json" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
        _ => None,
    }
}

fn build_ts_doc(relpath: &str, symbol_name: Option<&str>, signature: Option<&str>, docstring: Option<&str>, preview: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(relpath.to_owned());
    if let Some(s) = symbol_name { parts.push(s.to_owned()); }
    if let Some(s) = signature { parts.push(s.to_owned()); }
    if let Some(s) = docstring { parts.push(s.to_owned()); }
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

pub async fn scan_worktree(
    client: &Client,
    repo: &str,
    worktree: &str,
    root: &Path,
    commit: &str,
    _concurrency: usize,
    languages: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
) -> anyhow::Result<()> {
    let root_abs = root.canonicalize().with_context(|| "invalid root path")?;
    let repo_id = crate::db::get_or_create_repo(client, repo, root_abs.to_string_lossy().as_ref()).await?;
    let worktree_id = crate::db::get_or_create_worktree(client, repo_id, worktree, root_abs.to_string_lossy().as_ref()).await?;
    let commit_id = crate::db::get_or_create_commit(client, repo_id, commit, None).await?;

    // Stats tracking
    let mut files_processed = 0;
    let mut files_skipped = 0;
    let mut total_chunks = 0;
    let mut total_bytes = 0usize;
    let mut language_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    println!("🔍 Scanning worktree: {} @ {}", worktree, &commit[..8.min(commit.len())]);
    println!("   Repository: {}", repo);
    println!("   Path: {}", root_abs.display());

    let mut walk = WalkBuilder::new(&root_abs);
    walk.hidden(false).ignore(true).git_ignore(true).git_exclude(true);
    if let Some(globs) = &exclude {
        let mut ob = ignore::overrides::OverrideBuilder::new(&root_abs);
        for g in globs {
            // Treat excludes as negative overrides
            ob.add(&format!("!{}", g))?;
        }
        walk.overrides(ob.build()?);
    }

    let allow_langs: Option<Vec<String>> = languages.map(|v| v.into_iter().map(|s| s.to_lowercase()).collect());

    for dent in walk.build() {
        let dent = match dent { Ok(d) => d, Err(_) => continue };
        if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) { continue; }
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
        *language_counts.entry(language.unwrap().to_string()).or_insert(0) += 1;

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
            // Fallback: single module chunk
            total_chunks += 1;
            let preview = first_n_lines(&content, 40);
            let ts_doc = build_ts_doc(relpath.to_string_lossy().as_ref(), None, None, None, &preview);
            crate::db::insert_chunk(
                client,
                file_id,
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
            ).await?;
        } else {
            total_chunks += chunks.len();
            for ch in &chunks {
                let preview = first_n_lines(&content.split('\n').skip(ch.start_line as usize - 1).take((ch.end_line - ch.start_line + 1) as usize).collect::<Vec<&str>>().join("\n"), 40);
                let ts_doc = build_ts_doc(relpath.to_string_lossy().as_ref(), ch.symbol_name.as_deref(), ch.signature.as_deref(), ch.docstring.as_deref(), &preview);
                crate::db::insert_chunk(
                    client,
                    file_id,
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
                ).await?;
            }

            // Process Python imports and create edges
            if language.unwrap() == "py" {
                if let Err(e) = process_python_imports(client, repo_id, worktree_id, file_id, &chunks).await {
                    warn!("Failed to process Python imports for {}: {}", relpath.display(), e);
                }
            }
        }
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
            println!("     {} {}: {}",
                match lang.as_str() {
                    "ts" | "tsx" => "📘",
                    "js" | "jsx" => "📙",
                    "rs" => "🦀",
                    "py" => "🐍",
                    "md" => "📝",
                    "json" => "📋",
                    "yaml" | "yml" => "📄",
                    "toml" => "⚙️",
                    _ => "📄"
                },
                lang, count);
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
    let repo_id = crate::db::get_or_create_repo(client, repo, root_abs.to_string_lossy().as_ref()).await?;
    let worktree_id = crate::db::get_or_create_worktree(client, repo_id, worktree, root_abs.to_string_lossy().as_ref()).await?;
    let commit_id = crate::db::get_or_create_commit(client, repo_id, commit, None).await?;

    for path in paths {
        let abs = if path.is_absolute() { path.clone() } else { root_abs.join(path) };
        if !abs.exists() { continue; }
        if abs.is_dir() { continue; }
        let relpath = abs.strip_prefix(&root_abs).unwrap_or(&abs).to_path_buf();
        let language = detect_language_from_path(&abs);
        if language.is_none() { continue; }
        let content = match fs::read_to_string(&abs) { Ok(c) => c, Err(_) => continue };
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
        ).await?;
        let chunks = parser::extract_chunks(&content, language.unwrap());
        if chunks.is_empty() {
            let preview = first_n_lines(&content, 40);
            let ts_doc = build_ts_doc(relpath.to_string_lossy().as_ref(), None, None, None, &preview);
            crate::db::insert_chunk(
                client, file_id, None, "module", None, None, 1, content.lines().count() as i32,
                &preview, &ts_doc, 1.0, 0.0, None
            ).await?;
        } else {
            for ch in &chunks {
                let preview = first_n_lines(&content.split('\n').skip(ch.start_line as usize - 1).take((ch.end_line - ch.start_line + 1) as usize).collect::<Vec<&str>>().join("\n"), 40);
                let ts_doc = build_ts_doc(relpath.to_string_lossy().as_ref(), ch.symbol_name.as_deref(), ch.signature.as_deref(), ch.docstring.as_deref(), &preview);
                crate::db::insert_chunk(
                    client, file_id, ch.symbol_name.as_deref(), &ch.kind, ch.signature.as_deref(), ch.docstring.as_deref(),
                    ch.start_line, ch.end_line, &preview, &ts_doc, 1.0, 0.0, ch.metadata.as_ref()
                ).await?;
            }

            // Process Python imports and create edges
            if language.unwrap() == "py" {
                if let Err(e) = process_python_imports(client, repo_id, worktree_id, file_id, &chunks).await {
                    warn!("Failed to process Python imports for {}: {}", relpath.display(), e);
                }
            }
        }
    }

    info!(?repo, ?worktree, ?commit, updated_files=?paths.len(), "upsert selective complete");
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
        ChangeDetector, FileEvent, IncrementalProcessor, UpdateQueue, UpdateTask,
        WatcherConfig, WorktreeWatcher, Trigger,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let root_abs = root.canonicalize().with_context(|| "invalid root path")?;

    // Parse throttle duration and convert to milliseconds for WatcherConfig
    let throttle_dur = parse_duration(throttle)?;
    let debounce_ms = throttle_dur.as_millis().min(u64::MAX as u128) as u64;

    // Create connection pool
    let pool = crate::db::pool::create_pool().await?;

    // Initialize components
    let config = WatcherConfig {
        debounce_ms,
        channel_capacity: 1000,
    };

    let worktree_id = format!("{}:{}", repo, worktree);
    let (mut watcher, mut event_rx) = WorktreeWatcher::new(
        worktree_id.clone(),
        root_abs.clone(),
        config,
    )?;

    // Start watching
    watcher.start()?;
    info!(
        repo = %repo,
        worktree = %worktree,
        path = %root_abs.display(),
        "Started incremental watch"
    );

    // Create change detector and processor
    let detector = Arc::new(Mutex::new(ChangeDetector::with_capacity(pool.clone(), 1000)));
    let processor = IncrementalProcessor::new(pool.clone());
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
            // Convert IndexingEvent to FileEvent
            let file_event = match indexing_event.event_type {
                crate::incremental::EventType::Modified => FileEvent::Modified(indexing_event.path.clone()),
                crate::incremental::EventType::Deleted => FileEvent::Deleted(indexing_event.path.clone()),
                crate::incremental::EventType::Renamed => {
                    if let Some(old_path) = indexing_event.old_path {
                        FileEvent::Renamed(old_path, indexing_event.path.clone())
                    } else {
                        FileEvent::Modified(indexing_event.path.clone())
                    }
                }
            };

            // Get file_id from database (simplified - assumes file exists)
            // In production, we'd handle file creation here
            let relpath = indexing_event.path.strip_prefix(&root_clone).unwrap_or(&indexing_event.path);

            // Detect change type
            let change_type = match file_event {
                FileEvent::Modified(ref path) => {
                    // Try to get file_id from database
                    if let Ok(Some(file_id)) = get_file_id_by_path(&pool_clone, &repo_clone, &worktree_clone, relpath.to_str().unwrap()).await {
                        detector_clone.lock().await.detect_change(file_id, path).await.ok()
                    } else {
                        // New file - compute hash
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
                }
                FileEvent::Deleted(ref path) => {
                    if let Ok(Some(file_id)) = get_file_id_by_path(&pool_clone, &repo_clone, &worktree_clone, relpath.to_str().unwrap()).await {
                        detector_clone.lock().await.detect_deletion(file_id, path).await.ok().flatten()
                    } else {
                        None
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
                    let task = UpdateTask::new(
                        indexing_event.path.clone(),
                        change,
                        Trigger::Auto,
                    );
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
                        let task_again = {
                            queue_clone.lock().await.dequeue()
                        };
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
    let mut sigterm = signal(SignalKind::terminate())
        .context("Failed to install SIGTERM handler")?;

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

    let row = client.query_opt(
        "SELECT f.id FROM maproom.files f
         JOIN maproom.worktrees w ON f.worktree_id = w.id
         JOIN maproom.repos r ON w.repo_id = r.id
         WHERE r.name = $1 AND w.name = $2 AND f.relpath = $3
         ORDER BY f.id DESC LIMIT 1",
        &[&repo, &worktree, &relpath],
    ).await?;

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


